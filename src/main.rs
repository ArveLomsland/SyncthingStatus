// Ingen konsollvindu i release-bygg
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod icon;
mod platform;
mod syncthing;
#[cfg(windows)]
mod win_promote;

use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

use syncthing::{fmt_bytes, State, Status};

/// Kommandoer til bakgrunnstråden
enum Cmd {
    RefreshNow,
    RescanAll,
}

const TICK_MS: u32 = 100;
const ANIM_FRAMES: u32 = 12;

fn main() {
    let cfg = match config::load() {
        Ok(c) => c,
        Err(e) => {
            platform::error_dialog("SyncthingStatus", &e);
            std::process::exit(1);
        }
    };
    // Feilsøking: vis ikonene som ASCII
    if std::env::args().any(|a| a == "--preview") {
        for (st, phase) in [
            (State::Ok, 0.0),
            (State::Syncing, 0.0),
            (State::Paused, 0.0),
            (State::Error, 0.0),
            (State::Offline, 0.0),
        ] {
            println!("--- {:?} ---\n{}", st, icon::ascii_preview(st, phase));
        }
        return;
    }

    // Diagnosemodus: skriv status til konsollet og avslutt
    if std::env::args().any(|a| a == "--status") {
        println!("URL: {}", cfg.base_url);
        println!("API-nøkkel: {} tegn", cfg.api_key.len());
        match syncthing::Client::new(cfg.base_url.clone(), cfg.api_key.clone()) {
            Ok(mut c) => {
                let s = c.fetch();
                println!("Tilstand: {:?}", s.state());
                println!("{}", s.summary());
                for f in &s.folders {
                    println!(
                        "  {:<24} {:<10} {:>6.1}%  {} igjen",
                        f.display_name(),
                        f.state,
                        f.completion,
                        fmt_bytes(f.need_bytes as f64)
                    );
                }
                if let Some(e) = &s.conn_error {
                    println!("Feil: {e}");
                }
            }
            Err(e) => println!("Klientfeil: {e}"),
        }
        return;
    }

    if cfg.api_key.is_empty() {
        platform::error_dialog(
            "SyncthingStatus",
            "Fant ingen API-nøkkel. Sett SYNCTHING_APIKEY, eller aktiver GUI i Syncthing.",
        );
        std::process::exit(1);
    }

    let (status_tx, status_rx) = mpsc::channel::<Status>();
    let (cmd_tx, cmd_rx) = mpsc::channel::<Cmd>();

    {
        let base = cfg.base_url.clone();
        let key = cfg.api_key.clone();
        let interval = cfg.poll_interval;
        std::thread::spawn(move || poller(base, key, interval, status_tx, cmd_rx));
    }

    if let Err(e) = platform::init() {
        platform::error_dialog("SyncthingStatus", &e);
        std::process::exit(1);
    }

    if let Err(e) = run_tray(&cfg.base_url, status_rx, cmd_tx) {
        platform::error_dialog("SyncthingStatus", &e);
        std::process::exit(1);
    }
}

/// Bakgrunnstråd: spør Syncthing med jevne mellomrom
fn poller(
    base_url: String,
    api_key: String,
    interval: Duration,
    tx: Sender<Status>,
    cmd_rx: Receiver<Cmd>,
) {
    let mut client = match syncthing::Client::new(base_url, api_key) {
        Ok(c) => c,
        Err(e) => {
            let _ = tx.send(Status {
                conn_error: Some(e),
                ..Default::default()
            });
            return;
        }
    };

    loop {
        let status = client.fetch();
        if tx.send(status).is_err() {
            return;
        }
        match cmd_rx.recv_timeout(interval) {
            Ok(Cmd::RefreshNow) => {}
            Ok(Cmd::RescanAll) => {
                let _ = client.post("/rest/db/scan");
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
    }
}

struct MenuIds;
impl MenuIds {
    const OPEN: &'static str = "open";
    const RESCAN: &'static str = "rescan";
    const REFRESH: &'static str = "refresh";
    const QUIT: &'static str = "quit";
}

fn run_tray(
    base_url: &str,
    status_rx: Receiver<Status>,
    cmd_tx: Sender<Cmd>,
) -> Result<(), String> {
    let mut status = Status::default();
    let mut menu_lines: Vec<String> = Vec::new();

    let tray: TrayIcon = TrayIconBuilder::new()
        .with_tooltip("Syncthing — kobler til …")
        .with_icon(icon::build(State::Offline, 0.0))
        .with_menu(Box::new(build_menu(&menu_lines)))
        .build()
        .map_err(|e| format!("Kunne ikke lage tray-ikon: {e}"))?;

    let menu_events = MenuEvent::receiver();
    let tray_events = TrayIconEvent::receiver();

    let mut state = State::Offline;
    let mut frame: u32 = 0;
    let mut last_anim = Instant::now();
    let mut icon_dirty = true;

    // Windows 11: gjør ikonet synlig på oppgavelinjen i stedet for i «^»-menyen.
    // Registeroppføringen opprettes av Explorer først etter at ikonet er vist,
    // så vi prøver noen ganger de første sekundene.
    #[cfg(windows)]
    let mut promote = (!std::env::args().any(|a| a == "--no-promote")).then(|| {
        (Instant::now(), 0u32)
    });

    loop {
        // --- ny status fra bakgrunnstråd ---
        let mut got_status = false;
        while let Ok(s) = status_rx.try_recv() {
            status = s;
            got_status = true;
        }
        if got_status {
            let new_state = status.state();
            if new_state != state {
                state = new_state;
                frame = 0;
                icon_dirty = true;
            }
            apply_labels(&tray, &status);
            let lines = status_lines(&status);
            if lines != menu_lines {
                menu_lines = lines;
                let _ = tray.set_menu(Some(Box::new(build_menu(&menu_lines))));
            }
        }

        // --- animasjon ved synkronisering ---
        if matches!(state, State::Syncing | State::Scanning)
            && last_anim.elapsed() >= Duration::from_millis(100)
        {
            last_anim = Instant::now();
            frame = (frame + 1) % ANIM_FRAMES;
            icon_dirty = true;
        }
        if icon_dirty {
            icon_dirty = false;
            let phase = frame as f32 / ANIM_FRAMES as f32;
            let _ = tray.set_icon(Some(icon::build(state, phase)));
        }

        // --- menyvalg ---
        while let Ok(ev) = menu_events.try_recv() {
            match ev.id.0.as_str() {
                MenuIds::OPEN => {
                    let _ = open::that_detached(base_url);
                }
                MenuIds::RESCAN => {
                    let _ = cmd_tx.send(Cmd::RescanAll);
                }
                MenuIds::REFRESH => {
                    let _ = cmd_tx.send(Cmd::RefreshNow);
                }
                MenuIds::QUIT => return Ok(()),
                _ => {}
            }
        }

        // --- klikk på ikonet åpner web-GUI ---
        while let Ok(ev) = tray_events.try_recv() {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = ev
            {
                let _ = open::that_detached(base_url);
            }
        }

        #[cfg(windows)]
        if let Some((last_try, tries)) = promote.as_mut() {
            if last_try.elapsed() >= Duration::from_secs(2) {
                *last_try = Instant::now();
                *tries += 1;
                match win_promote::promote() {
                    Ok(true) => promote = None,
                    _ if *tries >= 10 => promote = None,
                    _ => {}
                }
            }
        }

        if !platform::pump_events(TICK_MS) {
            return Ok(());
        }
    }
}

/// Tooltip (Windows/macOS) og tittel ved siden av ikonet (Linux).
/// `set_tooltip` er en no-op i libappindicator, derfor tittel på Linux.
fn apply_labels(tray: &TrayIcon, status: &Status) {
    let _ = tray.set_tooltip(Some(status.summary()));

    #[cfg(target_os = "linux")]
    {
        let title = if status.online {
            match status.state() {
                State::Syncing => {
                    let need: u64 = status.folders.iter().map(|f| f.need_bytes).sum();
                    format!("Synkroniserer – {} igjen", fmt_bytes(need as f64))
                }
                st => st.label().to_string(),
            }
        } else {
            "Ikke tilkoblet".to_string()
        };
        tray.set_title(Some(title));
    }
}

/// Informasjonslinjer (deaktiverte menyelementer) øverst i menyen
fn status_lines(status: &Status) -> Vec<String> {
    let mut lines = Vec::new();
    if !status.online {
        lines.push("Ikke tilkoblet Syncthing".to_string());
        if let Some(e) = &status.conn_error {
            lines.push(format!("  {e}"));
        }
        return lines;
    }

    lines.push(format!(
        "Status: {}  ·  Syncthing {}",
        status.state().label(),
        status.version
    ));
    lines.push(format!(
        "Enheter: {} / {} tilkoblet",
        status.devices_connected, status.devices_total
    ));
    if status.in_rate > 1024.0 || status.out_rate > 1024.0 {
        lines.push(format!(
            "Trafikk: ned {}/s  ·  opp {}/s",
            fmt_bytes(status.in_rate),
            fmt_bytes(status.out_rate)
        ));
    }
    lines.push(String::new()); // separator

    for f in &status.folders {
        let detail = if f.paused {
            "pauset".to_string()
        } else if let Some(err) = &f.error {
            format!("feil: {err}")
        } else if f.need_items > 0 || f.state.starts_with("sync") {
            format!(
                "{:.0}% · {} igjen",
                f.completion,
                fmt_bytes(f.need_bytes as f64)
            )
        } else {
            match f.state.as_str() {
                "scanning" => "skanner".to_string(),
                "idle" => "oppdatert".to_string(),
                other => other.to_string(),
            }
        };
        lines.push(format!("{}  —  {}", f.display_name(), detail));
    }

    for e in status.errors.iter().take(3) {
        lines.push(format!("⚠ {e}"));
    }
    lines
}

fn build_menu(lines: &[String]) -> Menu {
    let menu = Menu::new();
    for line in lines {
        if line.is_empty() {
            let _ = menu.append(&PredefinedMenuItem::separator());
        } else {
            // deaktivert = ren informasjon
            let _ = menu.append(&MenuItem::new(line, false, None));
        }
    }
    if !lines.is_empty() {
        let _ = menu.append(&PredefinedMenuItem::separator());
    }
    let _ = menu.append(&MenuItem::with_id(
        MenuIds::OPEN,
        "Åpne Syncthing Web GUI",
        true,
        None,
    ));
    let _ = menu.append(&MenuItem::with_id(
        MenuIds::RESCAN,
        "Skann alle mapper",
        true,
        None,
    ));
    let _ = menu.append(&MenuItem::with_id(
        MenuIds::REFRESH,
        "Oppdater status nå",
        true,
        None,
    ));
    let _ = menu.append(&PredefinedMenuItem::separator());
    let _ = menu.append(&MenuItem::with_id(MenuIds::QUIT, "Avslutt", true, None));
    menu
}

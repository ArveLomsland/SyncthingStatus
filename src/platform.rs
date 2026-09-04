//! Plattformspesifikk oppstart, meldingspumpe og feildialog.
//!
//! Windows: Win32-meldingsløkke (`PeekMessage` + `MsgWaitForMultipleObjectsEx`).
//! Linux:   GTK-løkke, siden tray-ikonet går via libappindicator.

/// Må kalles før tray-ikonet opprettes.
pub fn init() -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        gtk::init().map_err(|e| {
            format!("Kunne ikke initialisere GTK ({e}). Kjører du i et grafisk skrivebord?")
        })?;
    }
    Ok(())
}

/// Behandler ventende hendelser og venter deretter opptil `timeout_ms`.
/// Returnerer `false` når appen skal avsluttes.
#[cfg(windows)]
pub fn pump_events(timeout_ms: u32) -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, MsgWaitForMultipleObjectsEx, PeekMessageW, TranslateMessage, MSG,
        PM_REMOVE, QS_ALLINPUT, WM_QUIT,
    };

    unsafe {
        let mut msg: MSG = std::mem::zeroed();
        while PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
            if msg.message == WM_QUIT {
                return false;
            }
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        MsgWaitForMultipleObjectsEx(0, std::ptr::null(), timeout_ms, QS_ALLINPUT, 0);
    }
    true
}

#[cfg(target_os = "linux")]
pub fn pump_events(timeout_ms: u32) -> bool {
    // Kjør GTK-løkken manuelt slik at menyklikk behandles, uten å blokkere
    // hovedtråden slik gtk::main() ville gjort.
    let deadline =
        std::time::Instant::now() + std::time::Duration::from_millis(timeout_ms as u64);
    loop {
        while gtk::events_pending() {
            gtk::main_iteration_do(false);
        }
        let now = std::time::Instant::now();
        if now >= deadline {
            return true;
        }
        std::thread::sleep(std::cmp::min(
            deadline - now,
            std::time::Duration::from_millis(10),
        ));
    }
}

#[cfg(not(any(windows, target_os = "linux")))]
pub fn pump_events(timeout_ms: u32) -> bool {
    std::thread::sleep(std::time::Duration::from_millis(timeout_ms as u64));
    true
}

/// Vis en feilmelding til brukeren (appen har ikke nødvendigvis konsoll).
#[cfg(windows)]
pub fn error_dialog(title: &str, text: &str) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};

    let wide = |s: &str| -> Vec<u16> { s.encode_utf16().chain(std::iter::once(0)).collect() };
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            wide(text).as_ptr(),
            wide(title).as_ptr(),
            MB_OK | MB_ICONERROR,
        );
    }
}

#[cfg(not(windows))]
pub fn error_dialog(title: &str, text: &str) {
    eprintln!("{title}: {text}");
    // Best effort: vis dialog hvis zenity/kdialog finnes
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("zenity")
            .args(["--error", "--title", title, "--text", text])
            .status()
            .or_else(|_| {
                std::process::Command::new("kdialog")
                    .args(["--error", text])
                    .status()
            });
    }
}

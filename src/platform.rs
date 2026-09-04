//! Platform-specific startup, message pump and error dialog.
//!
//! Windows: Win32 message loop (`PeekMessage` + `MsgWaitForMultipleObjectsEx`).
//! Linux:   GTK loop, since the tray icon goes through libappindicator.

/// Must be called before the tray icon is created.
pub fn init() -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        gtk::init().map_err(|e| {
            format!("Could not initialise GTK ({e}). Are you running a graphical desktop?")
        })?;
    }
    Ok(())
}

/// Processes pending events, then waits for up to `timeout_ms`.
/// Returns `false` when the app should exit.
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
    // Drive the GTK loop manually so menu clicks are handled, without blocking
    // the main thread the way gtk::main() would.
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

/// Show an error message to the user (the app may not have a console).
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
    // Best effort: show a dialog if zenity/kdialog is available
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

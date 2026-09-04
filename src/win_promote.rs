//! Windows 11 gjemmer nye tray-ikoner bak «^»-pilen. Innstillingen ligger i
//! HKCU\Control Panel\NotifyIconSettings\<hash>\IsPromoted, der <hash> beregnes
//! av Explorer ut fra filbanen. Vi finner riktig nøkkel ved å sammenligne
//! ExecutablePath, og setter IsPromoted=1 første gang appen kjører.

use std::ffi::c_void;

use windows_sys::Win32::Foundation::ERROR_SUCCESS;
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegEnumKeyExW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW, HKEY,
    HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE, REG_DWORD,
};

const SUBKEY: &str = r"Control Panel\NotifyIconSettings";

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Returnerer `Ok(true)` hvis ikonet ble (eller allerede var) gjort synlig.
pub fn promote() -> Result<bool, String> {
    let exe = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .to_string_lossy()
        .to_string();

    unsafe {
        let mut root: HKEY = std::ptr::null_mut();
        if RegOpenKeyExW(
            HKEY_CURRENT_USER,
            wide(SUBKEY).as_ptr(),
            0,
            KEY_READ,
            &mut root,
        ) != ERROR_SUCCESS
        {
            return Err("fant ikke NotifyIconSettings".into());
        }

        let mut index = 0u32;
        let mut found = false;
        loop {
            let mut name = [0u16; 256];
            let mut name_len = name.len() as u32;
            let rc = RegEnumKeyExW(
                root,
                index,
                name.as_mut_ptr(),
                &mut name_len,
                std::ptr::null(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
            if rc != ERROR_SUCCESS {
                break;
            }
            index += 1;

            let sub = String::from_utf16_lossy(&name[..name_len as usize]);
            let full = format!("{SUBKEY}\\{sub}");

            let mut key: HKEY = std::ptr::null_mut();
            if RegOpenKeyExW(
                HKEY_CURRENT_USER,
                wide(&full).as_ptr(),
                0,
                KEY_READ | KEY_SET_VALUE,
                &mut key,
            ) != ERROR_SUCCESS
            {
                continue;
            }

            let matches = read_string(key, "ExecutablePath")
                .map(|p| p.eq_ignore_ascii_case(&exe))
                .unwrap_or(false);

            if matches {
                if read_dword(key, "IsPromoted") != Some(1) {
                    let one: u32 = 1;
                    let rc = RegSetValueExW(
                        key,
                        wide("IsPromoted").as_ptr(),
                        0,
                        REG_DWORD,
                        &one as *const u32 as *const u8,
                        std::mem::size_of::<u32>() as u32,
                    );
                    if rc != ERROR_SUCCESS {
                        RegCloseKey(key);
                        RegCloseKey(root);
                        return Err(format!("kunne ikke skrive IsPromoted (feil {rc})"));
                    }
                }
                found = true;
            }

            RegCloseKey(key);
            if found {
                break;
            }
        }

        RegCloseKey(root);
        Ok(found)
    }
}

unsafe fn read_string(key: HKEY, name: &str) -> Option<String> {
    let mut buf = [0u16; 1024];
    let mut len = (buf.len() * 2) as u32;
    let rc = RegQueryValueExW(
        key,
        wide(name).as_ptr(),
        std::ptr::null(),
        std::ptr::null_mut(),
        buf.as_mut_ptr() as *mut u8,
        &mut len,
    );
    if rc != ERROR_SUCCESS {
        return None;
    }
    let chars = (len as usize / 2).saturating_sub(1);
    Some(String::from_utf16_lossy(&buf[..chars]))
}

unsafe fn read_dword(key: HKEY, name: &str) -> Option<u32> {
    let mut value: u32 = 0;
    let mut len = std::mem::size_of::<u32>() as u32;
    let rc = RegQueryValueExW(
        key,
        wide(name).as_ptr(),
        std::ptr::null(),
        std::ptr::null_mut(),
        &mut value as *mut u32 as *mut u8,
        &mut len,
    );
    let _ = std::ptr::null::<c_void>();
    if rc == ERROR_SUCCESS {
        Some(value)
    } else {
        None
    }
}

use std::time::Duration;

use windows::core::w;
use windows::Win32::Foundation::{BOOL, GetLastError, ERROR_ALREADY_EXISTS, HANDLE, HINSTANCE, HWND};
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData,
};
use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
use windows::Win32::System::Ole::CF_UNICODETEXT;
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP,
    VIRTUAL_KEY, VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateIconFromResourceEx, LookupIconIdFromDirectoryEx, HICON, LR_DEFAULTCOLOR,
};

pub fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn single_instance() -> bool {
    unsafe {
        let _ = CreateMutexW(None, false, w!("BYOK_STT_SINGLETON"));
        GetLastError() != ERROR_ALREADY_EXISTS
    }
}

pub fn set_clipboard_text(text: &str) -> Result<(), String> {
    unsafe {
        OpenClipboard(HWND::default()).map_err(|e| format!("OpenClipboard failed: {e}"))?;
        let r = set_clipboard_inner(text);
        let _ = CloseClipboard();
        r
    }
}

unsafe fn set_clipboard_inner(text: &str) -> Result<(), String> {
    EmptyClipboard().map_err(|e| format!("EmptyClipboard failed: {e}"))?;
    let wide = to_wide(text);
    let bytes = wide.len() * 2;
    let h = GlobalAlloc(GMEM_MOVEABLE, bytes).map_err(|e| format!("GlobalAlloc failed: {e}"))?;
    let dst = GlobalLock(h) as *mut u16;
    if dst.is_null() {
        return Err("GlobalLock failed".into());
    }
    std::ptr::copy_nonoverlapping(wide.as_ptr(), dst, wide.len());
    let _ = GlobalUnlock(h);
    SetClipboardData(CF_UNICODETEXT.0 as u32, HANDLE(h.0))
        .map(|_| ())
        .map_err(|e| format!("SetClipboardData failed: {e}"))
}

/// Force-clear logically stuck Win/Shift/Alt with synthetic key-ups.
///
/// A system flyout (e.g. the volume mixer) or a focus transition can swallow
/// the physical key-up mid-press, leaving Win logically held (Win+D then
/// minimizes everything, letters trigger Win+ shortcuts). Injected key-ups
/// reset the OS state; a later physical release is a harmless no-op.
pub fn clear_stuck_modifiers() {
    unsafe {
        let mk = |vk: u16| INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(vk),
                    wScan: 0,
                    dwFlags: KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        let clear = [
            mk(VK_LWIN.0),
            mk(VK_RWIN.0),
            mk(VK_SHIFT.0),
            mk(VK_MENU.0),
        ];
        SendInput(&clear, std::mem::size_of::<INPUT>() as i32);
    }
}

/// Simulate Ctrl+V into whatever window has keyboard focus.
///
/// Waits briefly for the user to release Win/Shift/Alt physically, then
/// force-clears any logically stuck modifiers before injecting Ctrl+V.
pub fn send_paste() {
    unsafe {
        let mk = |vk: u16, up: bool| INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(vk),
                    wScan: 0,
                    dwFlags: if up { KEYEVENTF_KEYUP } else { Default::default() },
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        // 1. Give the user a moment to release Win/Shift/Alt physically.
        let down = |vk: u16| GetAsyncKeyState(vk as i32) as u16 & 0x8000 != 0;
        let start = std::time::Instant::now();
        while down(VK_LWIN.0) || down(VK_RWIN.0) || down(VK_SHIFT.0) || down(VK_MENU.0) {
            if start.elapsed() > Duration::from_millis(1200) {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        // 2. Force-clear any logically stuck modifiers, then let the OS settle.
        clear_stuck_modifiers();
        std::thread::sleep(Duration::from_millis(30));
        // 3. Paste into whatever has focus.
        let seq = [
            mk(VK_CONTROL.0, false),
            mk(0x56, false), // V
            mk(0x56, true),
            mk(VK_CONTROL.0, true),
        ];
        SendInput(&seq, std::mem::size_of::<INPUT>() as i32);
    }
}

/// Build an HICON from embedded .ico file bytes (selects the given size).
pub fn load_icon_sized(bytes: &'static [u8], size: i32) -> HICON {
    unsafe {
        let id = LookupIconIdFromDirectoryEx(
            bytes.as_ptr(),
            BOOL::from(true),
            size,
            size,
            LR_DEFAULTCOLOR,
        );
        CreateIconFromResourceEx(
            &bytes[id as usize..],
            BOOL::from(true),
            0x00030000,
            size,
            size,
            LR_DEFAULTCOLOR,
        )
        .expect("CreateIconFromResourceEx")
    }
}

/// Build an HICON from embedded .ico file bytes (selects the 32x32 image).
pub fn load_icon(bytes: &'static [u8]) -> HICON {
    load_icon_sized(bytes, 32)
}

#[allow(unused)]
pub fn null_hinstance() -> HINSTANCE {
    HINSTANCE::default()
}


/// HKCU\Software\Microsoft\Windows\CurrentVersion\Run — per-user
/// autostart, no admin required. Registered command is the CURRENT exe path.
const RUN_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
const RUN_VALUE: &str = "byok-stt";

fn run_key_path() -> Vec<u16> {
    let mut v: Vec<u16> = RUN_KEY.encode_utf16().collect();
    v.push(0);
    v
}

fn run_value_name() -> Vec<u16> {
    let mut v: Vec<u16> = RUN_VALUE.encode_utf16().collect();
    v.push(0);
    v
}

/// Returns the registered startup command line, if any.
pub fn autostart_command() -> Option<String> {
    use windows::Win32::Foundation::ERROR_SUCCESS;
    use windows::Win32::System::Registry::{
        RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_CURRENT_USER, KEY_READ,
        REG_VALUE_TYPE,
    };
    unsafe {
        let mut hkey = HKEY::default();
        if RegOpenKeyExW(
            HKEY_CURRENT_USER,
            windows::core::PCWSTR(run_key_path().as_ptr()),
            0,
            KEY_READ,
            &mut hkey,
        ) != ERROR_SUCCESS
        {
            return None;
        }
        let mut buf = [0u16; 1024];
        let mut size = (buf.len() * 2) as u32;
        let mut ty = REG_VALUE_TYPE::default();
        let ok = RegQueryValueExW(
            hkey,
            windows::core::PCWSTR(run_value_name().as_ptr()),
            None,
            Some(&mut ty),
            Some(buf.as_mut_ptr() as *mut u8),
            Some(&mut size),
        ) == ERROR_SUCCESS;
        let _ = RegCloseKey(hkey);
        if ok {
            let len = buf.iter().position(|&c| c == 0).unwrap_or(0);
            Some(String::from_utf16_lossy(&buf[..len]))
        } else {
            None
        }
    }
}

/// Enable or disable launching byok-stt when Windows starts.
pub fn set_autostart(enable: bool) -> Result<(), String> {
    use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
    use windows::Win32::System::Registry::{
        RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegOpenKeyExW, RegSetValueExW, HKEY,
        HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE, REG_OPTION_NON_VOLATILE, REG_SZ,
    };
    unsafe {
        if enable {
            let mut hkey = HKEY::default();
            let err = RegCreateKeyExW(
                HKEY_CURRENT_USER,
                windows::core::PCWSTR(run_key_path().as_ptr()),
                0,
                None,
                REG_OPTION_NON_VOLATILE,
                KEY_SET_VALUE,
                None,
                &mut hkey,
                None,
            );
            if err != ERROR_SUCCESS {
                return Err(format!("open registry: {err:?}"));
            }
            let exe = std::env::current_exe().map_err(|e| format!("exe path: {e}"))?;
            let cmd = format!("\"{}\"", exe.display());
            let mut data: Vec<u16> = cmd.encode_utf16().collect();
            data.push(0);
            let bytes: Vec<u8> = data.iter().flat_map(|w| w.to_le_bytes()).collect();
            let err = RegSetValueExW(
                hkey,
                windows::core::PCWSTR(run_value_name().as_ptr()),
                0,
                REG_SZ,
                Some(&bytes),
            );
            let _ = RegCloseKey(hkey);
            if err != ERROR_SUCCESS {
                return Err(format!("write registry: {err:?}"));
            }
            Ok(())
        } else {
            let mut hkey = HKEY::default();
            let err = RegOpenKeyExW(
                HKEY_CURRENT_USER,
                windows::core::PCWSTR(run_key_path().as_ptr()),
                0,
                KEY_SET_VALUE | KEY_READ,
                &mut hkey,
            );
            if err != ERROR_SUCCESS {
                return Ok(()); // nothing registered yet
            }
            let err = RegDeleteValueW(hkey, windows::core::PCWSTR(run_value_name().as_ptr()));
            let _ = RegCloseKey(hkey);
            if err == ERROR_SUCCESS || err == ERROR_FILE_NOT_FOUND {
                Ok(())
            } else {
                Err(format!("delete registry value: {err:?}"))
            }
        }
    }
}

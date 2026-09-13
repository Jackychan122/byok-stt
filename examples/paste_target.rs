//! Manual paste-target for verifying --test-paste: opens a window with an
//! EDIT control, holds focus for 8 seconds, then prints the control's text.
//! Usage: run `target/release/examples/paste_target.exe` in the background,
//! then run `byok-stt --test-paste <text>` while it waits.

use std::sync::atomic::{AtomicUsize, Ordering};

use windows::core::w;
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::SetFocus;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, GetWindowTextW,
    PostQuitMessage, RegisterClassW, SetForegroundWindow, SetTimer, ShowWindow, TranslateMessage,
    HMENU, MSG, SW_SHOW, WINDOW_EX_STYLE, WINDOW_STYLE, WNDCLASSW, WM_TIMER, ES_AUTOHSCROLL,
    WS_CHILD, WS_VISIBLE,
};

static EDIT_HWND: AtomicUsize = AtomicUsize::new(0);

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if msg == WM_TIMER {
        let edit = EDIT_HWND.load(Ordering::Relaxed);
        let mut buf = [0u16; 1024];
        let n = GetWindowTextW(HWND(edit as *mut core::ffi::c_void), &mut buf);
        let text = String::from_utf16_lossy(&buf[..(n as usize).min(buf.len())]);
        println!("EDIT_CONTENT_START{text}EDIT_CONTENT_END");
        PostQuitMessage(0);
        return LRESULT(0);
    }
    DefWindowProcW(hwnd, msg, wp, lp)
}

fn main() {
    unsafe {
        let hinstance = GetModuleHandleW(None).unwrap();
        let wc = WNDCLASSW {
            lpfnWndProc: Some(wnd_proc),
            hInstance: HINSTANCE(hinstance.0),
            lpszClassName: w!("PASTE_TARGET"),
            ..Default::default()
        };
        RegisterClassW(&wc);
        let main = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("PASTE_TARGET"),
            w!("paste target"),
            WINDOW_STYLE(0x00CF0000), // WS_OVERLAPPEDWINDOW
            200,
            200,
            520,
            300,
            HWND::default(),
            HMENU::default(),
            HINSTANCE(hinstance.0),
            None,
        )
        .expect("window");
        let edit = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("EDIT"),
            w!(""),
            WINDOW_STYLE(WS_CHILD.0 | WS_VISIBLE.0 | ES_AUTOHSCROLL as u32),
            5,
            5,
            490,
            250,
            main,
            HMENU::default(),
            HINSTANCE(hinstance.0),
            None,
        )
        .expect("edit");
        EDIT_HWND.store(edit.0 as usize, Ordering::Relaxed);
        let _ = ShowWindow(main, SW_SHOW);
        let _ = SetForegroundWindow(main);
        let _ = SetFocus(edit);
        let _ = SetTimer(main, 1, 15000, None);

        let mut msg = MSG::default();
        loop {
            let r = GetMessageW(&mut msg, None, 0, 0);
            if r.0 <= 0 {
                break;
            }
            let _ = TranslateMessage(&msg);
        }
    }
}

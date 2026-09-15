use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HINSTANCE, HMODULE, HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY,
    NIIF_ERROR, NIIF_INFO, NOTIFYICONDATAW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CallNextHookEx, CreatePopupMenu, CreateWindowExW, DefWindowProcW,
    DispatchMessageW, DestroyMenu, FindWindowW,
    DestroyWindow, GetMessageW, GetCursorPos, IsDialogMessageW, IsWindow, KillTimer,
    PostMessageW,
    RegisterClassW, SetForegroundWindow, SetTimer, SetWindowsHookExW, TrackPopupMenu,
    TranslateMessage, UnhookWindowsHookEx, HHOOK, HMENU, KBDLLHOOKSTRUCT, MENU_ITEM_FLAGS, MSG,
    TPM_NONOTIFY, TPM_RETURNCMD, TPM_RIGHTBUTTON, WH_KEYBOARD_LL, WINDOW_EX_STYLE, WINDOW_STYLE,
    WNDCLASSW, WM_APP, WM_KEYDOWN, WM_LBUTTONUP, WM_RBUTTONUP, WM_SYSKEYDOWN,
    WM_TIMER, WS_OVERLAPPED,
};

use crate::{audio, config, indicator, logging, settings, stt, winutil};

const WMAPP_TRAY: u32 = WM_APP;
const WMAPP_START: u32 = WM_APP + 1;
const WMAPP_STOP: u32 = WM_APP + 2;
const WMAPP_RESULT: u32 = WM_APP + 3;
const WMAPP_RELOAD: u32 = WM_APP + 4;
const WMAPP_PING: u32 = WM_APP + 5;
const TIMER_MAX_REC: usize = 1;
const MIN_REC: Duration = Duration::from_millis(300);
static MAX_REC_MS: AtomicUsize = AtomicUsize::new(120_000);

const ID_SETTINGS: i32 = 1001;
const ID_EXIT: i32 = 1002;

const IDLE_ICO: &[u8] = include_bytes!("../assets/idle.ico");
const REC_ICO: &[u8] = include_bytes!("../assets/rec.ico");
const BUSY_ICO: &[u8] = include_bytes!("../assets/busy.ico");
static MAIN_HWND: AtomicUsize = AtomicUsize::new(0);
static MOD_ID: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(crate::hotkey::MOD_CTRL);
static KEY_VK: std::sync::atomic::AtomicU16 = std::sync::atomic::AtomicU16::new(crate::hotkey::VK_LWIN);
static MOD_DOWN: AtomicBool = AtomicBool::new(false);
static KEY_DOWN: AtomicBool = AtomicBool::new(false);
static IN_SESSION: AtomicBool = AtomicBool::new(false);
static SWALLOW_WIN: AtomicBool = AtomicBool::new(false);

enum Phase {
    Idle,
    Recording { recorder: audio::Recorder },
    Busy,
}

thread_local! {
    static PHASE: RefCell<Phase> = RefCell::new(Phase::Idle);
}

pub fn run() {
    if !winutil::single_instance() {
        // Another instance owns the tray. Wake it so the user gets feedback
        // (balloon) instead of a seemingly-dead click on the app icon.
        logging::log("another instance already running, showing its balloon");
        unsafe {
            let existing = FindWindowW(w!("BYOK_STT_MAIN"), PCWSTR::null());
            if let Ok(h) = existing {
                if !h.is_invalid() {
                    let _ = PostMessageW(h, WMAPP_PING, WPARAM(0), LPARAM(0));
                }
            }
        }
        return;
    }
    unsafe {
        let hinstance = GetModuleHandleW(None).unwrap();
        let wc = WNDCLASSW {
            lpfnWndProc: Some(wnd_proc),
            hInstance: HINSTANCE(hinstance.0),
            lpszClassName: w!("BYOK_STT_MAIN"),
            ..Default::default()
        };
        let atom = RegisterClassW(&wc);
        if atom == 0 {
            let e = windows::Win32::Foundation::GetLastError();
            logging::log(&format!("RegisterClassW failed: {:?}", e));
        }
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("BYOK_STT_MAIN"),
            w!("byok-stt"),
            WINDOW_STYLE(WS_OVERLAPPED.0),
            0,
            0,
            0,
            0,
            HWND::default(),
            HMENU::default(),
            HINSTANCE(hinstance.0),
            None,
        )
        .map_err(|e| format!("CreateWindowExW failed: {e:?} + GetLastError={:?}", windows::Win32::Foundation::GetLastError()))
        .expect("create main window");
        MAIN_HWND.store(hwnd.0 as usize, Ordering::Relaxed);

        add_tray_icon(hwnd, IDLE_ICO);
        indicator::init(hwnd);
        reload_settings();

        // First-run onboarding: without an API key the app cannot do its
        // one job, so surface Settings immediately instead of letting the
        // user discover the error only after their first failed dictation.
        if config::load().api_key.trim().is_empty() {
            settings::open(hwnd);
            balloon(
                hwnd,
                "Welcome to byok-stt",
                "Paste your API key, pick a model and press Save to start dictating.",
                false,
            );
        }

        let hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), HMODULE::default(), 0)
            .expect("install keyboard hook");

        let mut msg = MSG::default();
        loop {
            let r = GetMessageW(&mut msg, HWND::default(), 0, 0);
            if r.0 <= 0 {
                break;
            }
            // Settings window (when open) gets Tab/Enter dialog navigation.
            let handled = match crate::settings::dialog_hwnd() {
                Some(d) => IsDialogMessageW(d, &mut msg).as_bool(),
                None => false,
            };
            if !handled {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            // Exit (DestroyWindow) must end the loop: without this check
            // GetMessageW would block forever (the LL hook keeps the thread
            // alive) and the process would never exit.
            let h = HWND(MAIN_HWND.load(Ordering::Relaxed) as *mut core::ffi::c_void);
            if h.is_invalid() || !IsWindow(h).as_bool() {
                break;
            }
        }
        let _ = UnhookWindowsHookEx(hook);
        remove_tray_icon(hwnd);
    }
}

extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        if code >= 0 && lparam.0 != 0 {
            let st = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
            let vk = st.vkCode as u16;
            let msg = wparam.0 as u32;
            let down = msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN;
            let mod_id = MOD_ID.load(Ordering::Relaxed);
            let key_vk = KEY_VK.load(Ordering::Relaxed);
            let is_mod = crate::hotkey::is_modifier_vk(mod_id, vk);
            let is_key = crate::hotkey::is_key_vk(key_vk, vk);

            if is_mod {
                MOD_DOWN.store(down, Ordering::Relaxed);
            }
            if is_key {
                KEY_DOWN.store(down, Ordering::Relaxed);
            }

            if !IN_SESSION.load(Ordering::Relaxed)
                && down
                && MOD_DOWN.load(Ordering::Relaxed)
                && KEY_DOWN.load(Ordering::Relaxed)
            {
                IN_SESSION.store(true, Ordering::Relaxed);
                SWALLOW_WIN.store(crate::hotkey::uses_win(mod_id, key_vk), Ordering::Relaxed);
                logging::log("hotkey: session start");
                post_main(WMAPP_START);
            } else if IN_SESSION.load(Ordering::Relaxed) && !down && (is_mod || is_key) {
                IN_SESSION.store(false, Ordering::Relaxed);
                logging::log("hotkey: session end (keys released)");
                post_main(WMAPP_STOP);
            }

            // While a dictate session is active, hide the Win key from the OS
            // (when it is part of the hotkey) so releasing it never opens the
            // Start menu.
            let win_side = vk == crate::hotkey::VK_LWIN || vk == crate::hotkey::VK_RWIN;
            if SWALLOW_WIN.load(Ordering::Relaxed) && win_side {
                if !down {
                    SWALLOW_WIN.store(false, Ordering::Relaxed);
                }
                return LRESULT(1);
            }
        }
        CallNextHookEx(HHOOK::default(), code, wparam, lparam)
    }
}

/// Re-read the hotkey (and bubble visibility) from config.json.
pub fn reload_settings() {
    let cfg = config::load();
    if let Some(m) = crate::hotkey::parse_modifier(&cfg.hotkey_modifier) {
        MOD_ID.store(m, Ordering::Relaxed);
    }
    if let Some(k) = crate::hotkey::parse_key(&cfg.hotkey_key) {
        KEY_VK.store(k, Ordering::Relaxed);
    }
    MAX_REC_MS.store(
        cfg.max_recording_secs.clamp(5, 3600) as usize * 1000,
        Ordering::Relaxed,
    );
    indicator::set_always_visible(cfg.bubble_always_visible);
    logging::log(&format!(
        "hotkey reloaded: {}+{}",
        cfg.hotkey_modifier, cfg.hotkey_key
    ));
}

/// Post WMAPP_RELOAD to the running tray process so hotkey/bubble changes
/// apply immediately. No-op when settings run standalone (separate process).
pub fn notify_reload() {
    post_main(WMAPP_RELOAD);
}

/// Balloon anchored to the tray icon; usable from other modules.
pub fn balloon_main(title: &str, msg: &str) {
    let hwnd = MAIN_HWND.load(Ordering::Relaxed);
    if hwnd != 0 {
        balloon(HWND(hwnd as *mut core::ffi::c_void), title, msg, false);
    }
}
 
fn post_main(msg: u32) {
    let hwnd = MAIN_HWND.load(Ordering::Relaxed);
    if hwnd != 0 {
        unsafe {
            let _ = PostMessageW(
                HWND(hwnd as *mut core::ffi::c_void),
                msg,
                WPARAM(0),
                LPARAM(0),
            );
        }
    }
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    crate::logging::log(&format!("main wnd_proc msg=0x{msg:04X}"));
    match msg {
        WM_TIMER => {
            on_timer(hwnd);
            LRESULT(0)
        }
        WMAPP_START => {
            on_start(hwnd);
            LRESULT(0)
        }
        WMAPP_STOP => {
            on_stop(hwnd);
            LRESULT(0)
        }
        WMAPP_TRAY => {
            on_tray(hwnd, lparam);
            LRESULT(0)
        }
        WMAPP_RELOAD => {
            reload_settings();
            LRESULT(0)
        }
        WMAPP_PING => {
            balloon(
                hwnd,
                "byok-stt is already running",
                "Hold the hotkey to dictate, or right-click the microphone icon in the system tray (it may be inside the ^ hidden-icons overflow).",
                false,
            );
            LRESULT(0)
        }
        WMAPP_RESULT => {
            on_result(hwnd, lparam);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
static SESSION_START_MS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Watchdog: hard-stop a session that exceeds MAX_REC (safety net).
fn on_timer(hwnd: HWND) {
    let started = SESSION_START_MS.load(Ordering::Relaxed);
    if started == 0 {
        return;
    }
    if now_millis().saturating_sub(started) >= MAX_REC_MS.load(Ordering::Relaxed) as u64 {
        logging::log("watchdog: max recording time reached, stopping");
        on_stop(hwnd);
    }
}

fn on_start(hwnd: HWND) {
    PHASE.with(|p| {
        let mut p = p.borrow_mut();
        if !matches!(*p, Phase::Idle) {
            return;
        }
        match audio::start() {
            Ok(rec) => {
                SESSION_START_MS.store(now_millis(), Ordering::Relaxed);
                *p = Phase::Recording { recorder: rec };
                unsafe {
                    let _ = SetTimer(
                        hwnd,
                        TIMER_MAX_REC,
                        MAX_REC_MS.load(Ordering::Relaxed) as u32,
                        None,
                    );
                }
                set_icon(hwnd, REC_ICO, "listening");
                indicator::show_recording();
            }
            Err(e) => {
                logging::log(&format!("mic error: {e}"));
                balloon(hwnd, "Microphone error", &e, true);
            }
        }
    });
}

fn on_stop(hwnd: HWND) {
    let taken = PHASE.with(|p| std::mem::replace(&mut *p.borrow_mut(), Phase::Idle));
    match taken {
        Phase::Recording { recorder } => unsafe {
            let _ = KillTimer(hwnd, TIMER_MAX_REC);
            match recorder.stop() {
                Err(e) => {
                    logging::log(&format!("record stop error: {e}"));
                    set_icon(hwnd, IDLE_ICO, "idle");
                    indicator::hide();
                    balloon(hwnd, "Recording error", &e, true);
                }
                Ok((wav, dur)) => {
                    if dur < MIN_REC {
                        // accidental tap, ignore
                        set_icon(hwnd, IDLE_ICO, "idle");
                        indicator::hide();
                        winutil::clear_stuck_modifiers();
                    } else {
                        set_icon(hwnd, BUSY_ICO, "transcribing");
                        indicator::show_busy();
                        let main = MAIN_HWND.load(Ordering::Relaxed);
                        std::thread::spawn(move || {
                            let result = stt::transcribe_wav(&wav);
                            let raw = Box::into_raw(Box::new(result));
                            let _ = PostMessageW(
                                HWND(main as *mut core::ffi::c_void),
                                WMAPP_RESULT,
                                WPARAM(0),
                                LPARAM(raw as isize),
                            );
                        });
                        PHASE.with(|p| *p.borrow_mut() = Phase::Busy);
                    }
                }
            }
        },
        other => PHASE.with(|p| *p.borrow_mut() = other),
    }
}

fn on_result(hwnd: HWND, lparam: LPARAM) {
    if lparam.0 == 0 {
        return;
    }
    let result = unsafe { Box::from_raw(lparam.0 as *mut Result<String, String>) };
    set_icon(hwnd, IDLE_ICO, "idle");
    indicator::hide();
    PHASE.with(|p| *p.borrow_mut() = Phase::Idle);
    // The session keys were released a while ago; if anything swallowed the
    // physical key-up (flyout, focus switch), Win is still logically held.
    winutil::clear_stuck_modifiers();
    match *result {
        Ok(text) => {
            if text.is_empty() {
                balloon(hwnd, "Transcription empty", "The model returned no text.", true);
                return;
            }
            match winutil::set_clipboard_text(&text) {
                Ok(()) => {
                    std::thread::sleep(Duration::from_millis(60));
                    winutil::send_paste();
                }
                Err(e) => {
                    logging::log(&format!("clipboard error: {e}"));
                    balloon(hwnd, "Clipboard error", &e, true);
                }
            }
        }
        Err(e) => {
            logging::log(&format!("transcription error: {e}"));
            if e.contains("401") {
                balloon(
                    hwnd,
                    "Invalid API key",
                    "The provider rejected the API key (HTTP 401). Update it in tray > Settings.",
                    true,
                );
            } else if e.contains("not configured") {
                balloon(
                    hwnd,
                    "API key missing",
                    "Add your provider API key in tray > Settings.",
                    true,
                );
            } else {
                balloon(hwnd, "Transcription failed", &e, true);
            }
        }
    }
}

fn on_tray(hwnd: HWND, lparam: LPARAM) {
    let m = lparam.0 as u32;
    if m == WM_LBUTTONUP {
        // Left click toggles recording: start when idle, stop when recording.
        let idle = PHASE.with(|p| matches!(&*p.borrow(), Phase::Idle));
        let recording = PHASE.with(|p| matches!(&*p.borrow(), Phase::Recording { .. }));
        if idle {
            on_start(hwnd);
        } else if recording {
            on_stop(hwnd);
        } else {
            balloon(
                hwnd,
                "Still transcribing",
                "Wait for the current transcription to finish.",
                false,
            );
        }
        return;
    }
    if m != WM_RBUTTONUP {
        return;
    }
    unsafe {
        let mut pt = POINT::default();
        let _ = GetCursorPos(&mut pt);
        let hmenu = CreatePopupMenu().expect("CreatePopupMenu");
        let _ = AppendMenuW(hmenu, MENU_ITEM_FLAGS(MF_STRING), ID_SETTINGS as usize, w!("Settings"));
        let _ = AppendMenuW(hmenu, MENU_ITEM_FLAGS(MF_SEPARATOR), 0, PCWSTR::null());
        let _ = AppendMenuW(hmenu, MENU_ITEM_FLAGS(MF_STRING), ID_EXIT as usize, w!("Exit"));
        let _ = SetForegroundWindow(hwnd);
        let cmd = TrackPopupMenu(
            hmenu,
            TPM_RIGHTBUTTON | TPM_RETURNCMD | TPM_NONOTIFY,
            pt.x,
            pt.y,
            0,
            hwnd,
            None,
        );
        let _ = DestroyMenu(hmenu);
        match cmd.0 {
            ID_SETTINGS => settings::open(hwnd),
            ID_EXIT => {
                let _ = DestroyWindow(hwnd);
            }
            _ => {}
        }
    }
}

const MF_STRING: u32 = 0x0000_0000;
const MF_SEPARATOR: u32 = 0x0000_0800;

fn add_tray_icon(hwnd: HWND, ico: &'static [u8]) {
    unsafe {
        let mut nid = NOTIFYICONDATAW::default();
        nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        nid.hWnd = hwnd;
        nid.uID = 1;
        nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
        nid.uCallbackMessage = WMAPP_TRAY;
        nid.hIcon = winutil::load_icon(ico);
        set_utf16(&mut nid.szTip, "byok-stt: hold the hotkey to dictate");
        let ok = Shell_NotifyIconW(NIM_ADD, &nid);
        logging::log(&format!("tray icon add: {}", ok.as_bool()));
    }
}

fn remove_tray_icon(hwnd: HWND) {
    unsafe {
        let mut nid = NOTIFYICONDATAW::default();
        nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        nid.hWnd = hwnd;
        nid.uID = 1;
        let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
    }
}

fn set_icon(hwnd: HWND, ico: &'static [u8], state: &str) {
    unsafe {
        let mut nid = NOTIFYICONDATAW::default();
        nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        nid.hWnd = hwnd;
        nid.uID = 1;
        nid.uFlags = NIF_ICON;
        nid.hIcon = winutil::load_icon(ico);
        let ok = Shell_NotifyIconW(NIM_MODIFY, &nid);
        logging::log(&format!("tray icon state -> {state}: {}", ok.as_bool()));
    }
}

fn balloon(hwnd: HWND, title: &str, msg: &str, error: bool) {
    unsafe {
        let mut nid = NOTIFYICONDATAW::default();
        nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        nid.hWnd = hwnd;
        nid.uID = 1;
        nid.uFlags = NIF_INFO;
        nid.dwInfoFlags = if error { NIIF_ERROR } else { NIIF_INFO };
        set_utf16(&mut nid.szInfoTitle, title);
        set_utf16(&mut nid.szInfo, msg);
        let _ = Shell_NotifyIconW(NIM_MODIFY, &nid);
    }
}

fn set_utf16(arr: &mut [u16], s: &str) {
    for (i, c) in s.encode_utf16().take(arr.len() - 1).enumerate() {
        arr[i] = c;
    }
}

use std::cell::RefCell;

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateFontW, GetSysColorBrush, SetBkMode, SetTextColor, CLIP_DEFAULT_PRECIS, COLOR_WINDOW,
    DEFAULT_CHARSET, DEFAULT_QUALITY, FONT_PITCH, FW_NORMAL, HBRUSH, HFONT, HDC,
    OUT_DEFAULT_PRECIS, TRANSPARENT,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::Input::KeyboardAndMouse::SetFocus;
use windows::Win32::UI::WindowsAndMessaging::{
    BS_AUTOCHECKBOX, BS_DEFPUSHBUTTON, CBS_AUTOHSCROLL, CBS_DROPDOWN, CBS_DROPDOWNLIST,
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, ES_AUTOHSCROLL,
    ES_PASSWORD, GetDlgItem, GetMessageW, GetSystemMetrics, HMENU, IsDialogMessageW,
    MB_ICONWARNING, MB_OK, MessageBoxW, MSG, PostQuitMessage, RegisterClassW, SM_CXSCREEN,
    SM_CYSCREEN, SHOW_WINDOW_CMD, SW_SHOW, SendMessageW, SetForegroundWindow, SetWindowTextW,
    ShowWindow, TranslateMessage, WINDOW_EX_STYLE, WINDOW_STYLE, WINDOW_LONG_PTR_INDEX, WNDCLASSW,
    WM_CLOSE, WM_COMMAND, WM_CREATE, WM_DESTROY, WM_SETFONT, WS_BORDER, WS_CAPTION, WS_CHILD,
    WS_OVERLAPPED, WS_SYSMENU, WS_TABSTOP, WS_VISIBLE,
};

use crate::{config, winutil};

const APP_ICO: &[u8] = include_bytes!("../assets/app.ico");

const IDC_PROVIDER: i32 = 2001;
const IDC_MAXREC: i32 = 2011;
const IDC_PROMPT: i32 = 2012;
const IDC_BASE: i32 = 2002;
const IDC_KEY: i32 = 2003;
const IDC_MODEL: i32 = 2004;
const IDC_SAVE: i32 = 2005;
const IDC_CANCEL: i32 = 2006;
const IDC_HINT: i32 = 2007;
const IDC_HK_MOD: i32 = 2008;
const IDC_HK_KEY: i32 = 2009;
const IDC_ALWAYS: i32 = 2010;

/// (display name, base URL, default model, model suggestions)
const PROVIDERS: &[(&str, &str, &str, &[&str])] = &[
    (
        "OpenRouter",
        "https://openrouter.ai/api/v1",
        "openai/whisper-large-v3-turbo",
        &[
            "openai/whisper-large-v3-turbo",
            "mistralai/voxtral-small-24b-2507",
            "google/gemini-2.5-flash-lite",
            "openai/gpt-4o-mini-transcribe",
        ],
    ),
    (
        "OpenAI",
        "https://api.openai.com/v1",
        "gpt-4o-mini-transcribe",
        &["gpt-4o-mini-transcribe", "gpt-4o-transcribe", "whisper-1"],
    ),
    (
        "Groq",
        "https://api.groq.com/openai/v1",
        "whisper-large-v3-turbo",
        &["whisper-large-v3-turbo", "whisper-large-v3"],
    ),
    (
        "Google Gemini",
        "https://generativelanguage.googleapis.com/v1beta/openai",
        "gemini-2.5-flash-lite",
        &["gemini-2.5-flash-lite", "gemini-2.5-flash", "gemini-2.5-pro"],
    ),
    (
        "Mistral",
        "https://api.mistral.ai/v1",
        "voxtral-small-24b-2507",
        &["voxtral-small-24b-2507", "voxtral-mini-24b-2507"],
    ),
    (
        "ElevenLabs",
        "https://api.elevenlabs.io/v1",
        "scribe_v1",
        &["scribe_v1"],
    ),
    ("Custom", "", "", &[]),
];

/// Modifier choices shown in the hotkey combo (index = value sent to hotkey.rs).
const HK_MODIFIERS: &[&str] = &["Ctrl", "Alt", "Shift", "Win"];
/// Trigger-key suggestions for the editable hotkey key combo.
const HK_KEYS: &[&str] = &[
    "Win", "Space", "A", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q",
    "R", "S", "T", "U", "V", "X", "Y", "Z", "F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9",
];

thread_local! {
    static OPEN: RefCell<Option<HWND>> = const { RefCell::new(None) };
    static STANDALONE: RefCell<bool> = const { RefCell::new(false) };
}

/// The settings window handle, if open. The tray loop pumps messages through
/// `IsDialogMessageW` for this window so Tab/Enter work in the embedded case.
pub fn dialog_hwnd() -> Option<HWND> {
    OPEN.with(|o| *o.borrow())
}

/// Open the settings window with its own message loop (`byok-stt --settings`).
pub fn run_standalone() {
    STANDALONE.with(|s| *s.borrow_mut() = true);
    open(HWND::default());
    unsafe {
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let handled = match dialog_hwnd() {
                Some(d) => IsDialogMessageW(d, &mut msg).as_bool(),
                None => false,
            };
            if !handled {
                let _ = TranslateMessage(&msg);
                let _ = DispatchMessageW(&msg);
            }
        }
    }
}

pub fn open(parent: HWND) {
    crate::logging::log("settings window opening");
    if let Some(h) = dialog_hwnd() {
        unsafe {
            let _ = ShowWindow(h, SHOW_WINDOW_CMD(SW_SHOW.0));
            let _ = SetForegroundWindow(h);
        }
        return;
    }
    unsafe {
        let hinstance = GetModuleHandleW(None).unwrap();
        let class = w!("BYOK_STT_SETTINGS");
        let wc = WNDCLASSW {
            lpfnWndProc: Some(settings_proc),
            hInstance: HINSTANCE(hinstance.0),
            lpszClassName: class,
            hbrBackground: HBRUSH(GetSysColorBrush(COLOR_WINDOW).0),
            ..Default::default()
        };
        RegisterClassW(&wc);
        let sw = GetSystemMetrics(SM_CXSCREEN);
        let sh = GetSystemMetrics(SM_CYSCREEN);
        let (w, h) = (472, 358);
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            class,
            w!("byok-stt Settings"),
            WINDOW_STYLE(WS_OVERLAPPED.0 | WS_CAPTION.0 | WS_SYSMENU.0),
            (sw - w) / 2,
            (sh - h) / 2,
            w,
            h,
            parent,
            HMENU::default(),
            HINSTANCE(hinstance.0),
            None,
        )
        .expect("settings window");
        OPEN.with(|o| *o.borrow_mut() = Some(hwnd));
        let _ = ShowWindow(hwnd, SHOW_WINDOW_CMD(SW_SHOW.0));
        if !STANDALONE.with(|s| *s.borrow()) {
            let _ = SetForegroundWindow(hwnd);
        }
    }
}

unsafe fn ctl(
    parent: HWND,
    class: PCWSTR,
    style: WINDOW_STYLE,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    id: i32,
    text: PCWSTR,
) -> HWND {
    unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            class,
            text,
            WINDOW_STYLE(style.0 | WS_CHILD.0 | WS_VISIBLE.0),
            x,
            y,
            w,
            h,
            parent,
            HMENU(id as usize as *mut core::ffi::c_void),
            HINSTANCE::default(),
            None,
        )
        .expect("create control")
    }
}

fn read_ctl(parent: HWND, id: i32) -> String {
    unsafe {
        let h = match GetDlgItem(parent, id) {
            Ok(h) => h,
            Err(_) => return String::new(),
        };
        let mut buf = [0u16; 4096];
        let n = windows::Win32::UI::WindowsAndMessaging::GetWindowTextW(h, &mut buf);
        String::from_utf16_lossy(&buf[..(n as usize).min(buf.len())])
    }
}

fn set_ctl(parent: HWND, id: i32, text: &str) {
    unsafe {
        if let Ok(h) = GetDlgItem(parent, id) {
            let wide = winutil::to_wide(text);
            let _ = SetWindowTextW(h, PCWSTR(wide.as_ptr()));
        }
    }
}

/// Fill the base URL edit and the model combo from a provider preset.
fn apply_provider(hwnd: HWND, idx: i32) {
    let (_, base, default_model, models) = PROVIDERS
        .get(idx as usize)
        .copied()
        .unwrap_or(PROVIDERS[0]);
    if !base.is_empty() {
        set_ctl(hwnd, IDC_BASE, base);
    }
    unsafe {
        if let Ok(cb) = GetDlgItem(hwnd, IDC_MODEL) {
            let _ = SendMessageW(cb, 0x014B, WPARAM(0), LPARAM(0)); // CB_RESETCONTENT
            for m in models {
                let wide = winutil::to_wide(m);
                let _ = SendMessageW(cb, 0x0143, WPARAM(0), LPARAM(wide.as_ptr() as isize)); // CB_ADDSTRING
            }
        }
    }
    set_ctl(hwnd, IDC_MODEL, default_model);
}

/// Combo helper: fill with strings and select `sel` (or set editable text).
unsafe fn fill_combo(hwnd: HWND, id: i32, items: &[&str], sel_text: &str) {
    unsafe {
        if let Ok(cb) = GetDlgItem(hwnd, id) {
            let _ = SendMessageW(cb, 0x014B, WPARAM(0), LPARAM(0)); // CB_RESETCONTENT
            let mut sel = -1i32;
            for (i, it) in items.iter().enumerate() {
                let wide = winutil::to_wide(it);
                let _ = SendMessageW(cb, 0x0143, WPARAM(0), LPARAM(wide.as_ptr() as isize));
                if it.eq_ignore_ascii_case(sel_text) {
                    sel = i as i32;
                }
            }
            if sel >= 0 {
                let _ = SendMessageW(cb, 0x014E, WPARAM(sel as usize), LPARAM(0)); // CB_SETCURSEL
            } else if !sel_text.is_empty() {
                set_ctl(hwnd, id, sel_text);
            }
        }
    }
}

unsafe extern "system" fn settings_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_CREATE => unsafe {
            let dpi = GetDpiForWindow(hwnd).max(96);
            let s: f32 = dpi as f32 / 96.0;
            let px = |v: i32| -> i32 { (v as f32 * s).round() as i32 };
            // Segoe UI 9pt, negative height = character height.
            let font_height: i32 = -((9.0 * (dpi as f32) / 72.0).round() as i32);
            let font: HFONT = CreateFontW(
                font_height,
                0, // width
                0,
                0,
                FW_NORMAL.0 as i32,
                0,
                0,
                0,
                DEFAULT_CHARSET.0 as u32,
                OUT_DEFAULT_PRECIS.0 as u32,
                CLIP_DEFAULT_PRECIS.0 as u32,
                DEFAULT_QUALITY.0 as u32,
                FONT_PITCH(0).0 as u32,
                PCWSTR(winutil::to_wide("Segoe UI").as_ptr()),
            );
            let apply = |h: HWND| {
                let _ = SendMessageW(h, WM_SETFONT, WPARAM(font.0 as usize), LPARAM(1));
            };
            let label = WINDOW_STYLE(0);
            let edit = WINDOW_STYLE(ES_AUTOHSCROLL as u32 | WS_TABSTOP.0 | WS_BORDER.0);
            let combo_list = WINDOW_STYLE(CBS_DROPDOWNLIST as u32 | WS_TABSTOP.0);
            let combo_edit =
                WINDOW_STYLE(CBS_DROPDOWN as u32 | CBS_AUTOHSCROLL as u32 | WS_TABSTOP.0);
            let check = WINDOW_STYLE(BS_AUTOCHECKBOX as u32 | WS_TABSTOP.0);

            let l1 = ctl(hwnd, w!("STATIC"), label, px(12), px(19), px(84), px(16), 0, w!("Provider:"));
            let prov = ctl(hwnd, w!("COMBOBOX"), combo_list, px(100), px(16), px(356), px(180), IDC_PROVIDER, PCWSTR::null());
            let l2 = ctl(hwnd, w!("STATIC"), label, px(12), px(51), px(84), px(16), 0, w!("API base URL:"));
            let base = ctl(hwnd, w!("EDIT"), edit, px(100), px(48), px(356), px(22), IDC_BASE, PCWSTR::null());
            let l3 = ctl(hwnd, w!("STATIC"), label, px(12), px(83), px(84), px(16), 0, w!("API key:"));
            let key = ctl(hwnd, w!("EDIT"), WINDOW_STYLE(edit.0 | ES_PASSWORD as u32), px(100), px(80), px(356), px(22), IDC_KEY, PCWSTR::null());
            let l4 = ctl(hwnd, w!("STATIC"), label, px(12), px(115), px(84), px(16), 0, w!("Model:"));
            let model = ctl(hwnd, w!("COMBOBOX"), combo_edit, px(100), px(112), px(356), px(180), IDC_MODEL, PCWSTR::null());
            let l_prompt = ctl(hwnd, w!("STATIC"), label, px(12), px(146), px(84), px(16), 0, w!("Prompt:"));
            let prompt = ctl(hwnd, w!("EDIT"), edit, px(100), px(143), px(356), px(22), IDC_PROMPT, PCWSTR::null());
            let hint = ctl(hwnd, w!("STATIC"), label, px(12), px(172), px(444), px(30), IDC_HINT, w!("File models (whisper / voxtral / *-transcribe / scribe) use /audio/transcriptions; others use /chat/completions with inline audio."));

            let l5 = ctl(hwnd, w!("STATIC"), label, px(12), px(211), px(84), px(16), 0, w!("Hotkey:"));
            let hk_mod = ctl(hwnd, w!("COMBOBOX"), combo_list, px(100), px(208), px(90), px(180), IDC_HK_MOD, PCWSTR::null());
            let plus = ctl(hwnd, w!("STATIC"), label, px(194), px(211), px(12), px(16), 0, w!("+"));
            let hk_key = ctl(hwnd, w!("COMBOBOX"), combo_edit, px(210), px(208), px(110), px(180), IDC_HK_KEY, PCWSTR::null());
            let always = ctl(hwnd, w!("BUTTON"), check, px(100), px(240), px(236), px(18), IDC_ALWAYS, w!("Keep indicator bubble always visible"));
            let lmr = ctl(hwnd, w!("STATIC"), label, px(344), px(240), px(48), px(16), 0, w!("Max rec:"));
            let maxrec = ctl(hwnd, w!("EDIT"), edit, px(394), px(240), px(62), px(20), IDC_MAXREC, PCWSTR::null());

            let save = ctl(hwnd, w!("BUTTON"), WINDOW_STYLE(BS_DEFPUSHBUTTON as u32 | WS_TABSTOP.0), px(288), px(274), px(80), px(28), IDC_SAVE, w!("Save"));
            let cancel = ctl(hwnd, w!("BUTTON"), WINDOW_STYLE(WS_TABSTOP.0 as u32), px(376), px(274), px(80), px(28), IDC_CANCEL, w!("Cancel"));
            for h in [l1, prov, l2, base, l3, key, l4, model, l_prompt, prompt, hint, l5, hk_mod, plus, hk_key, always, lmr, maxrec, save, cancel] {
                apply(h);
            }

            // Load current config; match api_base to a preset when possible.
            let cfg = config::load();
            let cur_base = cfg.api_base.trim_end_matches('/').to_lowercase();
            let idx = PROVIDERS
                .iter()
                .position(|(_, b, _, _)| !b.is_empty() && b.to_lowercase() == cur_base)
                .unwrap_or(PROVIDERS.len() - 1) as i32;
            for (name, _, _, _) in PROVIDERS {
                let wide = winutil::to_wide(name);
                let _ = SendMessageW(prov, 0x0143, WPARAM(0), LPARAM(wide.as_ptr() as isize)); // CB_ADDSTRING
            }
            let _ = SendMessageW(prov, 0x014E, WPARAM(idx as usize), LPARAM(0)); // CB_SETCURSEL
            apply_provider(hwnd, idx);
            if !cfg.api_base.trim().is_empty() {
                set_ctl(hwnd, IDC_BASE, &cfg.api_base);
            }
            if !cfg.model.trim().is_empty() {
                set_ctl(hwnd, IDC_MODEL, cfg.model.trim());
            }
            if !cfg.api_key.trim().is_empty() {
                set_ctl(hwnd, IDC_KEY, &cfg.api_key);
            }
            set_ctl(hwnd, IDC_PROMPT, cfg.stt_prompt.trim());

            // Hotkey combos (macOS "cmd" shows as Win; parser maps them together).
            let mod_disp = if cfg.hotkey_modifier.eq_ignore_ascii_case("cmd") { "Win" } else { &cfg.hotkey_modifier };
            fill_combo(hwnd, IDC_HK_MOD, HK_MODIFIERS, mod_disp);
            fill_combo(hwnd, IDC_HK_KEY, HK_KEYS, &cfg.hotkey_key);
            set_ctl(hwnd, IDC_MAXREC, &cfg.max_recording_secs.to_string());
            let _ = SendMessageW(
                GetDlgItem(hwnd, IDC_ALWAYS).unwrap_or_default(),
                0x00F1, // BM_SETCHECK
                WPARAM(cfg.bubble_always_visible as usize * 2), // BST_CHECKED
                LPARAM(0),
            );
            // App icon: small = title bar, big = Alt-Tab.
            let icon_big = winutil::load_icon_sized(APP_ICO, 32);
            let icon_small = winutil::load_icon_sized(APP_ICO, 16);
            let _ = SendMessageW(hwnd, 0x0080, WPARAM(0), LPARAM(icon_small.0 as isize)); // WM_SETICON ICON_SMALL
            let _ = SendMessageW(hwnd, 0x0080, WPARAM(1), LPARAM(icon_big.0 as isize)); // WM_SETICON ICON_BIG
            let _ = SetFocus(key);
            LRESULT(0)
        },
        // WM_CTLCOLORSTATIC (0x0138): gray text for the hint label.
        0x0138 => unsafe {
            let ctl_hwnd = HWND(lparam.0 as *mut core::ffi::c_void);
            let id = windows::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW(
                ctl_hwnd,
                WINDOW_LONG_PTR_INDEX(-12), // GWL_ID
            ) as i32;
            let hdc = HDC(wparam.0 as *mut core::ffi::c_void);
            if id == IDC_HINT {
                let _ = SetTextColor(hdc, COLORREF(0x006E6E6E));
                let _ = SetBkMode(hdc, TRANSPARENT);
            }
            LRESULT(GetSysColorBrush(COLOR_WINDOW).0 as isize)
        },
        WM_COMMAND => {
            let id = (wparam.0 & 0xffff) as i32;
            let code = ((wparam.0 >> 16) & 0xffff) as i32;
            match (id, code) {
                (IDC_PROVIDER, 1) => {
                    // CBN_SELCHANGE: load the chosen provider preset.
                    unsafe {
                        if let Ok(cb) = GetDlgItem(hwnd, IDC_PROVIDER) {
                            let sel = SendMessageW(cb, 0x0147, WPARAM(0), LPARAM(0)).0; // CB_GETCURSEL
                            if sel >= 0 {
                                apply_provider(hwnd, sel as i32);
                            }
                        }
                    }
                    LRESULT(0)
                }
                (IDC_SAVE, 0) => {
                    on_save(hwnd);
                    LRESULT(0)
                }
                (IDC_CANCEL, 0) => {
                    unsafe {
                        let _ = DestroyWindow(hwnd);
                    }
                    LRESULT(0)
                }
                _ => LRESULT(0),
            }
        }
        WM_CLOSE => {
            unsafe {
                let _ = DestroyWindow(hwnd);
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            OPEN.with(|o| *o.borrow_mut() = None);
            if STANDALONE.with(|s| *s.borrow()) {
                PostQuitMessage(0);
            }
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

fn checkbox_checked(hwnd: HWND, id: i32) -> bool {
    unsafe {
        match GetDlgItem(hwnd, id) {
            Ok(h) => {
                let r = SendMessageW(h, 0x00F0, WPARAM(0), LPARAM(0)); // BM_GETCHECK
                r.0 == 1 // BST_CHECKED
            }
            Err(_) => false,
        }
    }
}

fn on_save(hwnd: HWND) {
    let key = read_ctl(hwnd, IDC_KEY);
    let model = read_ctl(hwnd, IDC_MODEL);
    let base = read_ctl(hwnd, IDC_BASE).trim().to_string();
    let hk_mod = read_ctl(hwnd, IDC_HK_MOD);
    let hk_key = read_ctl(hwnd, IDC_HK_KEY);
    let always = checkbox_checked(hwnd, IDC_ALWAYS);
    let prompt = read_ctl(hwnd, IDC_PROMPT).trim().to_string();
    let max_secs = read_ctl(hwnd, IDC_MAXREC).trim().parse::<u32>();
    let max_secs = match max_secs {
        Ok(v) if (5..=3600).contains(&v) => v,
        _ => {
            unsafe {
                let _ = MessageBoxW(
                    hwnd,
                    PCWSTR(
                        winutil::to_wide(
                            "Max recording must be a number of seconds between 5 and 3600.",
                        )
                        .as_ptr(),
                    ),
                    w!("byok-stt"),
                    MB_OK | MB_ICONWARNING,
                );
            }
            return;
        }
    };
    if key.trim().is_empty() {
        unsafe {
            let _ = MessageBoxW(
                hwnd,
                PCWSTR(winutil::to_wide("API key must not be empty.").as_ptr()),
                w!("byok-stt"),
                MB_OK | MB_ICONWARNING,
            );
        }
        return;
    }
    if base.is_empty() || !base.to_lowercase().starts_with("http") {
        unsafe {
            let _ = MessageBoxW(
                hwnd,
                PCWSTR(
                    winutil::to_wide(
                        "API base URL must start with https:// (e.g. https://openrouter.ai/api/v1).",
                    )
                    .as_ptr(),
                ),
                w!("byok-stt"),
                MB_OK | MB_ICONWARNING,
            );
        }
        return;
    }
    if crate::hotkey::parse_modifier(&hk_mod).is_none() {
        unsafe {
            let _ = MessageBoxW(
                hwnd,
                PCWSTR(
                    winutil::to_wide(
                        "Hotkey modifier must be Ctrl, Alt, Shift or Win.",
                    )
                    .as_ptr(),
                ),
                w!("byok-stt"),
                MB_OK | MB_ICONWARNING,
            );
        }
        return;
    }
    if crate::hotkey::parse_key(&hk_key).is_none() {
        unsafe {
            let _ = MessageBoxW(
                hwnd,
                PCWSTR(
                    winutil::to_wide(
                        "Hotkey key must be Win, Space, a letter, a digit or F1-F12.",
                    )
                    .as_ptr(),
                ),
                w!("byok-stt"),
                MB_OK | MB_ICONWARNING,
            );
        }
        return;
    }
    let cfg = config::Config {
        api_key: key.trim().to_string(),
        model: {
            let m = model.trim();
            if m.is_empty() {
                config::Config::default().model
            } else {
                m.to_string()
            }
        },
        api_base: base,
        stt_prompt: prompt,
        bubble_always_visible: always,
        hotkey_modifier: hk_mod.trim().to_lowercase(),
        hotkey_key: hk_key.trim().to_lowercase(),
        max_recording_secs: max_secs,
    };
    match config::save(&cfg) {
        Ok(()) => unsafe {
            // Live-apply hotkey + bubble changes in the running tray process.
            crate::tray::notify_reload();
            let _ = DestroyWindow(hwnd);
        },
        Err(e) => unsafe {
            let _ = MessageBoxW(
                hwnd,
                PCWSTR(winutil::to_wide(&format!("Failed to save config: {e}")).as_ptr()),
                w!("byok-stt"),
                MB_OK | MB_ICONWARNING,
            );
        },
    }
}

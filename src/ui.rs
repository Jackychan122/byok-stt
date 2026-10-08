//! UI text table (English / Traditional Chinese, Hong Kong usage).
//!
//! Language is decided from `ui_lang` in config.json:
//!   "auto" (default) -> the Windows UI language; any Chinese locale gets
//!   the Traditional table. "en" / "zh-hk" force one.
//!
//! The choice is cached in an atomic and re-resolved by `reload()` when the
//! tray reloads settings, so saving a new language takes effect immediately.

use std::sync::atomic::{AtomicU8, Ordering};

use crate::config;

/// 0 = not resolved yet, 1 = English, 2 = zh-HK.
static LANG: AtomicU8 = AtomicU8::new(0);

/// Every user-visible string, one field each. The two consts below must list
/// identical fields; the compiler enforces that via the struct type.
pub struct Texts {
    // Settings window
    pub settings_title: &'static str,
    pub provider: &'static str,
    pub api_base: &'static str,
    pub api_key: &'static str,
    pub model: &'static str,
    pub prompt: &'static str,
    pub prompt_warn: &'static str,
    pub hint: &'static str,
    pub hotkey: &'static str,
    pub max_rec: &'static str,
    pub always_visible: &'static str,
    pub autostart: &'static str,
    pub s2t: &'static str,
    pub lang_label: &'static str,
    /// Display labels for the language dropdown: [auto, en, zh-hk].
    pub lang_opts: &'static [&'static str],
    pub save: &'static str,
    pub cancel: &'static str,
    // Validation / save errors
    pub err_maxrec: &'static str,
    pub err_key_empty: &'static str,
    pub err_base: &'static str,
    pub err_hkmod: &'static str,
    pub err_hkkey: &'static str,
    pub err_autostart: &'static str,
    pub err_save_cfg: &'static str,
    // Onboarding
    pub welcome_title: &'static str,
    pub welcome_msg: &'static str,
    pub all_set_title: &'static str,
    pub all_set_msg: &'static str,
    // Tray menu + balloons
    pub menu_settings: &'static str,
    pub menu_exit: &'static str,
    pub already_title: &'static str,
    pub already_msg: &'static str,
    pub mic_error: &'static str,
    pub rec_error: &'static str,
    pub empty_title: &'static str,
    pub empty_msg: &'static str,
    pub clip_error: &'static str,
    pub key_invalid_title: &'static str,
    pub key_invalid_msg: &'static str,
    pub key_missing_title: &'static str,
    pub key_missing_msg: &'static str,
    pub failed_title: &'static str,
    pub busy_title: &'static str,
    pub busy_msg: &'static str,
    // stt.rs user-facing errors ({host}/{code} are substituted at runtime)
    pub err_no_key: &'static str,
    pub err_key_rejected: &'static str,
    pub err_rate_limited: &'static str,
    pub err_unreachable: &'static str,
    pub err_dns: &'static str,
}

const EN: Texts = Texts {
    settings_title: "byok-stt Settings",
    provider: "Provider:",
    api_base: "API base URL:",
    api_key: "API key:",
    model: "Model:",
    prompt: "Prompt:",
    prompt_warn: "This provider ignores the prompt for file models (whisper etc.) — it still works with OpenAI, Groq, self-hosted endpoints and chat audio models.",
    hint: "File models (whisper / voxtral / *-transcribe / scribe / asr) use /audio/transcriptions; others use /chat/completions with inline audio.",
    hotkey: "Hotkey:",
    max_rec: "Max recording time:",
    always_visible: "Keep indicator bubble always visible",
    autostart: "Start byok-stt when Windows starts",
    s2t: "Convert output to Traditional Chinese (简→繁)",
    lang_label: "Language:",
    lang_opts: &["Auto (follow Windows)", "English", "繁體中文（香港）"],
    save: "Save",
    cancel: "Cancel",
    err_maxrec: "Max recording must be a number of seconds between 5 and 3600.",
    err_key_empty: "API key must not be empty.",
    err_base: "API base URL must start with https:// (e.g. https://openrouter.ai/api/v1).",
    err_hkmod: "Hotkey modifier must be Ctrl, Alt, Shift or Win.",
    err_hkkey: "Hotkey key must be Win, Space, a letter, a digit or F1-F12.",
    err_autostart: "Could not update the startup entry: {e}",
    err_save_cfg: "Failed to save config: {e}",
    welcome_title: "Welcome to byok-stt",
    welcome_msg: "Paste your API key, pick a model and press Save to start dictating.",
    all_set_title: "You're all set!",
    all_set_msg: "Hold {m}+{k} and speak — your words are typed automatically.",
    menu_settings: "Settings",
    menu_exit: "Exit",
    already_title: "byok-stt is already running",
    already_msg: "Hold the hotkey to dictate, or right-click the microphone icon in the system tray (it may be inside the ^ hidden-icons overflow).",
    mic_error: "Microphone error",
    rec_error: "Recording error",
    empty_title: "Transcription empty",
    empty_msg: "The model returned no text.",
    clip_error: "Clipboard error",
    key_invalid_title: "Invalid API key",
    key_invalid_msg: "The provider rejected the API key (HTTP 401). Update it in tray > Settings.",
    key_missing_title: "API key missing",
    key_missing_msg: "Add your provider API key in tray > Settings.",
    failed_title: "Transcription failed",
    busy_title: "Still transcribing",
    busy_msg: "Wait for the current transcription to finish.",
    err_no_key: "API key not configured — open Settings from the tray menu.",
    err_key_rejected: "{host} rejected the API key (HTTP {code}). Open Settings and paste a valid key.",
    err_rate_limited: "{host} rate limit reached (HTTP 429). Wait a moment and try again.",
    err_unreachable: "Cannot reach {host}: the connection timed out or was dropped — the request never reached the server. Check your internet connection, VPN or proxy; this is a network route issue, not a server outage.",
    err_dns: "Cannot resolve {host} — check your internet connection.",
};

const ZH_HK: Texts = Texts {
    settings_title: "byok-stt 設定",
    provider: "供應商：",
    api_base: "API 基底網址：",
    api_key: "API 金鑰：",
    model: "模型：",
    prompt: "提示詞：",
    prompt_warn: "此供應商嘅檔案模型（whisper 等）會忽略提示詞 — 但 OpenAI、Groq、自建端點同對話音訊模型照用得。",
    hint: "檔案模型（whisper / voxtral / *-transcribe / scribe / asr）行 /audio/transcriptions；其他行 /chat/completions 內嵌音訊。",
    hotkey: "快速鍵：",
    max_rec: "最長錄音秒數：",
    always_visible: "永遠顯示狀態氣泡",
    autostart: "Windows 啟動時自動執行 byok-stt",
    s2t: "輸出轉為繁體中文（简→繁）",
    lang_label: "介面語言：",
    lang_opts: &["自動（跟隨 Windows）", "English", "繁體中文（香港）"],
    save: "儲存",
    cancel: "取消",
    err_maxrec: "最長錄音必須係 5 至 3600 秒之間嘅數字。",
    err_key_empty: "API 金鑰唔可以留空。",
    err_base: "API 基底網址必須以 https:// 開頭（例如 https://openrouter.ai/api/v1）。",
    err_hkmod: "快速鍵修飾鍵必須係 Ctrl、Alt、Shift 或 Win。",
    err_hkkey: "快速鍵主鍵必須係 Win、Space、字母、數字或 F1-F12。",
    err_autostart: "無法更新開機啟動項目：{e}",
    err_save_cfg: "無法儲存設定：{e}",
    welcome_title: "歡迎使用 byok-stt",
    welcome_msg: "貼上你嘅 API 金鑰，揀個模型，撳「儲存」就可以開始聽寫。",
    all_set_title: "大功告成！",
    all_set_msg: "撳住 {m}+{k} 開口講 — 你講嘅嘢會自動打晒出嚟。",
    menu_settings: "設定",
    menu_exit: "結束",
    already_title: "byok-stt 已在執行中",
    already_msg: "撳住快速鍵就可以聽寫；或者用右鍵撳系統匣嘅咪圖示（可能收埋喺 ^ 隱藏圖示入面）。",
    mic_error: "麥克風錯誤",
    rec_error: "錄音錯誤",
    empty_title: "轉寫結果為空",
    empty_msg: "模型冇回傳任何文字。",
    clip_error: "剪貼簿錯誤",
    key_invalid_title: "API 金鑰無效",
    key_invalid_msg: "供應商拒絕咗呢條 API 金鑰（HTTP 401）。請喺托盤 > 設定更新。",
    key_missing_title: "缺少 API 金鑰",
    key_missing_msg: "請喺托盤 > 設定加入你嘅供應商 API 金鑰。",
    failed_title: "轉寫失敗",
    busy_title: "轉寫中",
    busy_msg: "請等目前嘅轉寫完成。",
    err_no_key: "未設定 API 金鑰 — 請由托盤選單開啟設定。",
    err_key_rejected: "{host} 拒絕咗 API 金鑰（HTTP {code}）。請開啟設定貼上有效金鑰。",
    err_rate_limited: "{host} 已達速率限制（HTTP 429）。請稍等一陣再試。",
    err_unreachable: "無法連線至 {host}：連線逾時或中斷 — 請求未送達伺服器。請檢查網絡、VPN 或代理；呢個係網絡路由問題，唔係伺服器故障。",
    err_dns: "無法解析 {host} — 請檢查網絡連線。",
};

/// Config values for the language dropdown, index-aligned with lang_opts.
pub const LANG_VALUES: [&str; 3] = ["auto", "en", "zh-hk"];

fn detect() -> u8 {
    let cfg = config::load();
    let v = match cfg.ui_lang.as_str() {
        "en" => 1,
        "zh-hk" => 2,
        _ => {
            // LANG_CHINESE = 0x04; every Chinese locale (zh-HK/TW/CN/SG/MO)
            // gets the Traditional table. Override with ui_lang "en" if needed.
            let id = unsafe { windows::Win32::Globalization::GetUserDefaultUILanguage() };
            if id & 0xFF == 0x04 {
                2
            } else {
                1
            }
        }
    };
    crate::logging::log(&format!(
        "ui detect: ui_lang={:?} appdata={:?} -> {}",
        cfg.ui_lang,
        std::env::var("APPDATA").unwrap_or_default(),
        v
    ));
    if let Ok(s) = std::fs::read_to_string(config::config_dir().join("config.json")) {
        crate::logging::log(&format!("ui detect raw config: {s:?}"));
    } else {
        crate::logging::log("ui detect: config.json unreadable");
    }
    v
}

/// All user-visible strings in the current UI language.
pub fn t() -> &'static Texts {
    match LANG.load(Ordering::Relaxed) {
        1 => &EN,
        2 => &ZH_HK,
        _ => {
            let l = detect();
            LANG.store(l, Ordering::Relaxed);
            if l == 2 {
                &ZH_HK
            } else {
                &EN
            }
        }
    }
}

/// Drop the cached language; the next t() re-resolves it from config.
pub fn reload() {
    LANG.store(0, Ordering::Relaxed);
}

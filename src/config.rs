use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub api_key: String,
    pub model: String,
    /// OpenAI-compatible base URL, e.g. https://openrouter.ai/api/v1
    #[serde(default = "default_api_base")]
    pub api_base: String,
    /// Keep the on-screen indicator bubble visible at all times.
    #[serde(default)]
    pub bubble_always_visible: bool,
    /// Hotkey: modifier ("ctrl" | "alt" | "shift" | "win") + key
    /// ("win" | "space" | a-z | 0-9 | f1-f12). Defaults are per-OS.
    #[serde(default = "default_hotkey_modifier")]
    pub hotkey_modifier: String,
    #[serde(default = "default_hotkey_key")]
    pub hotkey_key: String,
    /// Hard limit for one recording, in seconds (clamped to 5..=3600).
    #[serde(default = "default_max_recording_secs")]
    pub max_recording_secs: u32,
    /// Optional style hint sent with every transcription: the Whisper
    /// `prompt` field on file-transcription endpoints (OpenAI/Groq pass it
    /// through; OpenRouter currently ignores it), or an instruction prefix
    /// for chat-style audio models.
    #[serde(default)]
    pub stt_prompt: String,
    /// Launch byok-stt when Windows starts (HKCU Run entry).
    #[serde(default)]
    pub start_with_windows: bool,
 }
 
 fn default_api_base() -> String {
     "https://openrouter.ai/api/v1".into()
 }

fn default_max_recording_secs() -> u32 {
    120
}
fn default_hotkey_modifier() -> String {
    // Windows: Ctrl+Win. macOS: Cmd+Space. Linux: Ctrl+Space.
    if cfg!(target_os = "macos") {
        "cmd".into()
    } else if cfg!(target_os = "linux") {
        "ctrl".into()
    } else {
        "ctrl".into()
    }
}

fn default_hotkey_key() -> String {
    if cfg!(target_os = "macos") {
        "space".into()
    } else if cfg!(target_os = "linux") {
        "space".into()
    } else {
        "win".into()
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            api_key: String::new(),
            model: "google/gemini-2.5-flash-lite".into(),
            api_base: default_api_base(),
            bubble_always_visible: false,
            hotkey_modifier: default_hotkey_modifier(),
            hotkey_key: default_hotkey_key(),
            max_recording_secs: default_max_recording_secs(),
            stt_prompt: String::new(),
            start_with_windows: false,
        }
    }
}

pub fn config_dir() -> PathBuf {
    let base = std::env::var("APPDATA").unwrap_or_else(|_| ".".into());
    let dir = PathBuf::from(base).join("byok-stt");
    let _ = fs::create_dir_all(&dir);
    dir
}

fn config_path() -> PathBuf {
    config_dir().join("config.json")
}

pub fn load() -> Config {
    fs::read_to_string(config_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(cfg: &Config) -> std::io::Result<()> {
    fs::write(config_path(), serde_json::to_string_pretty(cfg).unwrap())
}

pub fn load_indicator_pos() -> Option<(i32, i32)> {
    let s = fs::read_to_string(config_dir().join("indicator_pos.json")).ok()?;
    let v: serde_json::Value = serde_json::from_str(&s).ok()?;
    let x = v["x"].as_i64()? as i32;
    let y = v["y"].as_i64()? as i32;
    Some((x, y))
}

pub fn save_indicator_pos(x: i32, y: i32) {
    let _ = fs::write(
        config_dir().join("indicator_pos.json"),
        serde_json::json!({ "x": x, "y": y }).to_string(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_sane() {
        let c = Config::default();
        assert_eq!(c.api_base, "https://openrouter.ai/api/v1");
        assert!(!c.bubble_always_visible);
        assert_eq!(c.hotkey_modifier, "ctrl");
        assert_eq!(c.hotkey_key, if cfg!(target_os = "macos") { "space" } else { "win" });
        assert_eq!(c.max_recording_secs, 120);
    }

    #[test]
    fn missing_fields_fall_back_to_defaults() {
        // An old config.json (pre api_base/hotkey options) must still parse.
        let c: Config = serde_json::from_str(
            r#"{"api_key": "k", "model": "m"}"#,
        )
        .unwrap();
        assert_eq!(c.api_base, "https://openrouter.ai/api/v1");
        assert_eq!(c.hotkey_modifier, "ctrl");
        assert_eq!(c.max_recording_secs, 120);
    }

    #[test]
    fn round_trips_through_json() {
        let c = Config {
            api_key: "secret".into(),
            model: "whisper-1".into(),
            api_base: "https://api.groq.com/openai/v1".into(),
            bubble_always_visible: true,
            hotkey_modifier: "alt".into(),
            hotkey_key: "space".into(),
            max_recording_secs: 60,
            stt_prompt: "廣東話口語".into(),
            start_with_windows: true,
        };
        let json = serde_json::to_string(&c).unwrap();
        let back: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(back.api_key, "secret");
        assert_eq!(back.api_base, "https://api.groq.com/openai/v1");
        assert!(back.bubble_always_visible);
        assert_eq!(back.hotkey_modifier, "alt");
        assert_eq!(back.hotkey_key, "space");
        assert_eq!(back.max_recording_secs, 60);
        assert_eq!(back.stt_prompt, "廣東話口語");
        assert!(back.start_with_windows);
    }
}

use std::io::Write;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::config;

/// Log file stays open for the process lifetime. Reopening the file per call
/// is cheap in isolation, but logging runs on the UI thread — the same thread
/// that hosts the low-level keyboard hook — and open/write/close at render
/// rates can stall it past the OS hook timeout (which silently bypasses the
/// hook and leaks the swallowed Win key to the OS).
static LOG_FILE: Mutex<Option<std::fs::File>> = Mutex::new(None);

pub fn log(msg: &str) {
    let mut guard = LOG_FILE.lock().unwrap_or_else(|p| p.into_inner());
    if guard.is_none() {
        *guard = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(config::config_dir().join("log.txt"))
            .ok();
    }
    if let Some(f) = guard.as_mut() {
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let _ = writeln!(f, "[{secs}] {msg}");
    }
}

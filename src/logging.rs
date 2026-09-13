use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::config;

pub fn log(msg: &str) {
    let path = config::config_dir().join("log.txt");
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let _ = writeln!(f, "[{secs}] {msg}");
    }
}

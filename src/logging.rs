use std::io::Write;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::config;

/// Log file stays open for the process lifetime. Reopening the file per call
/// is cheap in isolation, but logging runs on the UI thread — the same thread
/// that hosts the low-level keyboard hook — and open/write/close at render
/// rates can stall it past the OS hook timeout (which silently bypasses the
/// hook and leaks the swallowed Win key to the OS).
static LOG_FILE: Mutex<Option<std::fs::File>> = Mutex::new(None);

/// Cap log.txt so years of sessions cannot grow it unbounded.
const MAX_LOG_BYTES: u64 = 1024 * 1024;
/// Re-check the size every N writes (checking metadata is one syscall).
const SIZE_CHECK_INTERVAL: u32 = 2048;
static WRITES: AtomicU32 = AtomicU32::new(0);

fn truncate_if_huge(f: &mut std::fs::File) {
    let huge = f
        .metadata()
        .map(|m| m.len() > MAX_LOG_BYTES)
        .unwrap_or(false);
    if huge {
        // Keep the newest ~64 KB, drop everything before it.
        let keep = 64 * 1024;
        let len = f.metadata().map(|m| m.len()).unwrap_or(0);
        if len > keep {
            use std::io::{Read, Seek, SeekFrom};
            let _ = f.seek(SeekFrom::Start(len - keep));
            let mut tail = Vec::with_capacity(keep as usize);
            let _ = f.read_to_end(&mut tail);
            let _ = f.set_len(0);
            let _ = f.seek(SeekFrom::Start(0));
            let _ = f.write_all(&tail);
        } else {
            let _ = f.set_len(0);
        }
    }
}

pub fn log(msg: &str) {
    let mut guard = LOG_FILE.lock().unwrap_or_else(|p| p.into_inner());
    if guard.is_none() {
        *guard = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(config::config_dir().join("log.txt"))
            .ok();
        if let Some(f) = guard.as_mut() {
            truncate_if_huge(f);
            WRITES.store(0, Ordering::Relaxed);
        }
    }
    if let Some(f) = guard.as_mut() {
        let n = WRITES.fetch_add(1, Ordering::Relaxed);
        if n.is_multiple_of(SIZE_CHECK_INTERVAL) {
            truncate_if_huge(f);
        }
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let _ = writeln!(f, "[{secs}] {msg}");
    }
}

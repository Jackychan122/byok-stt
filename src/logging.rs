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

fn truncate_if_huge() {
    // The long-lived log handle is append-only (FILE_APPEND_DATA), which
    // grants neither read nor set_len access — running the tail-keep on it
    // is a silent no-op, so log.txt grows forever. Use a temporary
    // read+write handle; the append handle keeps writing at the (new,
    // shorter) end of file.
    let path = config::config_dir().join("log.txt");
    let Ok(mut f) = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)
    else {
        return;
    };
    let len = match f.metadata() {
        Ok(m) => m.len(),
        Err(_) => return,
    };
    if len <= MAX_LOG_BYTES {
        return;
    }
    // Keep the newest ~64 KB, drop everything before it.
    let keep = 64 * 1024;
    use std::io::{Read, Seek, SeekFrom, Write};
    if len > keep {
        if f.seek(SeekFrom::Start(len - keep)).is_err() {
            return;
        }
        let mut tail = Vec::with_capacity(keep as usize);
        if f.read_to_end(&mut tail).is_err() {
            return;
        }
        if f.set_len(0).is_err() || f.seek(SeekFrom::Start(0)).is_err() {
            return;
        }
        let _ = f.write_all(&tail);
        let _ = f.set_len(tail.len() as u64);
    } else {
        let _ = f.set_len(0);
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
        if guard.is_some() {
            truncate_if_huge();
            WRITES.store(0, Ordering::Relaxed);
        }
    }
    if let Some(f) = guard.as_mut() {
        let n = WRITES.fetch_add(1, Ordering::Relaxed);
        if n.is_multiple_of(SIZE_CHECK_INTERVAL) {
            truncate_if_huge();
        }
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let _ = writeln!(f, "[{secs}] {msg}");
    }
}

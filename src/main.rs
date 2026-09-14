// No console window in release builds — this is a tray app.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;
mod hotkey;
mod config;
mod indicator;
mod logging;
mod settings;
mod stt;
mod tray;
mod winutil;
mod zh;

use std::env;

fn main() {
    // Crisp 1:1 rendering on high-DPI displays; must precede window creation.
    unsafe {
        use windows::Win32::UI::HiDpi::{
            SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
        };
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
    let args: Vec<String> = env::args().skip(1).collect();
    match args.first().map(|s| s.as_str()) {
        Some("--test-transcribe") => {
            let wav = match args.get(1) {
                Some(p) => p,
                None => {
                    eprintln!("usage: byok-stt --test-transcribe <file.wav>");
                    std::process::exit(2);
                }
            };
            match stt::transcribe_file(wav) {
                Ok(text) => {
                    println!("{text}");
                    if let Err(e) = winutil::set_clipboard_text(&text) {
                        eprintln!("clipboard: {e}");
                        std::process::exit(1);
                    }
                }
                Err(e) => {
                    eprintln!("{e}");
                    std::process::exit(1);
                }
            }
        }
        Some("--test-paste") => {
            let text = match args.get(1) {
                Some(t) => t,
                None => {
                    eprintln!("usage: byok-stt --test-paste <text>");
                    std::process::exit(2);
                }
            };
            // Give the user time to focus a target window.
            std::thread::sleep(std::time::Duration::from_millis(1500));
            if let Err(e) = winutil::set_clipboard_text(text) {
                eprintln!("clipboard: {e}");
                std::process::exit(1);
            }
            winutil::send_paste();
        }
        Some("--settings") => settings::run_standalone(),
        _ => tray::run(),
    }
}

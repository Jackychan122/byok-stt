use std::sync::atomic::{AtomicUsize, Ordering};
use std::cell::Cell;

use windows::core::w;
use windows::Win32::Foundation::{
    COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM,
};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreateCompatibleDC, CreateDIBSection, CreateSolidBrush, DeleteObject, Ellipse,
    EndPaint, GetDC, ReleaseDC, SelectObject, DIB_RGB_COLORS,
    BITMAPINFO, BITMAPINFOHEADER, BI_RGB, BLENDFUNCTION, HDC, PAINTSTRUCT, AC_SRC_ALPHA,
    AC_SRC_OVER,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::GetDpiForSystem;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetCapture, ReleaseCapture, SetCapture, SetFocus,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu,
    GetCursorPos, GetSystemMetrics, GetWindowRect, PostMessageW, RegisterClassW,
    SetForegroundWindow, SetTimer, SetWindowPos, ShowWindow,
    TrackPopupMenu, UpdateLayeredWindow, HMENU, MENU_ITEM_FLAGS,
    SM_CXSCREEN, SM_CYSCREEN, SHOW_WINDOW_CMD, SW_HIDE, SW_SHOWNOACTIVATE, SWP_NOACTIVATE,
    SWP_NOSIZE, SWP_NOZORDER, TPM_NONOTIFY, TPM_RETURNCMD, TPM_RIGHTBUTTON,
    UPDATE_LAYERED_WINDOW_FLAGS, WINDOW_EX_STYLE, WINDOW_STYLE, WNDCLASSW,
    WM_PAINT, WM_TIMER, WS_EX_LAYERED,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};

use crate::{config, settings};

const COLOR_REC: usize = 0x00323CE8; // RGB(232,60,50) red: recording
const COLOR_BUSY: usize = 0x001EB4F0; // RGB(240,180,30) amber: transcribing
const COLOR_IDLE: usize = 0x005A5A5E; // RGB(94,94,94) gray: idle (always-on mode)
const COLOR_BG: usize = 0x002D2D2D; // RGB(45,45,48) dark ring

const WMAPP_SETTINGS: usize = 1; // popup menu item id

static IND_HWND: AtomicUsize = AtomicUsize::new(0);
static MAIN_HWND: AtomicUsize = AtomicUsize::new(0);
static STATE: AtomicUsize = AtomicUsize::new(0);
static VISIBLE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static ALWAYS: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static ANGLE: AtomicUsize = AtomicUsize::new(0);
static SIZE_PX: AtomicUsize = AtomicUsize::new(48);
static MEM_DC: AtomicUsize = AtomicUsize::new(0);
static BITS: AtomicUsize = AtomicUsize::new(0);

thread_local! {
    // (click point, window top-left) captured at left-button down; None = not dragging.
    static DRAG: Cell<Option<((i32, i32), (i32, i32))>> = const { Cell::new(None) };
    static MOVED: Cell<bool> = const { Cell::new(false) };
}

pub fn init(main: HWND) {
    MAIN_HWND.store(main.0 as usize, Ordering::Relaxed);
    unsafe {
        let hinstance = GetModuleHandleW(None).unwrap();
        let wc = WNDCLASSW {
            lpfnWndProc: Some(ind_wnd_proc),
            hInstance: HINSTANCE(hinstance.0),
            lpszClassName: w!("BYOK_STT_INDICATOR"),
            ..Default::default()
        };
        RegisterClassW(&wc);

        // Physical pixel size, DPI-scaled from a 48 logical-px design.
        let dpi = GetDpiForSystem().max(96);
        let size = (48 * dpi / 96) as i32;
        SIZE_PX.store(size as usize, Ordering::Relaxed);

        let (mut x, mut y) = match config::load_indicator_pos() {
            Some((px, py)) => (px, py),
            None => (
                GetSystemMetrics(SM_CXSCREEN) - size - 24,
                GetSystemMetrics(SM_CYSCREEN) - size - 96,
            ),
        };
        x = x.clamp(0, GetSystemMetrics(SM_CXSCREEN) - size);
        y = y.clamp(0, GetSystemMetrics(SM_CYSCREEN) - size);

        let ex = WINDOW_EX_STYLE(
            WS_EX_TOPMOST.0 | WS_EX_TOOLWINDOW.0 | WS_EX_NOACTIVATE.0 | WS_EX_LAYERED.0,
        );
        let hwnd = CreateWindowExW(
            ex,
            w!("BYOK_STT_INDICATOR"),
            w!(""),
            WINDOW_STYLE(WS_POPUP.0),
            x,
            y,
            size,
            size,
            HWND::default(),
            HMENU::default(),
            HINSTANCE(hinstance.0),
            None,
        )
        .expect("indicator window");
        IND_HWND.store(hwnd.0 as usize, Ordering::Relaxed);


        // Persistent ARGB buffer for UpdateLayeredWindow.
        let hdc_screen = GetDC(None);
        let hdc_mem = CreateCompatibleDC(hdc_screen);
        let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: size,
                biHeight: -size, // top-down rows
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let hbm = CreateDIBSection(hdc_mem, &bmi, DIB_RGB_COLORS, &mut bits, None, 0)
            .expect("CreateDIBSection");
        SelectObject(hdc_mem, hbm);
        ReleaseDC(None, hdc_screen);
        MEM_DC.store(hdc_mem.0 as usize, Ordering::Relaxed);
        BITS.store(bits as usize, Ordering::Relaxed);
        let _ = SetTimer(hwnd, 1, 33, None); // spinner animation clock
        set_always_visible(config::load().bubble_always_visible);
    }
}

pub fn show_recording() {
    show(COLOR_REC);
}

pub fn show_busy() {
    show(COLOR_BUSY);
}
/// Apply the always-visible preference: show idle bubble, or hide if idle.
pub fn set_always_visible(on: bool) {
    ALWAYS.store(on, std::sync::atomic::Ordering::Relaxed);
    if on {
        show(COLOR_IDLE);
    } else if STATE.load(Ordering::Relaxed) == COLOR_IDLE {
        hide();
    }
}

pub fn hide() {
    crate::logging::log("indicator hide() called");
    // In always-visible mode, "hide" drops back to the idle gray bubble.
    if ALWAYS.load(std::sync::atomic::Ordering::Relaxed) {
        show(COLOR_IDLE);
        return;
    }
    STATE.store(0, Ordering::Relaxed);
    VISIBLE.store(false, Ordering::Relaxed);
    let hwnd = IND_HWND.load(Ordering::Relaxed);
    if hwnd == 0 {
        return;
    }
    unsafe {
        let _ = ShowWindow(HWND(hwnd as *mut core::ffi::c_void), SW_HIDE);
    }
}

fn show(color: usize) {
    crate::logging::log(&format!("indicator show(color={color:#x})"));
    STATE.store(color, Ordering::Relaxed);
    VISIBLE.store(true, Ordering::Relaxed);
    render();
    let hwnd = IND_HWND.load(Ordering::Relaxed);
    if hwnd == 0 {
        return;
    }
    unsafe {
        let _ = ShowWindow(
            HWND(hwnd as *mut core::ffi::c_void),
            SHOW_WINDOW_CMD(SW_SHOWNOACTIVATE.0),
        );
    }
}

unsafe extern "system" fn ind_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg != WM_TIMER && msg != WM_PAINT {
        crate::logging::log(&format!("indicator msg: 0x{msg:04X}"));
    }
    // Spinner animation clock (only spins while the busy bubble is shown).
    if msg == 0x0113 {
        if VISIBLE.load(Ordering::Relaxed) && STATE.load(Ordering::Relaxed) == COLOR_BUSY {
            ANGLE.store((ANGLE.load(Ordering::Relaxed) + 12) % 360, Ordering::Relaxed);
            render();
        }
        return LRESULT(0);
    }
    if msg == 0x000F {
        // WM_PAINT: GDI fallback (ULW normally owns rendering).
        let mut ps: PAINTSTRUCT = Default::default();
        let hdc = BeginPaint(hwnd, &mut ps);
        let ring = CreateSolidBrush(COLORREF(COLOR_BG as u32));
        let dot = CreateSolidBrush(COLORREF(STATE.load(Ordering::Relaxed) as u32));
        let s = SIZE_PX.load(Ordering::Relaxed) as i32;
        let old = SelectObject(hdc, ring);
        let _ = Ellipse(hdc, 0, 0, s, s);
        SelectObject(hdc, dot);
        let _ = Ellipse(hdc, s * 3 / 20, s * 3 / 20, s * 17 / 20, s * 17 / 20);
        SelectObject(hdc, old);
        let _ = DeleteObject(ring);
        let _ = DeleteObject(dot);
        let _ = EndPaint(hwnd, &ps);
        return LRESULT(0);
    }
    if msg == 0x0201 {
        // WM_LBUTTONDOWN: begin potential drag; marker proves delivery.
        let _ = std::fs::write(config::config_dir().join("click_marker.txt"), "clicked");
        let mut pt = POINT::default();
        let _ = GetCursorPos(&mut pt);
        let mut rect = RECT::default();
        let _ = GetWindowRect(hwnd, &mut rect);
        crate::logging::log(&format!(
            "indicator LBUTTONDOWN at cursor ({}, {}) window ({},{})",
            pt.x, pt.y, rect.left, rect.top
        ));
        DRAG.with(|d| d.set(Some(((pt.x, pt.y), (rect.left, rect.top)))));
        MOVED.with(|m| m.set(false));
        SetCapture(hwnd);
        return LRESULT(0);
    }
    if msg == 0x0200 {
        // WM_MOUSEMOVE while capturing: drag the bubble.
        unsafe {
            if GetCapture().0 as usize != IND_HWND.load(Ordering::Relaxed) {
                return DefWindowProcW(hwnd, msg, wparam, lparam);
            }
        }
        let mut pt = POINT::default();
        let _ = GetCursorPos(&mut pt);
        DRAG.with(|d| {
            if let Some(((sx, sy), (wx, wy))) = d.get() {
                let dx = pt.x - sx;
                let dy = pt.y - sy;
                if dx.abs() > 3 || dy.abs() > 3 {
                    MOVED.with(|m| m.set(true));
                }
                unsafe {
                    let _ = SetWindowPos(
                        hwnd,
                        None,
                        wx + dx,
                        wy + dy,
                        0,
                        0,
                        SWP_NOACTIVATE | SWP_NOSIZE | SWP_NOZORDER,
                    );
                }
            }
        });
        return LRESULT(0);
    }
    if msg == 0x0202 {
        // WM_LBUTTONUP: while recording, a click stops the session; otherwise
        // a plain click (no drag) opens Settings.
        unsafe {
            let _ = ReleaseCapture();
        }
        let moved = MOVED.with(|m| m.get());
        DRAG.with(|d| d.set(None));
        if !moved {
            if STATE.load(Ordering::Relaxed) == COLOR_REC {
                // Ask the main window to stop the current recording.
                let main = MAIN_HWND.load(Ordering::Relaxed);
                if main != 0 {
                    unsafe {
                        let _ = PostMessageW(
                            HWND(main as *mut core::ffi::c_void),
                            0x8002, // WM_APP+2 = WMAPP_STOP
                            WPARAM(0),
                            LPARAM(0),
                        );
                    }
                }
            } else {
                // Idle: left click starts recording, same as the tray icon
                // left click. (Busy state is ignored by on_start.)
                let main = MAIN_HWND.load(Ordering::Relaxed);
                if main != 0 {
                    unsafe {
                        let _ = PostMessageW(
                            HWND(main as *mut core::ffi::c_void),
                            0x8001, // WM_APP+1 = WMAPP_START
                            WPARAM(0),
                            LPARAM(0),
                        );
                    }
                }
            }
        } else {
            save_pos(hwnd);
        }
        return LRESULT(0);
    }
    if msg == 0x0205 {
        // WM_RBUTTONUP: popup menu with Settings.
        open_menu(hwnd);
        return LRESULT(0);
    }
    if msg == 0x0232 {
        // WM_EXITSIZEMOVE: persist position.
        save_pos(hwnd);
        return LRESULT(0);
    }
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

fn main_window() -> HWND {
    HWND(MAIN_HWND.load(Ordering::Relaxed) as *mut core::ffi::c_void)
}

/// Right-click popup menu: Settings.
fn open_menu(hwnd: HWND) {
    unsafe {
        let mut pt = POINT::default();
        let _ = GetCursorPos(&mut pt);
        let hmenu = CreatePopupMenu().expect("CreatePopupMenu");
        let _ = AppendMenuW(
            hmenu,
            MENU_ITEM_FLAGS(MF_STRING),
            WMAPP_SETTINGS,
            w!("Settings"),
        );
        let _ = SetFocus(hwnd);
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
        if cmd.0 as usize == WMAPP_SETTINGS {
            settings::open(main_window());
        }
        let _ = PostMessageW(hwnd, 0, WPARAM(0), LPARAM(0)); // WM_NULL, dismiss quirk
    }
}

const MF_STRING: u32 = 0x0000_0000;

fn save_pos(hwnd: HWND) {
    unsafe {
        let mut rect = RECT::default();
        if GetWindowRect(hwnd, &mut rect).is_ok() {
            config::save_indicator_pos(rect.left, rect.top);
        }
    }
}

/// Anti-aliased sample of the indicator at normalized coords ([-1,1]).
/// Returns (r, g, b, a): color 0..=255, alpha 0..=255.
fn sample(u: f32, v: f32, state: usize, angle: f32) -> (f32, f32, f32, f32) {
    let r = (u * u + v * v).sqrt();
    if state == COLOR_BUSY {
        // Spinning arc: ring band with comet fade over a 270-degree sweep.
        if (0.70..0.97).contains(&r) {
            let mut deg = v.atan2(u).to_degrees();
            if deg < 0.0 {
                deg += 360.0;
            }
            let rel = (deg - angle + 360.0) % 360.0;
            if rel <= 270.0 {
                let fade = 0.25 + 0.75 * (rel / 270.0);
                return (240.0, 180.0, 30.0, 255.0 * fade);
            }
        }
        return (0.0, 0.0, 0.0, 0.0);
    }
    if state == COLOR_IDLE {
        // Idle: gray dot inside a dark ring — visible but clearly not recording.
        if r <= 0.64 {
            return (94.0, 92.0, 92.0, 255.0);
        }
        if (0.74..0.97).contains(&r) {
            return (45.0, 45.0, 48.0, 255.0);
        }
        return (0.0, 0.0, 0.0, 0.0);
    }
    // Recording: red dot inside a dark ring.
    if r <= 0.64 {
        return (232.0, 60.0, 50.0, 255.0);
    }
    if (0.74..0.97).contains(&r) {
        return (45.0, 45.0, 48.0, 255.0);
    }
    (0.0, 0.0, 0.0, 0.0)
}

/// Redraw the layered window from a supersampled anti-aliased buffer.
fn render() {
    unsafe {
        let bits = BITS.load(Ordering::Relaxed);
        let hdc_mem = MEM_DC.load(Ordering::Relaxed);
        let hwnd = IND_HWND.load(Ordering::Relaxed);
        if bits == 0 || hdc_mem == 0 || hwnd == 0 {
            return;
        }
        let s = SIZE_PX.load(Ordering::Relaxed);
        let state = STATE.load(Ordering::Relaxed);
        let angle = ANGLE.load(Ordering::Relaxed) as f32;
        let sf = s as f32;
        let ss = 3.0_f32; // 3x3 supersampling

        let buf = bits as *mut u8;
        for py in 0..s {
            for px in 0..s {
                let (mut r, mut g, mut b, mut a) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
                for sy in 0..3 {
                    for sx in 0..3 {
                        let u = ((px as f32 + (sx as f32 + 0.5) / ss) / sf) * 2.0 - 1.0;
                        let v = ((py as f32 + (sy as f32 + 0.5) / ss) / sf) * 2.0 - 1.0;
                        let (sr, sg, sb, sa) = sample(u, v, state, angle);
                        r += sr;
                        g += sg;
                        b += sb;
                        a += sa;
                    }
                }
                let n = ss * ss;
                let a = (a / n / 255.0).clamp(0.0, 1.0);
                let idx = (py * s + px) * 4;
                // premultiplied BGRA
                *buf.add(idx) = ((b / n) * a) as u8;
                *buf.add(idx + 1) = ((g / n) * a) as u8;
                *buf.add(idx + 2) = ((r / n) * a) as u8;
                *buf.add(idx + 3) = (a * 255.0) as u8;
            }
        }

        let hdc_screen = GetDC(None);
        let size = SIZE {
            cx: s as i32,
            cy: s as i32,
        };
        let src = POINT { x: 0, y: 0 };
        let blend = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: AC_SRC_ALPHA as u8,
        };
        let ok = UpdateLayeredWindow(
            HWND(hwnd as *mut core::ffi::c_void),
            hdc_screen,
            None, // keep current position
            Some(&size),
            HDC(hdc_mem as *mut core::ffi::c_void),
            Some(&src),
            COLORREF(0),
            Some(&blend),
            UPDATE_LAYERED_WINDOW_FLAGS(2), // ULW_ALPHA
        );
        ReleaseDC(None, hdc_screen);
        crate::logging::log(&format!(
            "indicator render: state={} size={} ulw_ok={}",
            if state == COLOR_BUSY {
                "busy"
            } else if state == COLOR_IDLE {
                "idle"
            } else {
                "rec"
            },
            s,
            ok.is_ok()
        ));
    }
}

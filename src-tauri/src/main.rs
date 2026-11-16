// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    sync::{
        atomic::{AtomicIsize, Ordering},
        Mutex,
    },
    thread,
    time::Duration,
};
use tauri::{window::Color, Manager};

#[cfg(target_os = "windows")]
use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    Graphics::Dwm::{
        DwmExtendFrameIntoClientArea, DwmSetWindowAttribute, DWMNCRP_DISABLED,
        DWMWA_NCRENDERING_POLICY,
    },
    UI::{
        Controls::MARGINS,
        WindowsAndMessaging::{
            CallWindowProcW, DefWindowProcW, GetSystemMetrics, GetWindowLongW, SetWindowLongPtrW,
            SetWindowLongW, SetWindowPos, GWLP_WNDPROC, GWL_EXSTYLE, GWL_STYLE, SM_CXSCREEN,
            SM_CYSCREEN, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOZORDER, WM_ERASEBKGND,
            WM_MOUSEACTIVATE, WM_NCACTIVATE, WM_NCCALCSIZE, WM_NCPAINT, WNDPROC, WS_BORDER,
            WS_CAPTION, WS_DLGFRAME, WS_EX_APPWINDOW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
            WS_POPUP, WS_SYSMENU, WS_THICKFRAME,
        },
    },
};

#[cfg(target_os = "windows")]
static PREV_WND_PROC: AtomicIsize = AtomicIsize::new(0);

#[cfg(target_os = "windows")]
unsafe extern "system" fn custom_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        // Prevent Windows from drawing any non-client title bar / caption / borders
        WM_NCPAINT => LRESULT(0),

        // Prevent DefWindowProc from painting the active or inactive caption gradient
        WM_NCACTIVATE => LRESULT(1),

        // Prevent background erasure artifacts
        WM_ERASEBKGND => LRESULT(1),

        // Prevent window activation on mouse click so it acts like a background tool (never steals focus)
        WM_MOUSEACTIVATE => LRESULT(3), // MA_NOACTIVATE: process click without activating window

        // Ensure the client area covers the entire window rectangle with 0 non-client margins
        WM_NCCALCSIZE => {
            if wparam.0 != 0 {
                LRESULT(0)
            } else {
                let prev = PREV_WND_PROC.load(Ordering::Relaxed);
                if prev != 0 {
                    let prev_fn: WNDPROC = std::mem::transmute(prev);
                    CallWindowProcW(prev_fn, hwnd, msg, wparam, lparam)
                } else {
                    DefWindowProcW(hwnd, msg, wparam, lparam)
                }
            }
        }

        _ => {
            let prev = PREV_WND_PROC.load(Ordering::Relaxed);
            if prev != 0 {
                let prev_fn: WNDPROC = std::mem::transmute(prev);
                CallWindowProcW(prev_fn, hwnd, msg, wparam, lparam)
            } else {
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
        }
    }
}

#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub struct InteractiveRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

static INTERACTIVE_REGIONS: Mutex<Vec<InteractiveRect>> = Mutex::new(Vec::new());

#[tauri::command]
fn update_interactive_regions(regions: Vec<InteractiveRect>) {
    if let Ok(mut lock) = INTERACTIVE_REGIONS.lock() {
        *lock = regions;
    }
}

#[tauri::command]
fn set_click_through(window: tauri::WebviewWindow, ignore: bool) {
    let _ = window.set_ignore_cursor_events(ignore);
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            set_click_through,
            update_interactive_regions
        ])
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_background_color(Some(Color(0, 0, 0, 0)));
                let _ = window.set_ignore_cursor_events(true);
                let _ = window.set_decorations(false);
                let _ = window.set_shadow(false);

                #[cfg(target_os = "windows")]
                if let Ok(raw_hwnd) = window.hwnd() {
                    let hwnd = HWND(raw_hwnd.0 as isize);
                    unsafe {
                        // 1. Strip all caption, title bar, frame, and border styles; enforce pure WS_POPUP
                        let style = GetWindowLongW(hwnd, GWL_STYLE);
                        let clean_style = (style
                            & !(WS_CAPTION.0 as i32
                                | WS_THICKFRAME.0 as i32
                                | WS_BORDER.0 as i32
                                | WS_DLGFRAME.0 as i32
                                | WS_SYSMENU.0 as i32))
                            | WS_POPUP.0 as i32;
                        SetWindowLongW(hwnd, GWL_STYLE, clean_style);

                        // 2. Extended styles: WS_EX_TOOLWINDOW (hides from Alt+Tab and taskbar)
                        // and WS_EX_NOACTIVATE (never steals focus, prevents Windows Focus Mode / Focus Assist)
                        let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE);
                        let clean_ex_style = (ex_style & !(WS_EX_APPWINDOW.0 as i32))
                            | WS_EX_TOOLWINDOW.0 as i32
                            | WS_EX_NOACTIVATE.0 as i32;
                        SetWindowLongW(hwnd, GWL_EXSTYLE, clean_ex_style);

                        // 3. Disable DWM non-client rendering policy so DWM never paints a non-client frame
                        let policy = DWMNCRP_DISABLED;
                        let _ = DwmSetWindowAttribute(
                            hwnd,
                            DWMWA_NCRENDERING_POLICY,
                            &policy as *const _ as *const std::ffi::c_void,
                            std::mem::size_of_val(&policy) as u32,
                        );

                        // 4. Clear any extended DWM frame margins (reset to 0 on all edges)
                        let margins = MARGINS {
                            cxLeftWidth: 0,
                            cxRightWidth: 0,
                            cyTopHeight: 0,
                            cyBottomHeight: 0,
                        };
                        let _ = DwmExtendFrameIntoClientArea(hwnd, &margins);

                        // 5. Subclass the window to intercept WM_NCPAINT, WM_NCACTIVATE, WM_ERASEBKGND, WM_NCCALCSIZE, WM_MOUSEACTIVATE
                        let prev = SetWindowLongPtrW(
                            hwnd,
                            GWLP_WNDPROC,
                            custom_wnd_proc as *const () as usize as isize,
                        );
                        PREV_WND_PROC.store(prev, Ordering::Relaxed);

                        // 6. Size and position the window to seamlessly cover the monitor without OS fullscreen
                        let (x, y, w, h) = if let Ok(Some(mon)) = window.current_monitor() {
                            let p = mon.position();
                            let s = mon.size();
                            (p.x, p.y, s.width as i32, s.height as i32)
                        } else {
                            (0, 0, GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN))
                        };

                        let _ = SetWindowPos(
                            hwnd,
                            None,
                            x,
                            y,
                            w,
                            h,
                            SWP_NOACTIVATE | SWP_NOZORDER | SWP_FRAMECHANGED,
                        );
                    }
                }

                // Dynamic hit-testing thread:
                // Tracks cursor position across logical coordinates. If over an interactive region
                // (e.g. sidebar, top/bottom bar, buttons), mouse events are captured.
                // If over empty space (<main>), mouse events pass straight through to desktop/windows below.
                let win_clone = window.clone();
                thread::spawn(move || {
                    let mut is_interactive_state = false;
                    // Default to click-through on start
                    let _ = win_clone.set_ignore_cursor_events(true);

                    loop {
                        if let Ok(pos) = win_clone.cursor_position() {
                            let scale = win_clone.scale_factor().unwrap_or(1.0);
                            let logical_x = pos.x / scale;
                            let logical_y = pos.y / scale;

                            let regions = if let Ok(lock) = INTERACTIVE_REGIONS.lock() {
                                lock.clone()
                            } else {
                                Vec::new()
                            };

                            let is_inside = regions.iter().any(|r| {
                                logical_x >= r.x
                                    && logical_x <= (r.x + r.width)
                                    && logical_y >= r.y
                                    && logical_y <= (r.y + r.height)
                            });

                            if is_inside != is_interactive_state {
                                is_interactive_state = is_inside;
                                // Ignore cursor events when NOT inside interactive regions
                                let _ = win_clone.set_ignore_cursor_events(!is_inside);
                            }
                        }
                        thread::sleep(Duration::from_millis(16));
                    }
                });
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

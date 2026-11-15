// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{sync::atomic::{AtomicIsize, Ordering}, thread, time::Duration};
use tauri::{window::Color, Emitter, Manager};

#[cfg(target_os = "windows")]
use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    Graphics::Dwm::{DwmExtendFrameIntoClientArea, DwmSetWindowAttribute, DWMNCRP_DISABLED, DWMWA_NCRENDERING_POLICY},
    UI::{
        Controls::MARGINS,
        WindowsAndMessaging::{
            CallWindowProcW, DefWindowProcW, GetWindowLongW, SetWindowLongPtrW, SetWindowLongW,
            SetWindowPos, GWLP_WNDPROC, GWL_STYLE, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE,
            SWP_NOSIZE, SWP_NOZORDER, WM_ERASEBKGND, WM_NCACTIVATE, WM_NCCALCSIZE, WM_NCPAINT,
            WNDPROC, WS_BORDER, WS_CAPTION, WS_DLGFRAME, WS_POPUP, WS_SYSMENU, WS_THICKFRAME,
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

#[derive(Clone, serde::Serialize)]
struct CursorPos {
    x: f64,
    y: f64,
}

#[tauri::command]
fn set_click_through(window: tauri::WebviewWindow, ignore: bool) {
    let _ = window.set_ignore_cursor_events(ignore);
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![set_click_through])
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

                        // 2. Disable DWM non-client rendering policy so DWM never paints a non-client frame
                        let policy = DWMNCRP_DISABLED;
                        let _ = DwmSetWindowAttribute(
                            hwnd,
                            DWMWA_NCRENDERING_POLICY,
                            &policy as *const _ as *const std::ffi::c_void,
                            std::mem::size_of_val(&policy) as u32,
                        );

                        // 3. Clear any extended DWM frame margins (reset to 0 on all edges)
                        let margins = MARGINS {
                            cxLeftWidth: 0,
                            cxRightWidth: 0,
                            cyTopHeight: 0,
                            cyBottomHeight: 0,
                        };
                        let _ = DwmExtendFrameIntoClientArea(hwnd, &margins);

                        // 4. Subclass the window to intercept WM_NCPAINT and WM_NCACTIVATE,
                        // completely suppressing system caption / title bar drawing
                        let prev = SetWindowLongPtrW(
                            hwnd,
                            GWLP_WNDPROC,
                            custom_wnd_proc as *const () as usize as isize,
                        );
                        PREV_WND_PROC.store(prev, Ordering::Relaxed);

                        // 5. Notify the window manager that the frame has changed
                        let _ = SetWindowPos(
                            hwnd,
                            None,
                            0,
                            0,
                            0,
                            0,
                            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
                        );
                    }
                }

                // Poll cursor position and forward to Svelte for click-through toggle
                let win_clone = window.clone();
                thread::spawn(move || {
                    let mut last_pos = (-1.0_f64, -1.0_f64);
                    loop {
                        if let Ok(pos) = win_clone.cursor_position() {
                            if (pos.x - last_pos.0).abs() > 0.5
                                || (pos.y - last_pos.1).abs() > 0.5
                            {
                                last_pos = (pos.x, pos.y);
                                let _ =
                                    win_clone.emit("cursor-move", CursorPos { x: pos.x, y: pos.y });
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

// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{thread, time::Duration};
use tauri::{window::Color, Emitter, Manager};

#[cfg(target_os = "windows")]
use windows::Win32::{
    Foundation::HWND,
    Graphics::Dwm::DwmExtendFrameIntoClientArea,
    UI::Controls::MARGINS,
};

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

                // Extend DWM glass frame across entire window to eliminate any ghost border or titlebar
                #[cfg(target_os = "windows")]
                if let Ok(hwnd) = window.hwnd() {
                    let hwnd = HWND(hwnd.0 as isize);
                    unsafe {
                        let margins = MARGINS {
                            cxLeftWidth: -1,
                            cxRightWidth: -1,
                            cyTopHeight: -1,
                            cyBottomHeight: -1,
                        };
                        let _ = DwmExtendFrameIntoClientArea(hwnd, &margins);
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
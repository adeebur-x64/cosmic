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
    Foundation::{BOOL, HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM},
    Graphics::Dwm::{
        DwmExtendFrameIntoClientArea, DwmGetWindowAttribute, DwmSetWindowAttribute,
        DWMNCRP_DISABLED, DWMWA_EXTENDED_FRAME_BOUNDS, DWMWA_NCRENDERING_POLICY,
    },
    Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    },
    System::StationsAndDesktops::{
        OpenDesktopW, OpenWindowStationW, SetProcessWindowStation, SetThreadDesktop,
        DESKTOP_CONTROL_FLAGS,
    },
    UI::{
        Controls::MARGINS,
        Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON},
        Shell::{
            SHAppBarMessage, SHQueryUserNotificationState, APPBARDATA, ABE_BOTTOM, ABE_LEFT,
            ABE_RIGHT, ABE_TOP, ABM_NEW, ABM_QUERYPOS, ABM_REMOVE, ABM_SETPOS,
            QUNS_RUNNING_D3D_FULL_SCREEN,
        },
        WindowsAndMessaging::{
            CallWindowProcW, CreateWindowExW, DefWindowProcW, DestroyWindow, EnumWindows,
            GetAncestor, GetClassNameW, GetDesktopWindow, GetForegroundWindow,
            GetShellWindow, GetSystemMetrics, GetWindowLongW, GetWindowRect, GetWindowTextW,
            IsIconic, IsWindowVisible, IsZoomed, PostMessageW, SendNotifyMessageW, SetWindowLongPtrW,
            SetWindowLongW, SetWindowPos, ShowWindow, SystemParametersInfoW, GA_ROOT, GWLP_WNDPROC,
            GWL_EXSTYLE, GWL_STYLE, HMENU, HWND_BOTTOM, HWND_BROADCAST, HWND_NOTOPMOST,
            HWND_TOPMOST, SM_CXSCREEN, SM_CYSCREEN, SPIF_UPDATEINIFILE, SPI_GETWORKAREA,
            SPI_SETWORKAREA, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER,
            SW_MAXIMIZE, SW_RESTORE, WM_CLOSE, WM_ERASEBKGND, WM_MOUSEACTIVATE, WM_NCACTIVATE, WM_NCCALCSIZE, WM_NCPAINT,
            WM_SETTINGCHANGE, WNDPROC, WS_BORDER, WS_CAPTION, WS_CHILD, WS_DLGFRAME,
            WS_EX_APPWINDOW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
            WS_SYSMENU, WS_THICKFRAME,
        },
    },
};

#[cfg(target_os = "windows")]
static PREV_WND_PROC: AtomicIsize = AtomicIsize::new(0);
#[cfg(target_os = "windows")]
static OUR_HWND: AtomicIsize = AtomicIsize::new(0);

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

#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub struct WorkAreaRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

static INTERACTIVE_REGIONS: Mutex<Vec<InteractiveRect>> = Mutex::new(Vec::new());
static ORIGINAL_WORK_AREA: Mutex<Option<RECT>> = Mutex::new(None);
static CURRENT_WORK_AREA: Mutex<Option<RECT>> = Mutex::new(None);
static REGISTERED_APPBARS: Mutex<Vec<HWND>> = Mutex::new(Vec::new());

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

#[cfg(target_os = "windows")]
fn attach_to_default_desktop() {
    unsafe {
        if let Ok(hw) = OpenWindowStationW(windows::core::w!("WinSta0"), false, 0x02000000) {
            let _ = SetProcessWindowStation(hw);
        }
        if let Ok(hd) = OpenDesktopW(windows::core::w!("Default"), DESKTOP_CONTROL_FLAGS(0), false, 0x02000000) {
            let _ = SetThreadDesktop(hd);
        }
    }
}

#[cfg(target_os = "windows")]
unsafe fn clear_appbars() {
    attach_to_default_desktop();
    if let Ok(mut lock) = REGISTERED_APPBARS.lock() {
        for hwnd in lock.drain(..) {
            let mut abd = APPBARDATA {
                cbSize: std::mem::size_of::<APPBARDATA>() as u32,
                hWnd: hwnd,
                ..Default::default()
            };
            let _ = SHAppBarMessage(ABM_REMOVE, &mut abd);
            let _ = DestroyWindow(hwnd);
            let _ = PostMessageW(hwnd, WM_CLOSE, WPARAM(0), LPARAM(0));
        }
    }
}

#[cfg(target_os = "windows")]
unsafe fn register_edge_appbar(edge: u32, thickness: i32, screen_w: i32, screen_h: i32) -> Option<HWND> {
    if thickness <= 0 {
        return None;
    }

    attach_to_default_desktop();

    let (x, y, w, h) = match edge {
        ABE_LEFT => (0, 0, thickness, screen_h),
        ABE_TOP => (0, 0, screen_w, thickness),
        ABE_RIGHT => (screen_w - thickness, 0, thickness, screen_h),
        ABE_BOTTOM => (0, screen_h - thickness, screen_w, thickness),
        _ => return None,
    };

    let hwnd = CreateWindowExW(
        WS_EX_TOOLWINDOW,
        windows::core::w!("STATIC"),
        windows::core::w!("CosmicAppBarEdge"),
        WS_POPUP,
        x,
        y,
        w,
        h,
        HWND(0),
        HMENU(0),
        HINSTANCE(0),
        None,
    );

    if hwnd.0 == 0 {
        return None;
    }

    let mut abd = APPBARDATA {
        cbSize: std::mem::size_of::<APPBARDATA>() as u32,
        hWnd: hwnd,
        uCallbackMessage: 0x8001,
        uEdge: edge,
        rc: RECT {
            left: x,
            top: y,
            right: x + w,
            bottom: y + h,
        },
        lParam: LPARAM(0),
    };

    let _ = SHAppBarMessage(ABM_NEW, &mut abd);
    let _ = SHAppBarMessage(ABM_QUERYPOS, &mut abd);

    match edge {
        ABE_LEFT => abd.rc.right = abd.rc.left + thickness,
        ABE_TOP => abd.rc.bottom = abd.rc.top + thickness,
        ABE_RIGHT => abd.rc.left = abd.rc.right - thickness,
        ABE_BOTTOM => abd.rc.top = abd.rc.bottom - thickness,
        _ => {}
    }

    let _ = SHAppBarMessage(ABM_SETPOS, &mut abd);
    let _ = SetWindowPos(
        hwnd,
        HWND(0),
        abd.rc.left,
        abd.rc.top,
        abd.rc.right - abd.rc.left,
        abd.rc.bottom - abd.rc.top,
        SWP_NOZORDER | SWP_NOACTIVATE,
    );

    Some(hwnd)
}

#[cfg(target_os = "windows")]
unsafe fn should_confine_window(hwnd: HWND, our_hwnd: HWND) -> bool {
    if hwnd.0 == 0 || hwnd == our_hwnd {
        return false;
    }
    if hwnd == GetDesktopWindow() || hwnd == GetShellWindow() {
        return false;
    }
    if !IsWindowVisible(hwnd).as_bool() || IsIconic(hwnd).as_bool() {
        return false;
    }

    // Ignore registered appbars
    if let Ok(lock) = REGISTERED_APPBARS.lock() {
        if lock.contains(&hwnd) {
            return false;
        }
    }

    // Filter out Windows shell, taskbars, toolbars, popups, and menus
    let mut class_buf = [0u16; 256];
    let len = GetClassNameW(hwnd, &mut class_buf);
    if len > 0 {
        let class_name = String::from_utf16_lossy(&class_buf[..len as usize]);
        if class_name == "Shell_TrayWnd"
            || class_name == "Shell_SecondaryTrayWnd"
            || class_name == "Progman"
            || class_name == "WorkerW"
            || class_name == "#32768"
            || class_name == "NotifyIconOverflowWindow"
            || class_name.starts_with("Windows.UI.Core.CoreWindow")
            || class_name.contains("Tooltip")
            || class_name.contains("CosmicAppBarEdge")
        {
            return false;
        }
    }

    // Filter by window style: ignore child windows, tool windows
    let style = GetWindowLongW(hwnd, GWL_STYLE) as u32;
    let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;

    if (style & WS_CHILD.0 != 0) || (ex_style & WS_EX_TOOLWINDOW.0 != 0) {
        return false;
    }

    true
}

#[cfg(target_os = "windows")]
unsafe fn confine_window_to_rect(hwnd: HWND, target_area: &RECT, is_maximized: bool) {
    let mut win_rect = RECT::default();
    if GetWindowRect(hwnd, &mut win_rect).is_err() {
        return;
    }

    // Get true visible frame bounds (excluding invisible DWM drop shadow margins)
    let mut frame_rect = RECT::default();
    let has_frame = DwmGetWindowAttribute(
        hwnd,
        DWMWA_EXTENDED_FRAME_BOUNDS,
        &mut frame_rect as *mut RECT as *mut std::ffi::c_void,
        std::mem::size_of::<RECT>() as u32,
    )
    .is_ok();

    let (f_left, f_top, f_right, f_bottom) = if has_frame {
        (frame_rect.left, frame_rect.top, frame_rect.right, frame_rect.bottom)
    } else {
        (win_rect.left, win_rect.top, win_rect.right, win_rect.bottom)
    };

    let diff_left = f_left - win_rect.left;
    let diff_top = f_top - win_rect.top;
    let diff_right = win_rect.right - f_right;
    let diff_bottom = win_rect.bottom - f_bottom;

    let target_left = target_area.left;
    let target_top = target_area.top;
    let target_right = target_area.right;
    let target_bottom = target_area.bottom;

    if is_maximized {
        // If window is maximized, its visual bounds must match target_area
        let tolerance = 4;
        let is_misaligned = (f_left - target_left).abs() > tolerance
            || (f_top - target_top).abs() > tolerance
            || (f_right - target_right).abs() > tolerance
            || (f_bottom - target_bottom).abs() > tolerance;

        if is_misaligned {
            let new_x = target_left - diff_left;
            let new_y = target_top - diff_top;
            let new_w = (target_right - target_left) + diff_left + diff_right;
            let new_h = (target_bottom - target_top) + diff_top + diff_bottom;

            let _ = SetWindowPos(
                hwnd,
                HWND(0),
                new_x,
                new_y,
                new_w,
                new_h,
                SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
            );
        }
    } else {
        // Floating window: check if it breaches any of the 4 borders
        let w = f_right - f_left;
        let h = f_bottom - f_top;
        let max_w = target_right - target_left;
        let max_h = target_bottom - target_top;

        let clamped_w = w.min(max_w);
        let clamped_h = h.min(max_h);

        let mut clamped_x = f_left;
        let mut clamped_y = f_top;

        if clamped_x < target_left {
            clamped_x = target_left;
        }
        if clamped_y < target_top {
            clamped_y = target_top;
        }
        if clamped_x + clamped_w > target_right {
            clamped_x = target_right - clamped_w;
        }
        if clamped_y + clamped_h > target_bottom {
            clamped_y = target_bottom - clamped_h;
        }

        // If visual bounds changed, apply the new position
        if clamped_x != f_left || clamped_y != f_top || clamped_w != w || clamped_h != h {
            let new_win_x = clamped_x - diff_left;
            let new_win_y = clamped_y - diff_top;
            let new_win_w = clamped_w + diff_left + diff_right;
            let new_win_h = clamped_h + diff_top + diff_bottom;

            let _ = SetWindowPos(
                hwnd,
                HWND(0),
                new_win_x,
                new_win_y,
                new_win_w,
                new_win_h,
                SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
            );
        }
    }
}

#[cfg(target_os = "windows")]
unsafe fn confine_all_windows(target_area: &RECT) {
    unsafe extern "system" fn enum_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let (target_area, our_hwnd) = *(lparam.0 as *const (&RECT, HWND));
        if should_confine_window(hwnd, our_hwnd) {
            let is_max = IsZoomed(hwnd).as_bool();
            if is_max {
                let mut frame_rect = RECT::default();
                let has_frame = DwmGetWindowAttribute(
                    hwnd,
                    DWMWA_EXTENDED_FRAME_BOUNDS,
                    &mut frame_rect as *mut RECT as *mut std::ffi::c_void,
                    std::mem::size_of::<RECT>() as u32,
                )
                .is_ok();
                let mut win_rect = RECT::default();
                let _ = GetWindowRect(hwnd, &mut win_rect);
                let check_rect = if has_frame { frame_rect } else { win_rect };

                if (check_rect.left - target_area.left).abs() > 4
                    || (check_rect.top - target_area.top).abs() > 4
                    || (check_rect.right - target_area.right).abs() > 4
                    || (check_rect.bottom - target_area.bottom).abs() > 4
                {
                    let _ = ShowWindow(hwnd, SW_RESTORE);
                    let _ = ShowWindow(hwnd, SW_MAXIMIZE);
                }
            } else {
                confine_window_to_rect(hwnd, target_area, false);
            }
        }
        BOOL(1)
    }

    let our_hwnd = HWND(OUR_HWND.load(Ordering::Relaxed));
    let data = (target_area, our_hwnd);
    let _ = EnumWindows(Some(enum_callback), LPARAM(&data as *const _ as isize));
}

#[tauri::command]
fn set_work_area(rect: WorkAreaRect) {
    #[cfg(target_os = "windows")]
    unsafe {
        attach_to_default_desktop();

        let mut orig_lock = ORIGINAL_WORK_AREA.lock().unwrap();
        let orig = if let Some(saved) = *orig_lock {
            saved
        } else {
            let mut current_orig = RECT::default();
            if SystemParametersInfoW(
                SPI_GETWORKAREA,
                0,
                Some(&mut current_orig as *mut RECT as *mut std::ffi::c_void),
                Default::default(),
            )
            .is_ok()
            {
                *orig_lock = Some(current_orig);
                current_orig
            } else {
                let default_rect = RECT {
                    left: 0,
                    top: 0,
                    right: GetSystemMetrics(SM_CXSCREEN),
                    bottom: GetSystemMetrics(SM_CYSCREEN),
                };
                *orig_lock = Some(default_rect);
                default_rect
            }
        };

        // Intersect with original work area to strictly respect existing system taskbars
        let mut new_rect = RECT {
            left: rect.left.max(orig.left),
            top: rect.top.max(orig.top),
            right: rect.right.min(orig.right),
            bottom: rect.bottom.min(orig.bottom),
        };

        if let Ok(mut current_lock) = CURRENT_WORK_AREA.lock() {
            if let Some(cur) = *current_lock {
                if cur.left == new_rect.left
                    && cur.top == new_rect.top
                    && cur.right == new_rect.right
                    && cur.bottom == new_rect.bottom
                {
                    return;
                }
            }
            *current_lock = Some(new_rect);
        }

        // 1. Register official Windows AppBars for all 4 edges so Electron/Chromium (Discord, Antigravity IDE, Chrome, VS Code)
        // receive the native Windows Shell AppBar reservation and dynamically maximize strictly inside <main>
        clear_appbars();

        let screen_w = GetSystemMetrics(SM_CXSCREEN);
        let screen_h = GetSystemMetrics(SM_CYSCREEN);

        let left_thickness = new_rect.left;
        let top_thickness = new_rect.top;
        let right_thickness = screen_w - new_rect.right;
        let bottom_thickness = screen_h - new_rect.bottom;

        let mut appbars = Vec::new();
        if let Some(h) = register_edge_appbar(ABE_LEFT, left_thickness, screen_w, screen_h) {
            appbars.push(h);
        }
        if let Some(h) = register_edge_appbar(ABE_TOP, top_thickness, screen_w, screen_h) {
            appbars.push(h);
        }
        if let Some(h) = register_edge_appbar(ABE_RIGHT, right_thickness, screen_w, screen_h) {
            appbars.push(h);
        }
        if let Some(h) = register_edge_appbar(ABE_BOTTOM, bottom_thickness, screen_w, screen_h) {
            appbars.push(h);
        }

        if let Ok(mut lock) = REGISTERED_APPBARS.lock() {
            *lock = appbars;
        }

        // 2. Also update SPI_SETWORKAREA
        let _ = SystemParametersInfoW(
            SPI_SETWORKAREA,
            0,
            Some(&mut new_rect as *mut RECT as *mut std::ffi::c_void),
            SPIF_UPDATEINIFILE,
        );

        // 3. Broadcast setting change asynchronously
        let _ = SendNotifyMessageW(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            WPARAM(SPI_SETWORKAREA.0 as usize),
            LPARAM(0),
        );

        // 4. Immediately confine any already open floating or misaligned application windows
        confine_all_windows(&new_rect);
    }
}

#[tauri::command]
fn restore_work_area() {
    #[cfg(target_os = "windows")]
    unsafe {
        attach_to_default_desktop();
        clear_appbars();

        if let Ok(mut lock) = ORIGINAL_WORK_AREA.lock() {
            if let Some(mut orig) = lock.take() {
                let _ = SystemParametersInfoW(
                    SPI_SETWORKAREA,
                    0,
                    Some(&mut orig as *mut RECT as *mut std::ffi::c_void),
                    SPIF_UPDATEINIFILE,
                );
                let _ = SendNotifyMessageW(
                    HWND_BROADCAST,
                    WM_SETTINGCHANGE,
                    WPARAM(SPI_SETWORKAREA.0 as usize),
                    LPARAM(0),
                );
            }
        }
        if let Ok(mut current_lock) = CURRENT_WORK_AREA.lock() {
            *current_lock = None;
        }
    }
}

#[cfg(target_os = "windows")]
fn is_foreground_fullscreen(our_hwnd: HWND) -> bool {
    unsafe {
        let fg = GetForegroundWindow();
        if fg.0 == 0 || fg == our_hwnd || fg == GetDesktopWindow() || fg == GetShellWindow() {
            return false;
        }

        let root_fg = GetAncestor(fg, GA_ROOT);
        let target = if root_fg.0 != 0 { root_fg } else { fg };
        if target == our_hwnd || target == GetDesktopWindow() || target == GetShellWindow() {
            return false;
        }

        // Ignore Windows taskbar, desktop background, and internal AppBars
        let mut class_buf = [0u16; 256];
        let len = GetClassNameW(target, &mut class_buf);
        if len > 0 {
            let class_name = String::from_utf16_lossy(&class_buf[..len as usize]);
            if class_name == "Shell_TrayWnd"
                || class_name == "Shell_SecondaryTrayWnd"
                || class_name == "Progman"
                || class_name == "WorkerW"
                || class_name.contains("CosmicAppBarEdge")
            {
                return false;
            }
            if class_name.starts_with("Windows.UI.Core.CoreWindow") {
                let mut title_buf = [0u16; 256];
                let tlen = GetWindowTextW(target, &mut title_buf);
                if tlen > 0 {
                    let title = String::from_utf16_lossy(&title_buf[..tlen as usize]);
                    if title.contains("Text Input") || title.contains("Snipping") {
                        return false;
                    }
                }
            }
        }

        // 1. Check DirectX / Presentation state (Exclusive Fullscreen Games)
        if let Ok(state) = SHQueryUserNotificationState() {
            if state == QUNS_RUNNING_D3D_FULL_SCREEN {
                return true;
            }
        }

        // 2. Geometry check: Check if target window covers or exceeds the entire monitor
        // Standard maximized applications are confined within our AppBars (rcWork) and NEVER cover the monitor.
        // True fullscreen applications (YouTube video, VLC, full-screen games, F11) cover rcMonitor completely.
        let mut win_rect = RECT::default();
        if GetWindowRect(target, &mut win_rect).is_err() {
            return false;
        }

        let h_monitor = MonitorFromWindow(target, MONITOR_DEFAULTTONEAREST);
        if h_monitor.0 == 0 {
            return false;
        }

        let mut mon_info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };

        if GetMonitorInfoW(h_monitor, &mut mon_info).as_bool() {
            let m = mon_info.rcMonitor;

            let mut frame_rect = RECT::default();
            let has_frame = DwmGetWindowAttribute(
                target,
                DWMWA_EXTENDED_FRAME_BOUNDS,
                &mut frame_rect as *mut RECT as *mut std::ffi::c_void,
                std::mem::size_of::<RECT>() as u32,
            )
            .is_ok();

            let (l, t, r, b) = if has_frame {
                (frame_rect.left, frame_rect.top, frame_rect.right, frame_rect.bottom)
            } else {
                (win_rect.left, win_rect.top, win_rect.right, win_rect.bottom)
            };

            let covers = (l <= m.left + 2 && t <= m.top + 2 && r >= m.right - 2 && b >= m.bottom - 2)
                || (win_rect.left <= m.left + 2 && win_rect.top <= m.top + 2 && win_rect.right >= m.right - 2 && win_rect.bottom >= m.bottom - 2);

            if covers {
                return true;
            }
        }

        false
    }
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            set_click_through,
            update_interactive_regions,
            set_work_area,
            restore_work_area
        ])
        .on_window_event(|_window, event| {
            if let tauri::WindowEvent::Destroyed = event {
                restore_work_area();
            }
        })
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_background_color(Some(Color(0, 0, 0, 0)));
                let _ = window.set_ignore_cursor_events(true);
                let _ = window.set_decorations(false);
                let _ = window.set_shadow(false);
                let _ = window.set_always_on_top(true);

                #[cfg(target_os = "windows")]
                if let Ok(raw_hwnd) = window.hwnd() {
                    let hwnd = HWND(raw_hwnd.0 as isize);
                    OUR_HWND.store(raw_hwnd.0 as isize, Ordering::Relaxed);

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

                        // Start with HWND_TOPMOST so borders float above normal desktop windows
                        let _ = SetWindowPos(
                            hwnd,
                            HWND_TOPMOST,
                            x,
                            y,
                            w,
                            h,
                            SWP_NOACTIVATE | SWP_FRAMECHANGED,
                        );
                    }
                }

                // Dynamic background loop:
                // 1. Hit-testing: cursor over interactive region -> capture clicks; cursor over <main> -> pass clicks through.
                // 2. Fullscreen monitor: always on top unless an actual fullscreen / borderless windowed game is active.
                // 3. Active Window Confinement: keeps all application windows strictly inside <main> desktop area.
                let win_clone = window.clone();
                thread::spawn(move || {
                    #[cfg(target_os = "windows")]
                    attach_to_default_desktop();

                    let mut is_interactive_state = false;
                    let mut last_fg = HWND(0);
                    // Default to click-through on start
                    let _ = win_clone.set_ignore_cursor_events(true);

                    #[cfg(target_os = "windows")]
                    let raw_hwnd = win_clone.hwnd().ok().map(|h| HWND(h.0 as isize));

                    loop {
                        #[cfg(target_os = "windows")]
                        let has_fullscreen_app = if let Some(hwnd) = raw_hwnd {
                            is_foreground_fullscreen(hwnd)
                        } else {
                            false
                        };
                        #[cfg(not(target_os = "windows"))]
                        let has_fullscreen_app = false;

                        // 1. Cursor Hit-Testing (only when NOT in fullscreen, ensuring clicks reach the video)
                        if !has_fullscreen_app {
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
                                    let _ = win_clone.set_ignore_cursor_events(!is_inside);
                                }
                            }
                        } else if is_interactive_state {
                            is_interactive_state = false;
                            let _ = win_clone.set_ignore_cursor_events(true);
                        }

                        // 2. Fullscreen Application / Borderless Window Detection & Z-Order Management
                        #[cfg(target_os = "windows")]
                        if let Some(hwnd) = raw_hwnd {
                            let ex_style = unsafe { GetWindowLongW(hwnd, GWL_EXSTYLE) as u32 };
                            let is_actually_topmost = (ex_style & WS_EX_TOPMOST.0) != 0;
                            let fg = unsafe { GetForegroundWindow() };

                            if has_fullscreen_app {
                                if is_actually_topmost {
                                    let _ = win_clone.set_always_on_top(false);
                                    unsafe {
                                        let _ = SetWindowPos(
                                            hwnd,
                                            HWND_NOTOPMOST,
                                            0,
                                            0,
                                            0,
                                            0,
                                            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                                        );
                                        let _ = SetWindowPos(
                                            hwnd,
                                            HWND_BOTTOM,
                                            0,
                                            0,
                                            0,
                                            0,
                                            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                                        );
                                    }
                                }
                            } else {
                                // If not topmost, or if foreground window changed, reaffirm topmost so borders & corner rounders stay above
                                if !is_actually_topmost || (fg != last_fg && fg != hwnd && fg.0 != 0) {
                                    let _ = win_clone.set_always_on_top(true);
                                    unsafe {
                                        let _ = SetWindowPos(
                                            hwnd,
                                            HWND_TOPMOST,
                                            0,
                                            0,
                                            0,
                                            0,
                                            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                                        );
                                    }
                                }
                            }
                            last_fg = fg;

                            // 3. Active Window Confinement
                            if !has_fullscreen_app {
                                if let Ok(work_lock) = CURRENT_WORK_AREA.lock() {
                                    if let Some(target_area) = *work_lock {
                                        unsafe {
                                            let is_mouse_down = (GetAsyncKeyState(VK_LBUTTON.0 as i32) as u16 & 0x8000) != 0;
                                            let fg = GetForegroundWindow();
                                            if fg.0 != 0 && fg != hwnd && should_confine_window(fg, hwnd) {
                                                let is_maximized = IsZoomed(fg).as_bool();
                                                if is_maximized || !is_mouse_down {
                                                    confine_window_to_rect(fg, &target_area, is_maximized);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        thread::sleep(Duration::from_millis(25));
                    }
                });
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

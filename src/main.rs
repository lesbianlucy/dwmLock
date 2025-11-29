#![allow(unsafe_op_in_unsafe_fn)]

#[cfg(not(windows))]
compile_error!("dwmlock currently only targets Windows platforms.");

mod blur;
mod capture;
mod config;
mod keyboard;
mod monitors;
mod notifications;
mod render;
mod settings;
mod settings_dialog;
mod state;

use crate::{
    blur::blur_buffer,
    capture::{build_bitmap_info, capture_screen},
    config::{APPROVAL_CAPTION, APPROVAL_PROMPT, CLASS_NAME, TIMER_ID, TIMER_INTERVAL_MS},
    keyboard::CtrlAltDeleteHook,
    monitors::{destroy_overlays, spawn_overlays},
    notifications::dismiss_notifications,
    render::draw_overlay,
    settings::{Settings, load_settings, persist_settings},
    settings_dialog::show_settings_dialog,
    state::{AppState, app_state, arm_warning, init_state, mark_warning},
};
use std::{env, process};
use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
        Graphics::Gdi::{
            BeginPaint, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DIB_RGB_COLORS,
            DeleteDC, DeleteObject, EndPaint, InvalidateRect, PAINTSTRUCT, SRCCOPY, SelectObject,
            StretchDIBits,
        },
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::{
            CS_HREDRAW, CS_VREDRAW, ClipCursor, CreateWindowExW, DefWindowProcW, DestroyWindow,
            DispatchMessageW, GetMessageW, HMENU, HWND_TOPMOST, KillTimer, LoadCursorW, MSG,
            MessageBoxW, PostQuitMessage, RegisterClassW, SC_CLOSE, SW_SHOW, SWP_NOMOVE,
            SWP_NOSIZE, SWP_SHOWWINDOW, SetCursorPos, SetForegroundWindow, SetTimer, SetWindowPos,
            ShowCursor, ShowWindow, TranslateMessage, WINDOW_EX_STYLE, WM_ACTIVATE, WM_CHAR,
            WM_CLOSE, WM_CREATE, WM_DESTROY, WM_ERASEBKGND, WM_KEYDOWN, WM_MOUSEMOVE, WM_PAINT,
            WM_SYSCOMMAND, WM_SYSKEYDOWN, WM_TIMER, WNDCLASS_STYLES, WNDCLASSW, WS_EX_TOOLWINDOW,
            WS_EX_TOPMOST, WS_POPUP, WS_VISIBLE,
        },
    },
    core::{Result, w},
};

fn main() {
    if let Err(err) = run() {
        eprintln!("dwmlock failed: {err:?}");
        process::exit(1);
    }
}

fn run() -> Result<()> {
    unsafe {
        let mut settings = load_settings();
        if should_open_settings_ui(&settings) {
            if show_settings_dialog(&mut settings)? {
                persist_settings(&settings);
            }
        }

        if settings.dismiss_notifications_on_startup {
            dismiss_notifications();
        }

        if !confirm_lock() {
            return Ok(());
        }

        let mut captured = capture_screen()?;
        blur_buffer(
            &mut captured.pixels,
            captured.width as usize,
            captured.height as usize,
            settings.blur_radius.max(1),
        );
        let bitmap_info = build_bitmap_info(captured.width, captured.height);

        init_state(AppState {
            width: captured.width,
            height: captured.height,
            pixels: captured.pixels,
            bitmap_info,
            password: settings.password.clone(),
            input: String::new(),
            warning_since: None,
            settings,
            monitor_windows: Vec::new(),
        });

        let _ctrl_alt_delete_hook = CtrlAltDeleteHook::install()?;
        create_window_loop()?;
    }

    Ok(())
}

fn should_open_settings_ui(settings: &Settings) -> bool {
    if settings.open_settings_on_startup {
        return true;
    }

    env::args().any(|arg| {
        matches!(
            arg.as_str(),
            "--settings" | "--open-settings" | "--settings-on-startup"
        )
    })
}

unsafe fn create_window_loop() -> Result<()> {
    let instance = GetModuleHandleW(None)?;

    let class = WNDCLASSW {
        style: WNDCLASS_STYLES(CS_HREDRAW.0 | CS_VREDRAW.0),
        lpfnWndProc: Some(window_proc),
        hInstance: instance.into(),
        lpszClassName: CLASS_NAME,
        hCursor: LoadCursorW(None, windows::Win32::UI::WindowsAndMessaging::IDC_ARROW)?,
        ..Default::default()
    };

    if RegisterClassW(&class) == 0 {
        return Err(windows::core::Error::from_win32());
    }

    let state = app_state().lock().unwrap();
    let width = state.width;
    let height = state.height;
    drop(state);

    let hwnd = CreateWindowExW(
        WINDOW_EX_STYLE(WS_EX_TOPMOST.0 | WS_EX_TOOLWINDOW.0),
        CLASS_NAME,
        w!("dwmlock"),
        WS_POPUP | WS_VISIBLE,
        0,
        0,
        width,
        height,
        HWND(0),
        HMENU(0),
        instance,
        None,
    );
    if hwnd.0 == 0 {
        return Err(windows::core::Error::from_win32());
    }

    let _ = SetWindowPos(
        hwnd,
        HWND_TOPMOST,
        0,
        0,
        0,
        0,
        SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW,
    );
    let _ = ShowWindow(hwnd, SW_SHOW);
    let _ = SetForegroundWindow(hwnd);

    {
        let settings = {
            let state = app_state().lock().unwrap();
            state.settings.clone()
        };
        let overlays = spawn_overlays(instance.into(), &settings);
        let mut state = app_state().lock().unwrap();
        state.monitor_windows = overlays;
    }

    let mut message = MSG::default();
    while GetMessageW(&mut message, HWND(0), 0, 0).into() {
        let _ = TranslateMessage(&message);
        DispatchMessageW(&message);
    }

    Ok(())
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_CREATE => {
            focus_and_lock(hwnd);
            SetTimer(hwnd, TIMER_ID, TIMER_INTERVAL_MS, None);
            LRESULT(0)
        }
        WM_PAINT => {
            paint(hwnd);
            LRESULT(0)
        }
        WM_ERASEBKGND => LRESULT(1),
        WM_MOUSEMOVE => {
            mark_warning();
            focus_and_lock(hwnd);
            let _ = InvalidateRect(hwnd, None, false);
            LRESULT(0)
        }
        WM_CHAR => {
            handle_char(hwnd, wparam.0 as u32);
            LRESULT(0)
        }
        WM_SYSCOMMAND => {
            let command = (wparam.0 & 0xfff0) as u32;
            if command == SC_CLOSE as u32 {
                LRESULT(0)
            } else {
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
        }
        WM_TIMER => {
            let _ = InvalidateRect(hwnd, None, false);
            LRESULT(0)
        }
        WM_KEYDOWN | WM_SYSKEYDOWN => LRESULT(0),
        WM_CLOSE => LRESULT(0),
        WM_ACTIVATE => {
            let _ = SetForegroundWindow(hwnd);
            LRESULT(0)
        }
        WM_DESTROY => {
            {
                let mut state = app_state().lock().unwrap();
                destroy_overlays(&state.monitor_windows);
                state.monitor_windows.clear();
            }
            let _ = KillTimer(hwnd, TIMER_ID);
            release_locks();
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn paint(hwnd: HWND) {
    let mut paint_struct = PAINTSTRUCT::default();
    let hdc = BeginPaint(hwnd, &mut paint_struct);
    {
        let state = app_state().lock().unwrap();
        if !draw_buffered(hdc, &state) {
            StretchDIBits(
                hdc,
                0,
                0,
                state.width,
                state.height,
                0,
                0,
                state.width,
                state.height,
                Some(state.pixels.as_ptr() as *const _),
                &state.bitmap_info,
                DIB_RGB_COLORS,
                SRCCOPY,
            );
            draw_overlay(hdc, &state);
        }
    }
    let _ = EndPaint(hwnd, &paint_struct);
}

unsafe fn draw_buffered(hdc: windows::Win32::Graphics::Gdi::HDC, state: &AppState) -> bool {
    let buffer_dc = CreateCompatibleDC(hdc);
    if buffer_dc.0 == 0 {
        return false;
    }

    let buffer_bitmap = CreateCompatibleBitmap(hdc, state.width, state.height);
    if buffer_bitmap.0 == 0 {
        let _ = DeleteDC(buffer_dc);
        return false;
    }

    let old = SelectObject(buffer_dc, buffer_bitmap);
    StretchDIBits(
        buffer_dc,
        0,
        0,
        state.width,
        state.height,
        0,
        0,
        state.width,
        state.height,
        Some(state.pixels.as_ptr() as *const _),
        &state.bitmap_info,
        DIB_RGB_COLORS,
        SRCCOPY,
    );
    draw_overlay(buffer_dc, state);
    let _ = BitBlt(
        hdc,
        0,
        0,
        state.width,
        state.height,
        buffer_dc,
        0,
        0,
        SRCCOPY,
    );
    let _ = SelectObject(buffer_dc, old);
    let _ = DeleteObject(buffer_bitmap);
    let _ = DeleteDC(buffer_dc);
    true
}

fn handle_char(hwnd: HWND, char_code: u32) {
    let mut state = app_state().lock().unwrap();
    match char_code {
        0x08 => {
            state.input.pop();
        }
        0x0D => {
            if state.input == state.password {
                drop(state);
                unsafe {
                    release_locks();
                    let _ = DestroyWindow(hwnd);
                }
                return;
            } else {
                state.input.clear();
                arm_warning(&mut state);
            }
        }
        0x1B => {}
        ch => {
            if let Some(c) = char::from_u32(ch) {
                if c.is_ascii() && !c.is_ascii_control() {
                    state.input.push(c);
                }
            }
        }
    }

    drop(state);
    unsafe {
        let _ = InvalidateRect(hwnd, None, false);
    }
}

unsafe fn focus_and_lock(hwnd: HWND) {
    let _ = SetForegroundWindow(hwnd);
    ShowCursor(false);

    let state = app_state().lock().unwrap();
    let center_x = state.width / 2;
    let center_y = state.height / 2;
    drop(state);

    let clip_rect = RECT {
        left: center_x,
        top: center_y,
        right: center_x + 1,
        bottom: center_y + 1,
    };
    let _ = ClipCursor(Some(&clip_rect as *const RECT));
    let _ = SetCursorPos(center_x, center_y);
}

unsafe fn release_locks() {
    let _ = ClipCursor(None);
    ShowCursor(true);
}

unsafe fn confirm_lock() -> bool {
    use windows::Win32::UI::WindowsAndMessaging::{IDYES, MB_ICONWARNING, MB_YESNO};

    let response = MessageBoxW(
        HWND(0),
        APPROVAL_PROMPT,
        APPROVAL_CAPTION,
        MB_ICONWARNING | MB_YESNO,
    );
    response == IDYES
}

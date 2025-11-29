#![allow(unsafe_op_in_unsafe_fn)]

#[cfg(not(windows))]
compile_error!("lockwin currently only targets Windows platforms.");

mod blur;
mod capture;
mod config;
mod game;
mod monitors;
mod render;
mod settings;
mod state;

use crate::{
    blur::blur_buffer,
    capture::{build_bitmap_info, capture_screen},
    config::{APPROVAL_CAPTION, APPROVAL_PROMPT, BLUR_RADIUS, CLASS_NAME, TIMER_ID, TIMER_INTERVAL_MS},
    game::MiniGameState,
    monitors::{destroy_overlays, spawn_overlays},
    render::draw_overlay,
    settings::load_settings,
    state::{app_state, arm_warning, init_state, mark_warning, AppState},
};
use std::process;
use windows::{
    core::{w, Result},
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
        Graphics::Gdi::{
            BeginPaint, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, EndPaint,
            InvalidateRect, SelectObject, StretchDIBits, DIB_RGB_COLORS, PAINTSTRUCT, SRCCOPY,
        },
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            Input::KeyboardAndMouse::VK_ESCAPE,
            WindowsAndMessaging::{
                ClipCursor, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW, KillTimer,
                LoadCursorW, MessageBoxW, PostQuitMessage, RegisterClassW, SetCursorPos, SetForegroundWindow, SetTimer,
                SetWindowPos, ShowCursor, ShowWindow, TranslateMessage, CS_HREDRAW, CS_VREDRAW, HMENU, MSG, SC_CLOSE,
                SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW, SW_SHOW, WINDOW_EX_STYLE, WNDCLASSW, WM_ACTIVATE, WM_CHAR,
                WM_CLOSE, WM_CREATE, WM_DESTROY, WM_ERASEBKGND, WM_KEYDOWN, WM_MOUSEMOVE, WM_PAINT, WM_SYSCOMMAND,
                WM_SYSKEYDOWN, WM_TIMER, WNDCLASS_STYLES, HWND_TOPMOST, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
                WS_VISIBLE,
            },
        },
    },
};

fn main() {
    if let Err(err) = run() {
        eprintln!("lockwin failed: {err:?}");
        process::exit(1);
    }
}

fn run() -> Result<()> {
    unsafe {
        if !confirm_lock() {
            return Ok(());
        }

        let settings = load_settings();
        let mut captured = capture_screen()?;
        let autostart_game = settings.minigame_autostart;
        blur_buffer(
            &mut captured.pixels,
            captured.width as usize,
            captured.height as usize,
            BLUR_RADIUS,
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
            game: MiniGameState::new(autostart_game),
        });

        create_window_loop()?;
    }

    Ok(())
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
        w!("lockwin"),
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

unsafe extern "system" fn window_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
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
            {
                let mut state = app_state().lock().unwrap();
                state.game.tick();
            }
            let _ = InvalidateRect(hwnd, None, false);
            LRESULT(0)
        }
        WM_KEYDOWN | WM_SYSKEYDOWN => {
            handle_keydown(hwnd, wparam.0 as u32);
            LRESULT(0)
        }
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
    if let Some(ch) = char::from_u32(char_code) {
        let consumed = state.game.handle_input(ch);
        if consumed {
            drop(state);
            unsafe {
                let _ = InvalidateRect(hwnd, None, false);
            }
            return;
        }
    }

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

fn handle_keydown(hwnd: HWND, vk: u32) {
    const KEY_G: u32 = b'G' as u32;

    let mut state = app_state().lock().unwrap();
    match vk {
        KEY_G => {
            state.game.toggle();
            drop(state);
            unsafe {
                let _ = InvalidateRect(hwnd, None, false);
            }
        }
        k if k == VK_ESCAPE.0 as u32 => {
            if state.game.active {
                state.game.stop();
                drop(state);
                unsafe {
                    let _ = InvalidateRect(hwnd, None, false);
                }
            }
        }
        _ => {}
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

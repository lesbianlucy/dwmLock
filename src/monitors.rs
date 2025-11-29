use std::{collections::HashSet, mem::size_of, ptr::null_mut};
use windows::{
    core::{w, PCWSTR},
    Win32::{
        Foundation::{BOOL, HWND, LPARAM, RECT},
        Graphics::Gdi::{
            CreateSolidBrush, EnumDisplayMonitors, GetMonitorInfoW, HBRUSH, HDC, MONITORINFOEXW,
        },
        UI::WindowsAndMessaging::{
            CreateWindowExW, DefWindowProcW, DestroyWindow, RegisterClassW, ShowWindow, CS_HREDRAW, CS_VREDRAW, HMENU,
            HMONITOR, SW_SHOW, WINDOW_EX_STYLE, WNDCLASSW, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP, WS_VISIBLE,
        },
    },
};

use crate::settings::Settings;

const BLANK_CLASS: PCWSTR = w!("LockWinBlankWindow");

#[derive(Clone)]
struct MonitorDescriptor {
    name: String,
    rect: RECT,
}

pub fn spawn_overlays(instance: windows::Win32::Foundation::HINSTANCE, settings: &Settings) -> Vec<HWND> {
    if settings.disable_monitors.is_empty() {
        return Vec::new();
    }

    unsafe {
        register_blank_class(instance);
    }

    let disable_set: HashSet<String> = settings
        .disable_monitors
        .iter()
        .map(|s| canonicalize_name(s))
        .collect();

    let monitors = enumerate_monitors();
    let mut overlays = Vec::new();
    for monitor in monitors {
        let canonical = canonicalize_name(&monitor.name);
        if disable_set.contains(&canonical) {
            if let Some(hwnd) = unsafe { create_blank_window(instance, &monitor.rect) } {
                overlays.push(hwnd);
            }
        }
    }
    overlays
}

pub fn destroy_overlays(handles: &[HWND]) {
    for hwnd in handles {
        if hwnd.0 != 0 {
            unsafe {
                let _ = DestroyWindow(*hwnd);
            }
        }
    }
}

unsafe fn register_blank_class(instance: windows::Win32::Foundation::HINSTANCE) {
    use std::sync::Once;
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let brush = CreateSolidBrush(windows::Win32::Graphics::Gdi::COLORREF(0x00000000));
        let class = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(DefWindowProcW),
            hInstance: instance,
            lpszClassName: BLANK_CLASS,
            hbrBackground: HBRUSH(brush.0),
            ..Default::default()
        };
        let _ = RegisterClassW(&class);
    });
}

unsafe fn create_blank_window(
    instance: windows::Win32::Foundation::HINSTANCE,
    rect: &RECT,
) -> Option<HWND> {
    let hwnd = CreateWindowExW(
        WINDOW_EX_STYLE(WS_EX_TOPMOST.0 | WS_EX_TOOLWINDOW.0),
        BLANK_CLASS,
        PCWSTR::null(),
        WS_POPUP | WS_VISIBLE,
        rect.left,
        rect.top,
        rect.right - rect.left,
        rect.bottom - rect.top,
        HWND(0),
        HMENU(0),
        instance,
        None,
    );
    if hwnd.0 == 0 {
        None
    } else {
        ShowWindow(hwnd, SW_SHOW);
        Some(hwnd)
    }
}

fn enumerate_monitors() -> Vec<MonitorDescriptor> {
    unsafe extern "system" fn callback(
        hmonitor: HMONITOR,
        _hdc: HDC,
        rect: *mut RECT,
        lparam: LPARAM,
    ) -> BOOL {
        let monitors = &mut *(lparam.0 as *mut Vec<MonitorDescriptor>);
        let mut info = MONITORINFOEXW {
            monitorInfo: Default::default(),
            szDevice: [0; 32],
        };
        info.monitorInfo.cbSize = size_of::<MONITORINFOEXW>() as u32;
        if GetMonitorInfoW(hmonitor, &mut info.monitorInfo as *mut _ as *mut _) != BOOL(0) {
            let name = widestring_to_string(&info.szDevice);
            monitors.push(MonitorDescriptor {
                name,
                rect: *rect,
            });
        }
        BOOL(1)
    }

    let mut monitors = Vec::new();
    unsafe {
        let _ = EnumDisplayMonitors(
            HDC(0),
            None,
            Some(callback),
            LPARAM(&mut monitors as *mut _ as isize),
        );
    }
    monitors
}

fn canonicalize_name(name: &str) -> String {
    let trimmed = name.trim().trim_start_matches("\\\\.\\");
    let upper = trimmed.to_ascii_uppercase();
    if upper.starts_with("DISPLAY") {
        upper
    } else {
        format!("DISPLAY{upper}")
    }
}

fn widestring_to_string(buffer: &[u16]) -> String {
    let nul_pos = buffer.iter().position(|ch| *ch == 0).unwrap_or(buffer.len());
    String::from_utf16_lossy(&buffer[..nul_pos])
}

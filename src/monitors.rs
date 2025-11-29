use std::{collections::HashSet, mem::size_of};
use windows::{
    Win32::{
        Foundation::{BOOL, HWND, LPARAM, LRESULT, RECT, WPARAM},
        Graphics::Gdi::{
            BLACK_BRUSH, EnumDisplayMonitors, GetMonitorInfoW, GetStockObject, HBRUSH, HDC,
            HMONITOR, MONITORINFOEXW,
        },
        UI::WindowsAndMessaging::{
            CS_HREDRAW, CS_VREDRAW, CreateWindowExW, DestroyWindow, HMENU, RegisterClassW, SW_SHOW,
            ShowWindow, WINDOW_EX_STYLE, WNDCLASSW, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
            WS_VISIBLE,
        },
    },
    core::{PCWSTR, w},
};

use crate::settings::{MonitorBlankingMode, Settings};

const BLANK_CLASS: PCWSTR = w!("DwmLockBlankWindow");

#[derive(Clone)]
struct MonitorDescriptor {
    name: String,
    rect: RECT,
}

pub fn available_monitor_names() -> Vec<String> {
    enumerate_monitors().into_iter().map(|m| m.name).collect()
}

pub fn spawn_overlays(
    instance: windows::Win32::Foundation::HINSTANCE,
    settings: &Settings,
) -> Vec<HWND> {
    if matches!(settings.monitor_mode, MonitorBlankingMode::None) {
        return Vec::new();
    }

    if matches!(settings.monitor_mode, MonitorBlankingMode::Custom)
        && settings.disable_monitors.is_empty()
    {
        return Vec::new();
    }

    unsafe {
        register_blank_class(instance);
    }

    let disable_set: Option<HashSet<String>> = match settings.monitor_mode {
        MonitorBlankingMode::Custom => Some(
            settings
                .disable_monitors
                .iter()
                .map(|s| canonicalize_name(s))
                .collect(),
        ),
        _ => None,
    };

    let monitors = enumerate_monitors();
    let mut overlays = Vec::new();
    for monitor in monitors {
        let target = match settings.monitor_mode {
            MonitorBlankingMode::All => true,
            MonitorBlankingMode::Custom => {
                let canonical = canonicalize_name(&monitor.name);
                disable_set
                    .as_ref()
                    .map(|set| set.contains(&canonical))
                    .unwrap_or(false)
            }
            MonitorBlankingMode::None => false,
        };

        if target {
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

unsafe extern "system" fn blank_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    windows::Win32::UI::WindowsAndMessaging::DefWindowProcW(hwnd, msg, wparam, lparam)
}

unsafe fn register_blank_class(instance: windows::Win32::Foundation::HINSTANCE) {
    use std::sync::Once;
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let brush = GetStockObject(BLACK_BRUSH);
        let class = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(blank_wnd_proc),
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
        let _ = ShowWindow(hwnd, SW_SHOW);
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
            monitors.push(MonitorDescriptor { name, rect: *rect });
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
    let nul_pos = buffer
        .iter()
        .position(|ch| *ch == 0)
        .unwrap_or(buffer.len());
    String::from_utf16_lossy(&buffer[..nul_pos])
}

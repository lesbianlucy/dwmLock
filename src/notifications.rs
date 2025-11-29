#![allow(unsafe_op_in_unsafe_fn)]

use windows::Win32::{
    Foundation::{BOOL, HWND, LPARAM, WPARAM},
    UI::WindowsAndMessaging::{
        EnumWindows, GetClassNameW, GetWindowTextW, IsWindowVisible, PostMessageW, WM_CLOSE,
    },
};

const BUFFER: usize = 256;

pub fn dismiss_notifications() {
    unsafe {
        let _ = EnumWindows(Some(enumerate_windows), LPARAM(0));
    }
}

unsafe extern "system" fn enumerate_windows(hwnd: HWND, _lparam: LPARAM) -> BOOL {
    if !IsWindowVisible(hwnd).as_bool() {
        return BOOL(1);
    }

    let class_name = window_text(|buffer| GetClassNameW(hwnd, buffer));
    let title = window_text(|buffer| GetWindowTextW(hwnd, buffer));

    if looks_like_notification(&class_name, &title) {
        let _ = PostMessageW(hwnd, WM_CLOSE, WPARAM(0), LPARAM(0));
    }
    BOOL(1)
}

unsafe fn window_text<F>(mut getter: F) -> String
where
    F: FnMut(&mut [u16]) -> i32,
{
    let mut buffer = [0u16; BUFFER];
    let len = getter(&mut buffer);
    if len > 0 {
        String::from_utf16_lossy(&buffer[..len as usize])
    } else {
        String::new()
    }
}

fn looks_like_notification(class_name: &str, title: &str) -> bool {
    let class = class_name.to_ascii_lowercase();
    let title = title.to_ascii_lowercase();
    class.contains("toast")
        || class.contains("notification")
        || (class.contains("corewindow") && title.contains("notification"))
        || title.contains("notification")
        || title.contains("action center")
}

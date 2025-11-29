#![allow(unsafe_op_in_unsafe_fn)]

use windows::{
    Win32::{
        Foundation::{HINSTANCE, LPARAM, LRESULT, WPARAM},
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            Input::KeyboardAndMouse::{
                GetAsyncKeyState, VK_CONTROL, VK_DELETE, VK_LCONTROL, VK_LMENU, VK_MENU,
                VK_RCONTROL, VK_RMENU,
            },
            WindowsAndMessaging::{
                CallNextHookEx, HC_ACTION, HHOOK, KBDLLHOOKSTRUCT, SetWindowsHookExW,
                UnhookWindowsHookEx, WH_KEYBOARD_LL, WM_KEYDOWN, WM_SYSKEYDOWN,
            },
        },
    },
    core::Result,
};

pub struct CtrlAltDeleteHook {
    handle: HHOOK,
}

impl CtrlAltDeleteHook {
    pub unsafe fn install() -> Result<Self> {
        let instance: HINSTANCE = GetModuleHandleW(None)?.into();
        let handle = SetWindowsHookExW(WH_KEYBOARD_LL, Some(low_level_keyboard_proc), instance, 0)?;
        Ok(Self { handle })
    }
}

impl Drop for CtrlAltDeleteHook {
    fn drop(&mut self) {
        unsafe {
            let _ = UnhookWindowsHookEx(self.handle);
        }
    }
}

unsafe extern "system" fn low_level_keyboard_proc(
    code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if code == HC_ACTION as i32
        && (wparam.0 as u32 == WM_KEYDOWN || wparam.0 as u32 == WM_SYSKEYDOWN)
    {
        let info = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
        if is_delete(info.vkCode) && ctrl_alt_active() {
            // Block Ctrl+Alt+Delete by swallowing the key event.
            return LRESULT(1);
        }
    }

    CallNextHookEx(HHOOK(0), code, wparam, lparam)
}

fn is_delete(vk_code: u32) -> bool {
    vk_code == VK_DELETE.0 as u32
}

fn ctrl_alt_active() -> bool {
    let ctrl = unsafe {
        GetAsyncKeyState(VK_CONTROL.0 as i32) < 0
            || GetAsyncKeyState(VK_LCONTROL.0 as i32) < 0
            || GetAsyncKeyState(VK_RCONTROL.0 as i32) < 0
    };
    let alt = unsafe {
        GetAsyncKeyState(VK_MENU.0 as i32) < 0
            || GetAsyncKeyState(VK_LMENU.0 as i32) < 0
            || GetAsyncKeyState(VK_RMENU.0 as i32) < 0
    };
    ctrl && alt
}

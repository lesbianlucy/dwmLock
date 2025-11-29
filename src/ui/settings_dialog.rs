use std::{ffi::OsStr, os::windows::ffi::OsStrExt, sync::Once};

use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
        Graphics::Gdi::{GetStockObject, HBRUSH, WHITE_BRUSH},
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::{
            BM_GETCHECK, BM_SETCHECK, BN_CLICKED, CB_ADDSTRING, CB_GETCURSEL, CB_SETCURSEL,
            CBN_SELCHANGE, CBS_DROPDOWNLIST, CBS_HASSTRINGS, CREATESTRUCTW, CreateWindowExW,
            DefWindowProcW, DestroyWindow, DispatchMessageW, GWL_STYLE, GWLP_USERDATA, GetMessageW,
            GetSystemMetrics, GetWindowLongPtrW, GetWindowRect, GetWindowTextW, HMENU,
            HWND_TOPMOST, IDC_ARROW, LB_ADDSTRING, LB_DELETESTRING, LB_GETCURSEL, LB_RESETCONTENT,
            LoadCursorW, MSG, PostQuitMessage, RegisterClassW, SM_CXSCREEN, SM_CYSCREEN, SW_SHOW,
            SWP_NOSIZE, SendMessageW, SetWindowLongPtrW, SetWindowPos, SetWindowTextW, ShowWindow,
            TranslateMessage, WINDOW_EX_STYLE, WINDOW_STYLE, WM_CLOSE, WM_COMMAND, WM_CREATE,
            WM_DESTROY, WM_NCCREATE, WNDCLASS_STYLES, WNDCLASSW, WS_CHILD, WS_DISABLED,
            WS_EX_CLIENTEDGE, WS_EX_TOPMOST, WS_OVERLAPPEDWINDOW, WS_TABSTOP, WS_VISIBLE,
        },
    },
    core::{PCWSTR, Result, w},
};

use crate::{
    monitors::available_monitor_names,
    settings::{MonitorBlankingMode, Settings},
};

const SETTINGS_CLASS_NAME: PCWSTR = w!("DwmLockSettingsWindow");
const SETTINGS_WIDTH: i32 = 520;
const SETTINGS_HEIGHT: i32 = 460;

const ID_MONITOR_MODE_COMBO: isize = 1000;
const ID_PASSWORD_EDIT: isize = 1001;
const ID_SHOW_ON_START: isize = 1002;
const ID_DISMISS_NOTIFICATIONS: isize = 1003;
const ID_MONITOR_COMBO: isize = 1004;
const ID_MONITOR_LIST: isize = 1005;
const ID_MONITOR_ADD: isize = 1006;
const ID_MONITOR_REMOVE: isize = 1007;
const ID_BLUR_EDIT: isize = 1008;
const ID_BLUR_CHECKBOX: isize = 1009;
const ID_APPLY_BUTTON: isize = 1010;
const ID_CLOSE_BUTTON: isize = 1011;
const ID_TEXT_ON_MONITORS: isize = 1012;
const BST_CHECKED_STATE: usize = 1;
const BST_UNCHECKED_STATE: usize = 0;
const MONITOR_MODE_OPTIONS: &[(MonitorBlankingMode, &str); 3] = &[
    (MonitorBlankingMode::None, "Keep all monitors on"),
    (MonitorBlankingMode::All, "Turn off every monitor"),
    (MonitorBlankingMode::Custom, "Choose specific monitors"),
];

static SETTINGS_CLASS: Once = Once::new();

pub unsafe fn show_settings_dialog(settings: &mut Settings) -> Result<bool> {
    let instance = GetModuleHandleW(None)?;
    register_settings_window_class(instance.into());

    let state = Box::new(SettingsDialogState::new(
        settings,
        available_monitor_names(),
    ));
    let state_ptr = Box::into_raw(state);

    let hwnd = CreateWindowExW(
        WINDOW_EX_STYLE(WS_EX_TOPMOST.0),
        SETTINGS_CLASS_NAME,
        w!("DwmLock Settings"),
        WS_OVERLAPPEDWINDOW,
        0,
        0,
        SETTINGS_WIDTH,
        SETTINGS_HEIGHT,
        HWND(0),
        HMENU(0),
        instance,
        Some(state_ptr as *mut _ as *mut core::ffi::c_void),
    );
    if hwnd.0 == 0 {
        let _ = Box::from_raw(state_ptr);
        return Err(windows::core::Error::from_win32());
    }

    center_window(hwnd);
    let _ = ShowWindow(hwnd, SW_SHOW);
    let mut message = MSG::default();
    while GetMessageW(&mut message, HWND(0), 0, 0).into() {
        let _ = TranslateMessage(&message);
        DispatchMessageW(&message);
    }

    let state = Box::from_raw(state_ptr);
    Ok(state.applied)
}

fn register_settings_window_class(instance: windows::Win32::Foundation::HINSTANCE) {
    SETTINGS_CLASS.call_once(|| unsafe {
        let class = WNDCLASSW {
            style: WNDCLASS_STYLES::default(),
            lpfnWndProc: Some(settings_wnd_proc),
            hInstance: instance,
            lpszClassName: SETTINGS_CLASS_NAME,
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            hbrBackground: HBRUSH(GetStockObject(WHITE_BRUSH).0),
            ..Default::default()
        };
        let _ = RegisterClassW(&class);
    });
}

unsafe extern "system" fn settings_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_NCCREATE => {
            let createstruct = &*(lparam.0 as *const CREATESTRUCTW);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, createstruct.lpCreateParams as isize);
            LRESULT(1)
        }
        WM_CREATE => {
            if let Some(state) = dialog_state_mut(hwnd) {
                state.initialize_controls(hwnd);
            }
            LRESULT(0)
        }
        WM_COMMAND => {
            if let Some(state) = dialog_state_mut(hwnd) {
                handle_command(hwnd, state, wparam);
            }
            LRESULT(0)
        }
        WM_CLOSE => {
            let _ = DestroyWindow(hwnd);
            LRESULT(0)
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

struct SettingsDialogState {
    settings: *mut Settings,
    monitor_names: Vec<String>,
    selected_monitors: Vec<String>,
    password_edit: Option<HWND>,
    blur_edit: Option<HWND>,
    blur_checkbox: Option<HWND>,
    show_checkbox: Option<HWND>,
    dismiss_checkbox: Option<HWND>,
    monitor_mode_combo: Option<HWND>,
    monitor_combo: Option<HWND>,
    monitor_list: Option<HWND>,
    monitor_add_button: Option<HWND>,
    monitor_remove_button: Option<HWND>,
    monitor_mode: MonitorBlankingMode,
    blur_enabled: bool,
    text_checkbox: Option<HWND>,
    text_on_all_monitors: bool,
    applied: bool,
}

impl SettingsDialogState {
    fn new(settings: &mut Settings, monitor_names: Vec<String>) -> Self {
        Self {
            settings,
            monitor_names,
            selected_monitors: settings.disable_monitors.clone(),
            password_edit: None,
            blur_edit: None,
            blur_checkbox: None,
            show_checkbox: None,
            dismiss_checkbox: None,
            monitor_mode_combo: None,
            monitor_combo: None,
            monitor_list: None,
            monitor_add_button: None,
            monitor_remove_button: None,
            monitor_mode: settings.monitor_mode,
            blur_enabled: settings.blur_enabled,
            text_checkbox: None,
            text_on_all_monitors: settings.text_on_all_monitors,
            applied: false,
        }
    }

    unsafe fn initialize_controls(&mut self, hwnd: HWND) {
        let mut layout_y = 20;
        let left = 20;
        let content_width = SETTINGS_WIDTH - (left * 2);
        self.password_edit = Some(create_labeled_edit(
            hwnd,
            "Password",
            left,
            &mut layout_y,
            content_width,
            ID_PASSWORD_EDIT,
        ));

        if let Some(edit) = self.password_edit {
            let current = self.current_password();
            set_edit_text(edit, &current);
        }

        layout_y += 10;
        self.blur_edit = Some(create_labeled_edit(
            hwnd,
            "Blur radius (1-64)",
            left,
            &mut layout_y,
            content_width,
            ID_BLUR_EDIT,
        ));
        if let Some(edit) = self.blur_edit {
            set_edit_text(edit, &self.current_blur_radius());
        }

        layout_y += 10;
        self.blur_checkbox = Some(create_checkbox(
            hwnd,
            "Enable blur background",
            left,
            layout_y,
            content_width,
            ID_BLUR_CHECKBOX,
        ));
        if let Some(checkbox) = self.blur_checkbox {
            set_checkbox_state(checkbox, self.blur_enabled);
        }

        layout_y += 30;
        self.show_checkbox = Some(create_checkbox(
            hwnd,
            "Open settings on startup",
            left,
            layout_y,
            content_width,
            ID_SHOW_ON_START,
        ));
        layout_y += 30;
        self.dismiss_checkbox = Some(create_checkbox(
            hwnd,
            "Dismiss notifications before locking",
            left,
            layout_y,
            content_width,
            ID_DISMISS_NOTIFICATIONS,
        ));
        if let Some(show) = self.show_checkbox {
            set_checkbox_state(show, self.bool_flag(|s| s.open_settings_on_startup));
        }
        if let Some(dismiss) = self.dismiss_checkbox {
            set_checkbox_state(
                dismiss,
                self.bool_flag(|s| s.dismiss_notifications_on_startup),
            );
        }

        layout_y += 30;
        create_label(hwnd, "Monitor behavior", left, layout_y - 10, content_width);
        self.monitor_mode_combo = Some(create_combo(
            hwnd,
            left,
            layout_y,
            content_width,
            ID_MONITOR_MODE_COMBO,
        ));
        if let Some(combo) = self.monitor_mode_combo {
            let labels: Vec<String> = MONITOR_MODE_OPTIONS
                .iter()
                .map(|(_, text)| text.to_string())
                .collect();
            populate_combo(combo, &labels);
            set_monitor_mode_selection(combo, self.monitor_mode);
        }

        layout_y += 40;
        create_label(
            hwnd,
            "Monitors to blank",
            left,
            layout_y - 10,
            content_width,
        );
        self.monitor_combo = Some(create_combo(
            hwnd,
            left,
            layout_y,
            content_width - 140,
            ID_MONITOR_COMBO,
        ));
        if let Some(combo) = self.monitor_combo {
            populate_combo(combo, &self.monitor_names);
        }
        self.monitor_add_button = Some(create_button(
            hwnd,
            "Add",
            left + content_width - 130,
            layout_y,
            60,
            28,
            ID_MONITOR_ADD,
        ));
        self.monitor_remove_button = Some(create_button(
            hwnd,
            "Remove",
            left + content_width - 65,
            layout_y,
            60,
            28,
            ID_MONITOR_REMOVE,
        ));

        layout_y += 40;
        self.monitor_list = Some(create_listbox(
            hwnd,
            left,
            layout_y,
            content_width,
            120,
            ID_MONITOR_LIST,
        ));
        if let Some(list) = self.monitor_list {
            populate_list(list, &self.selected_monitors);
        }

        layout_y += 10;
        self.text_checkbox = Some(create_checkbox(
            hwnd,
            "Show lock text on each monitor",
            left,
            layout_y,
            content_width,
            ID_TEXT_ON_MONITORS,
        ));
        if let Some(text) = self.text_checkbox {
            set_checkbox_state(text, self.text_on_all_monitors);
        }
        layout_y += 30;
        layout_y += 120;
        create_button(
            hwnd,
            "Apply Changes",
            left,
            layout_y,
            120,
            32,
            ID_APPLY_BUTTON,
        );
        create_button(hwnd, "Close", left + 140, layout_y, 90, 32, ID_CLOSE_BUTTON);

        self.update_blur_edit_state();
        self.update_monitor_control_state();
    }

    fn bool_flag<F>(&self, getter: F) -> bool
    where
        F: Fn(&Settings) -> bool,
    {
        unsafe { getter(&*self.settings) }
    }

    fn blur_controls_enabled(&self) -> bool {
        self.blur_enabled
    }

    unsafe fn blur_checkbox_changed(&mut self) {
        if let Some(checkbox) = self.blur_checkbox {
            self.blur_enabled = checkbox_checked(checkbox);
            self.update_blur_edit_state();
        }
    }

    unsafe fn text_checkbox_changed(&mut self) {
        if let Some(checkbox) = self.text_checkbox {
            self.text_on_all_monitors = checkbox_checked(checkbox);
        }
    }

    unsafe fn update_blur_edit_state(&self) {
        let enabled = self.blur_controls_enabled();
        if let Some(edit) = self.blur_edit {
            set_control_enabled(edit, enabled);
        }
    }

    fn monitor_mode_is_custom(&self) -> bool {
        matches!(self.monitor_mode, MonitorBlankingMode::Custom)
    }

    unsafe fn monitor_mode_combo_changed(&mut self) {
        let combo = match self.monitor_mode_combo {
            Some(c) => c,
            None => return,
        };
        let index = SendMessageW(combo, CB_GETCURSEL, WPARAM(0), LPARAM(0)).0 as isize;
        if let Some(mode) = monitor_mode_from_index(index) {
            self.monitor_mode = mode;
            self.update_monitor_control_state();
        }
    }

    unsafe fn update_monitor_control_state(&self) {
        let enabled = self.monitor_mode_is_custom();
        if let Some(combo) = self.monitor_combo {
            set_control_enabled(combo, enabled);
        }
        if let Some(list) = self.monitor_list {
            set_control_enabled(list, enabled);
        }
        if let Some(add) = self.monitor_add_button {
            set_control_enabled(add, enabled);
        }
        if let Some(remove) = self.monitor_remove_button {
            set_control_enabled(remove, enabled);
        }
    }

    unsafe fn add_selected_monitor(&mut self) {
        if !self.monitor_mode_is_custom() {
            return;
        }
        let combo = match self.monitor_combo {
            Some(c) => c,
            None => return,
        };
        let index = SendMessageW(combo, CB_GETCURSEL, WPARAM(0), LPARAM(0)).0 as isize;
        if index < 0 {
            return;
        }
        let name = self
            .monitor_names
            .get(index as usize)
            .cloned()
            .unwrap_or_default();
        if !self
            .selected_monitors
            .iter()
            .any(|m| m.eq_ignore_ascii_case(&name))
        {
            self.selected_monitors.push(name.clone());
            if let Some(list) = self.monitor_list {
                let _ = add_list_entry(list, &name);
            }
        }
    }

    unsafe fn remove_selected_monitor(&mut self) {
        if !self.monitor_mode_is_custom() {
            return;
        }
        let list = match self.monitor_list {
            Some(l) => l,
            None => return,
        };
        let index = SendMessageW(list, LB_GETCURSEL, WPARAM(0), LPARAM(0)).0 as isize;
        if index < 0 {
            return;
        }
        if (index as usize) < self.selected_monitors.len() {
            self.selected_monitors.remove(index as usize);
            SendMessageW(list, LB_DELETESTRING, WPARAM(index as usize), LPARAM(0));
        }
    }

    unsafe fn apply_settings(&mut self) {
        if let Some(password) = self.password_edit {
            let value = read_text(password);
            if !value.trim().is_empty() {
                (*self.settings).password = value;
            }
        }
        if let Some(blur_edit) = self.blur_edit {
            let value = read_text(blur_edit);
            if let Ok(parsed) = value.trim().parse::<usize>() {
                (*self.settings).blur_radius = parsed.max(1).min(64);
            }
        }
        if let Some(show) = self.show_checkbox {
            (*self.settings).open_settings_on_startup = checkbox_checked(show);
        }
        if let Some(dismiss) = self.dismiss_checkbox {
            (*self.settings).dismiss_notifications_on_startup = checkbox_checked(dismiss);
        }
        if let Some(text) = self.text_checkbox {
            self.text_on_all_monitors = checkbox_checked(text);
        }
        (*self.settings).monitor_mode = self.monitor_mode;
        (*self.settings).blur_enabled = self.blur_enabled;
        (*self.settings).disable_monitors = self.selected_monitors.clone();
        (*self.settings).text_on_all_monitors = self.text_on_all_monitors;
        self.applied = true;
    }

    fn current_password(&self) -> String {
        unsafe { (*self.settings).password.clone() }
    }

    fn current_blur_radius(&self) -> String {
        unsafe { (*self.settings).blur_radius.max(1).to_string() }
    }
}

unsafe fn handle_command(hwnd: HWND, state: &mut SettingsDialogState, wparam: WPARAM) {
    let command = ((wparam.0 >> 16) & 0xffff) as i32;
    let control_id = (wparam.0 & 0xffff) as isize;

    match command {
        code if code == BN_CLICKED as i32 => match control_id {
            ID_BLUR_CHECKBOX => state.blur_checkbox_changed(),
            ID_TEXT_ON_MONITORS => state.text_checkbox_changed(),
            ID_MONITOR_ADD => state.add_selected_monitor(),
            ID_MONITOR_REMOVE => state.remove_selected_monitor(),
            ID_APPLY_BUTTON => state.apply_settings(),
            ID_CLOSE_BUTTON => {
                state.apply_settings();
                let _ = DestroyWindow(hwnd);
            }
            _ => {}
        },
        code if code == CBN_SELCHANGE as i32 => {
            if control_id == ID_MONITOR_MODE_COMBO {
                state.monitor_mode_combo_changed();
            }
        }
        _ => {}
    }
}

unsafe fn dialog_state_mut(hwnd: HWND) -> Option<&'static mut SettingsDialogState> {
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut SettingsDialogState;
    ptr.as_mut()
}

unsafe fn create_label(hwnd: HWND, text: &str, x: i32, y: i32, width: i32) {
    let wide = to_wide(text);
    let _ = CreateWindowExW(
        WINDOW_EX_STYLE(0),
        w!("STATIC"),
        PCWSTR(wide.as_ptr()),
        WS_CHILD | WS_VISIBLE,
        x,
        y,
        width,
        20,
        hwnd,
        HMENU(0),
        None,
        None,
    );
}

unsafe fn create_labeled_edit(
    hwnd: HWND,
    label: &str,
    left: i32,
    layout_y: &mut i32,
    width: i32,
    id: isize,
) -> HWND {
    create_label(hwnd, label, left, *layout_y - 2, width);
    let edit = CreateWindowExW(
        WINDOW_EX_STYLE(WS_EX_CLIENTEDGE.0),
        w!("EDIT"),
        PCWSTR::null(),
        WS_CHILD | WS_VISIBLE | WS_TABSTOP,
        left,
        *layout_y + 18,
        width,
        26,
        hwnd,
        HMENU(id),
        None,
        None,
    );
    *layout_y += 40;
    edit
}

unsafe fn create_checkbox(hwnd: HWND, text: &str, x: i32, y: i32, width: i32, id: isize) -> HWND {
    let wide = to_wide(text);
    CreateWindowExW(
        WINDOW_EX_STYLE(0),
        w!("BUTTON"),
        PCWSTR(wide.as_ptr()),
        WS_CHILD | WS_VISIBLE | WS_TABSTOP,
        x,
        y,
        width,
        24,
        hwnd,
        HMENU(id),
        None,
        None,
    )
}

unsafe fn create_combo(hwnd: HWND, x: i32, y: i32, width: i32, id: isize) -> HWND {
    CreateWindowExW(
        WINDOW_EX_STYLE(0),
        w!("COMBOBOX"),
        PCWSTR::null(),
        WS_CHILD
            | WS_VISIBLE
            | WS_TABSTOP
            | WINDOW_STYLE(CBS_DROPDOWNLIST as u32)
            | WINDOW_STYLE(CBS_HASSTRINGS as u32),
        x,
        y,
        width,
        200,
        hwnd,
        HMENU(id),
        None,
        None,
    )
}

unsafe fn create_listbox(hwnd: HWND, x: i32, y: i32, width: i32, height: i32, id: isize) -> HWND {
    CreateWindowExW(
        WINDOW_EX_STYLE(WS_EX_CLIENTEDGE.0),
        w!("LISTBOX"),
        PCWSTR::null(),
        WS_CHILD | WS_VISIBLE | WS_TABSTOP,
        x,
        y,
        width,
        height,
        hwnd,
        HMENU(id),
        None,
        None,
    )
}

unsafe fn create_button(
    hwnd: HWND,
    text: &str,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    id: isize,
) -> HWND {
    let wide = to_wide(text);
    CreateWindowExW(
        WINDOW_EX_STYLE(0),
        w!("BUTTON"),
        PCWSTR(wide.as_ptr()),
        WS_CHILD | WS_VISIBLE | WS_TABSTOP,
        x,
        y,
        width,
        height,
        hwnd,
        HMENU(id),
        None,
        None,
    )
}

unsafe fn populate_combo(combo: HWND, items: &[String]) {
    for entry in items {
        let wide = to_wide(entry);
        SendMessageW(
            combo,
            CB_ADDSTRING,
            WPARAM(0),
            LPARAM(wide.as_ptr() as isize),
        );
    }
}

unsafe fn set_monitor_mode_selection(combo: HWND, mode: MonitorBlankingMode) {
    if let Some(index) = monitor_mode_option_index(mode) {
        SendMessageW(combo, CB_SETCURSEL, WPARAM(index), LPARAM(0));
    }
}

fn monitor_mode_option_index(mode: MonitorBlankingMode) -> Option<usize> {
    MONITOR_MODE_OPTIONS
        .iter()
        .position(|(value, _)| *value == mode)
}

fn monitor_mode_from_index(index: isize) -> Option<MonitorBlankingMode> {
    if index < 0 {
        return None;
    }
    MONITOR_MODE_OPTIONS
        .get(index as usize)
        .map(|(value, _)| *value)
}

unsafe fn set_control_enabled(control: HWND, enabled: bool) {
    if control.0 == 0 {
        return;
    }
    let mut style = GetWindowLongPtrW(control, GWL_STYLE);
    if enabled {
        style &= !(WS_DISABLED.0 as isize);
    } else {
        style |= WS_DISABLED.0 as isize;
    }
    let _ = SetWindowLongPtrW(control, GWL_STYLE, style);
}

unsafe fn populate_list(list: HWND, items: &[String]) {
    SendMessageW(list, LB_RESETCONTENT, WPARAM(0), LPARAM(0));
    for entry in items {
        let _ = add_list_entry(list, entry);
    }
}

unsafe fn add_list_entry(list: HWND, entry: &str) {
    let wide = to_wide(entry);
    SendMessageW(
        list,
        LB_ADDSTRING,
        WPARAM(0),
        LPARAM(wide.as_ptr() as isize),
    );
}

unsafe fn set_edit_text(edit: HWND, text: &str) {
    let wide = to_wide(text);
    let _ = SetWindowTextW(edit, PCWSTR(wide.as_ptr()));
}

unsafe fn set_checkbox_state(checkbox: HWND, checked: bool) {
    let state = if checked {
        BST_CHECKED_STATE
    } else {
        BST_UNCHECKED_STATE
    };
    SendMessageW(checkbox, BM_SETCHECK, WPARAM(state), LPARAM(0));
}

unsafe fn checkbox_checked(checkbox: HWND) -> bool {
    let state = SendMessageW(checkbox, BM_GETCHECK, WPARAM(0), LPARAM(0));
    state.0 as usize == BST_CHECKED_STATE
}

unsafe fn read_text(control: HWND) -> String {
    let mut buffer = vec![0u16; 256];
    let len = GetWindowTextW(control, &mut buffer);
    if len > 0 {
        String::from_utf16_lossy(&buffer[..len as usize])
    } else {
        String::new()
    }
}

fn to_wide(text: &str) -> Vec<u16> {
    let mut wide: Vec<u16> = OsStr::new(text).encode_wide().collect();
    wide.push(0);
    wide
}

unsafe fn center_window(hwnd: HWND) {
    let mut rect = RECT::default();
    if GetWindowRect(hwnd, &mut rect).is_ok() {
        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;
        let screen_width = GetSystemMetrics(SM_CXSCREEN);
        let screen_height = GetSystemMetrics(SM_CYSCREEN);
        let left = (screen_width - width) / 2;
        let top = (screen_height - height) / 2;
        let _ = SetWindowPos(
            hwnd,
            HWND_TOPMOST,
            left.max(0),
            top.max(0),
            width,
            height,
            SWP_NOSIZE,
        );
    }
}

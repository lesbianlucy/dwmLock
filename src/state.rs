use once_cell::sync::OnceCell;
use std::{sync::Mutex, time::Instant};
use windows::{Win32::Foundation::HWND, Win32::Graphics::Gdi::BITMAPINFO};

use crate::{config::WARNING_DURATION, game::MiniGameState, settings::Settings};

pub static APP_STATE: OnceCell<Mutex<AppState>> = OnceCell::new();

#[derive(Debug)]
pub struct AppState {
    pub width: i32,
    pub height: i32,
    pub pixels: Vec<u8>,
    pub bitmap_info: BITMAPINFO,
    pub password: String,
    pub input: String,
    pub warning_since: Option<Instant>,
    pub settings: Settings,
    pub monitor_windows: Vec<HWND>,
    pub game: MiniGameState,
}

pub fn init_state(state: AppState) {
    APP_STATE
        .set(Mutex::new(state))
        .expect("state already initialized");
}

pub fn app_state() -> &'static Mutex<AppState> {
    APP_STATE.get().expect("state not initialized")
}

pub fn mark_warning() {
    if let Some(cell) = APP_STATE.get() {
        if let Ok(mut state) = cell.lock() {
            arm_warning(&mut state);
        }
    }
}

pub fn arm_warning(state: &mut AppState) {
    state.warning_since = Some(Instant::now());
}

pub fn warning_active(state: &AppState) -> bool {
    state
        .warning_since
        .map(|instant| instant.elapsed() < WARNING_DURATION)
        .unwrap_or(false)
}

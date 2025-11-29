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
mod state;
mod ui;

use crate::{
    blur::blur_buffer,
    capture::{build_bitmap_info, capture_screen},
    keyboard::CtrlAltDeleteHook,
    notifications::dismiss_notifications,
    settings::{Settings, load_settings, persist_settings},
    state::{AppState, init_state},
    ui::{
        settings_dialog::show_settings_dialog,
        window::{confirm_lock, create_window_loop},
    },
};
use std::{env, process};
use windows::core::Result;

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
        if settings.blur_enabled {
            blur_buffer(
                &mut captured.pixels,
                captured.width as usize,
                captured.height as usize,
                settings.blur_radius.max(1),
            );
        }
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

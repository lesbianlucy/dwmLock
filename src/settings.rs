use dirs::config_dir;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

use crate::config::{DEFAULT_BLUR_RADIUS, DEFAULT_PASSWORD, SETTINGS_DIR_NAME, SETTINGS_FILE_NAME};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MonitorBlankingMode {
    None,
    All,
    Custom,
}

impl Default for MonitorBlankingMode {
    fn default() -> Self {
        MonitorBlankingMode::Custom
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default = "default_password")]
    pub password: String,
    #[serde(default = "default_disable_monitors")]
    pub disable_monitors: Vec<String>,
    #[serde(default)]
    pub monitor_mode: MonitorBlankingMode,
    #[serde(default, alias = "show_settings_on_startup")]
    pub open_settings_on_startup: bool,
    #[serde(default = "default_dismiss_notifications")]
    pub dismiss_notifications_on_startup: bool,
    #[serde(default = "default_blur_radius")]
    pub blur_radius: usize,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            password: default_password(),
            disable_monitors: default_disable_monitors(),
            monitor_mode: MonitorBlankingMode::default(),
            open_settings_on_startup: false,
            dismiss_notifications_on_startup: true,
            blur_radius: default_blur_radius(),
        }
    }
}

pub fn load_settings() -> Settings {
    let path = settings_path();
    let mut settings = if let Ok(bytes) = fs::read(&path) {
        serde_json::from_slice(&bytes).unwrap_or_else(|_| ensure_default(&path))
    } else {
        ensure_default(&path)
    };
    if settings.password.trim().is_empty() {
        settings.password = default_password();
    }
    if settings.blur_radius == 0 {
        settings.blur_radius = default_blur_radius();
    }
    settings
}

fn ensure_default(path: &PathBuf) -> Settings {
    let settings = Settings::default();
    save_settings(path, &settings);
    settings
}

fn save_settings(path: &PathBuf, settings: &Settings) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_vec_pretty(settings) {
        let _ = fs::write(path, json);
    }
}

pub fn persist_settings(settings: &Settings) {
    let path = settings_path();
    save_settings(&path, settings);
}

fn default_password() -> String {
    DEFAULT_PASSWORD.to_string()
}

fn default_disable_monitors() -> Vec<String> {
    vec!["DISPLAY2".to_string()]
}

fn default_dismiss_notifications() -> bool {
    true
}

fn default_blur_radius() -> usize {
    DEFAULT_BLUR_RADIUS
}

pub fn settings_path() -> PathBuf {
    let mut base = config_dir()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    base.push(SETTINGS_DIR_NAME);
    base.push(SETTINGS_FILE_NAME);
    base
}

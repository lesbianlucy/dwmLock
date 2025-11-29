use dirs::config_dir;
use serde::{Deserialize, Serialize};
use std::{fs, io, path::PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub disable_monitors: Vec<String>,
    pub minigame_autostart: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            disable_monitors: vec!["DISPLAY2".to_string()],
            minigame_autostart: false,
        }
    }
}

pub fn load_settings() -> Settings {
    let path = settings_path();
    if let Ok(bytes) = fs::read(&path) {
        serde_json::from_slice(&bytes).unwrap_or_else(|_| ensure_default(&path))
    } else {
        ensure_default(&path)
    }
}

fn ensure_default(path: &PathBuf) -> Settings {
    let settings = Settings::default();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_vec_pretty(&settings) {
        let _ = fs::write(path, json);
    }
    settings
}

pub fn settings_path() -> PathBuf {
    let mut base = config_dir().unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    base.push("LockWin");
    base.push("lockwin_settings.json");
    base
}

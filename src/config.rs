use std::time::Duration;
use windows::core::{PCWSTR, w};

pub const DEFAULT_PASSWORD: &str = "media";
pub const DEFAULT_BLUR_RADIUS: usize = 12;
pub const WARNING_DURATION: Duration = Duration::from_secs(5);
pub const TIMER_ID: usize = 1;
pub const TIMER_INTERVAL_MS: u32 = 1000;
pub const WARNING_MESSAGE: &str = "Fuck off BITCH";
pub const CLASS_NAME: PCWSTR = w!("DwmLockMainWindow");
pub const APPROVAL_PROMPT: PCWSTR = w!(
    "Lock screen now?\nThis will blur the display and capture input until you type the password."
);
pub const APPROVAL_CAPTION: PCWSTR = w!("DwmLock consent");
pub const SETTINGS_DIR_NAME: &str = "DwmLock";
pub const SETTINGS_FILE_NAME: &str = "dwmlock_settings.json";

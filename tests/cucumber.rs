use async_trait::async_trait;
use cucumber::{WorldInit, given, then, when};
use std::convert::Infallible;

use dwmlock::settings::{MonitorBlankingMode, Settings};

#[derive(Debug, WorldInit)]
struct LockWorld {
    settings: Settings,
}

impl Default for LockWorld {
    fn default() -> Self {
        Self {
            settings: Settings::default(),
        }
    }
}

#[async_trait(?Send)]
impl cucumber::World for LockWorld {
    type Error = Infallible;

    async fn new() -> Result<Self, Self::Error> {
        Ok(Self::default())
    }
}

#[given("default settings")]
async fn default_settings(world: &mut LockWorld) {
    world.settings = Settings::default();
}

#[when("the user disables blur")]
fn disable_blur(world: &mut LockWorld) {
    world.settings.blur_enabled = false;
}

#[when("the user enables monitor text")]
fn enable_monitor_text(world: &mut LockWorld) {
    world.settings.text_on_all_monitors = true;
}

#[then("blur should be disabled")]
fn assert_blur_disabled(world: &mut LockWorld) {
    assert!(!world.settings.blur_enabled, "blur should be off");
}

#[then("monitor text should be enabled")]
fn assert_monitor_text_on(world: &mut LockWorld) {
    assert!(
        world.settings.text_on_all_monitors,
        "monitor text should be on"
    );
}

#[then("monitor mode should be custom")]
fn assert_monitor_mode_custom(world: &mut LockWorld) {
    assert_eq!(world.settings.monitor_mode, MonitorBlankingMode::Custom);
}

#[then("disable list contains DISPLAY2")]
fn assert_default_disable_list(world: &mut LockWorld) {
    assert!(
        world
            .settings
            .disable_monitors
            .iter()
            .any(|entry| entry == "DISPLAY2")
    );
}

#[tokio::test]
async fn cucumber_features() {
    LockWorld::run("tests/features").await;
}

use rand::{distributions::SliceRandom, thread_rng};
use std::time::{Duration, Instant};

const TARGETS: &[u8] = b"asdfghjklqwertyuiopzxcvbnm1234567890";
const TICK_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone)]
pub struct MiniGameState {
    pub active: bool,
    pub target: char,
    pub last_update: Instant,
    pub score: u32,
    pub misses: u32,
}

impl MiniGameState {
    pub fn new(active: bool) -> Self {
        let mut state = Self {
            active,
            target: 'a',
            last_update: Instant::now(),
            score: 0,
            misses: 0,
        };
        state.target = state.random_target();
        state
    }

    pub fn toggle(&mut self) {
        self.active = !self.active;
        self.last_update = Instant::now();
    }

    pub fn handle_char(&mut self, ch: char) {
        if !self.active {
            return;
        }

        if ch.eq_ignore_ascii_case(&self.target) {
            self.score = self.score.saturating_add(1);
            self.target = self.random_target();
            self.last_update = Instant::now();
        } else if ch.is_ascii() {
            self.misses = self.misses.saturating_add(1);
        }
    }

    pub fn tick(&mut self) {
        if self.active && self.last_update.elapsed() >= TICK_TIMEOUT {
            self.misses = self.misses.saturating_add(1);
            self.target = self.random_target();
            self.last_update = Instant::now();
        }
    }

    pub fn target_display(&self) -> String {
        self.target.to_ascii_uppercase().to_string()
    }

    fn random_target(&self) -> char {
        let mut rng = thread_rng();
        let byte = TARGETS.choose(&mut rng).copied().unwrap_or(b'a');
        byte as char
    }
}

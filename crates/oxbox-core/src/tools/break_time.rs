//! Tool: **Break Time** (`break-time`). Scheduler takes an injected monotonic clock.
use crate::{CoreError, CoreResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BreakSettings {
    pub minutes: u64,
    pub eye_rest: bool,
    pub pause_fullscreen: bool,
    pub enabled: bool,
}
impl Default for BreakSettings {
    fn default() -> Self {
        Self {
            minutes: 20,
            eye_rest: false,
            pause_fullscreen: true,
            enabled: false,
        }
    }
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BreakStatus {
    pub enabled: bool,
    pub awaiting_break: bool,
    pub remaining_seconds: u64,
    pub eye_rest: bool,
    pub paused: bool,
}
#[derive(Default)]
pub struct BreakScheduler {
    period: u64,
    next: u64,
    last: u64,
    enabled: bool,
    awaiting: bool,
    eye_rest: bool,
    paused: bool,
}
impl BreakScheduler {
    pub fn start(&mut self, now: u64, minutes: u64, eye_rest: bool) -> CoreResult<()> {
        if minutes == 0 || minutes > 24 * 60 {
            return Err(CoreError::InvalidInput(
                "Choose a break interval from 1 to 1440 minutes.".into(),
            ));
        }
        self.period = minutes * 60;
        self.next = now.saturating_add(self.period);
        self.last = now;
        self.enabled = true;
        self.awaiting = false;
        self.eye_rest = eye_rest;
        self.paused = false;
        Ok(())
    }
    pub fn tick(&mut self, now: u64, inactive: bool) -> bool {
        let elapsed = now.saturating_sub(self.last);
        self.last = now;
        self.paused = inactive && self.enabled;
        if !self.enabled || self.awaiting {
            return false;
        }
        if inactive {
            self.next = self.next.saturating_add(elapsed);
            return false;
        }
        if now >= self.next {
            self.awaiting = true;
            return true;
        }
        false
    }
    pub fn snooze(&mut self, now: u64) {
        if self.enabled {
            self.next = now.saturating_add(300);
            self.last = now;
            self.awaiting = false;
        }
    }
    pub fn skip(&mut self, now: u64) {
        if self.enabled {
            self.next = now.saturating_add(self.period);
            self.last = now;
            self.awaiting = false;
        }
    }
    pub fn stop(&mut self) {
        self.enabled = false;
        self.awaiting = false;
        self.paused = false;
    }
    pub fn status(&self, now: u64) -> BreakStatus {
        BreakStatus {
            enabled: self.enabled,
            awaiting_break: self.awaiting,
            remaining_seconds: if self.enabled {
                self.next.saturating_sub(now)
            } else {
                0
            },
            eye_rest: self.eye_rest,
            paused: self.paused,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fake_clock_supports_pause_snooze_skip() {
        let mut s = BreakScheduler::default();
        s.start(0, 1, false).unwrap();
        assert!(!s.tick(59, false));
        assert!(s.tick(60, false));
        assert!(!s.tick(61, false));
        s.snooze(61);
        assert!(!s.tick(360, false));
        assert!(s.tick(361, false));
        s.skip(361);
        assert!(!s.tick(420, false));
        assert!(s.tick(421, false));
        s.stop();
        assert!(!s.tick(1000, false));
    }
    #[test]
    fn inactive_time_does_not_accumulate() {
        let mut s = BreakScheduler::default();
        s.start(0, 1, false).unwrap();
        assert!(!s.tick(30, false));
        assert!(!s.tick(90, true));
        assert!(!s.tick(119, false));
        assert!(s.tick(120, false));
        assert!(s.start(0, 0, false).is_err());
    }
}

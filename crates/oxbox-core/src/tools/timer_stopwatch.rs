//! Tool: **Timer & Stopwatch**. Injected monotonic milliseconds; no UI cadence dependency.
use crate::{CoreError, CoreResult};
#[derive(Debug, Clone, Default)]
pub struct Clock {
    duration: u64,
    started: Option<u64>,
    accumulated: u64,
    completed: bool,
    pub laps: Vec<u64>,
}
impl Clock {
    pub fn timer(duration: u64) -> CoreResult<Self> {
        if duration == 0 || duration > 7 * 24 * 60 * 60 * 1000 {
            return Err(CoreError::InvalidInput(
                "Choose a timer duration greater than zero and no longer than seven days.".into(),
            ));
        }
        Ok(Self {
            duration,
            ..Self::default()
        })
    }
    pub fn duration(&self) -> u64 {
        self.duration
    }
    pub fn running(&self) -> bool {
        self.started.is_some()
    }
    pub fn completed(&self) -> bool {
        self.completed
    }
    pub fn elapsed(&self, now: u64) -> u64 {
        self.accumulated.saturating_add(
            self.started
                .map_or(0, |started| now.saturating_sub(started)),
        )
    }
    pub fn remaining(&self, now: u64) -> u64 {
        self.duration.saturating_sub(self.elapsed(now))
    }
    pub fn start(&mut self, now: u64) {
        if !self.running() && !self.completed {
            self.started = Some(now);
        }
    }
    pub fn pause(&mut self, now: u64) {
        self.accumulated = self.elapsed(now);
        self.started = None;
    }
    pub fn reset(&mut self) {
        self.started = None;
        self.accumulated = 0;
        self.completed = false;
        self.laps.clear();
    }
    pub fn finish(&mut self, now: u64) -> bool {
        if self.duration == 0 || !self.running() || self.completed || self.remaining(now) > 0 {
            return false;
        }
        self.accumulated = self.duration;
        self.started = None;
        self.completed = true;
        true
    }
    pub fn lap(&mut self, now: u64) -> CoreResult<u64> {
        if !self.running() {
            return Err(CoreError::InvalidInput(
                "Start the stopwatch before recording a lap.".into(),
            ));
        }
        if self.laps.len() >= 1000 {
            return Err(CoreError::InvalidInput(
                "At most 1,000 laps may be recorded. Reset to start a new session.".into(),
            ));
        }
        let elapsed = self.elapsed(now);
        self.laps.push(elapsed);
        Ok(elapsed)
    }
}
pub fn format_time(milliseconds: u64) -> String {
    let seconds = milliseconds / 1000;
    format!(
        "{:02}:{:02}:{:02}",
        seconds / 3600,
        (seconds / 60) % 60,
        seconds % 60
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn paused_clock_and_delayed_ticks_preserve_elapsed_time() {
        let mut clock = Clock::timer(60_000).unwrap();
        clock.start(1000);
        clock.pause(16_000);
        assert_eq!(clock.remaining(100_000), 45_000);
        clock.start(100_000);
        assert!(!clock.finish(144_999));
        assert!(clock.finish(200_000));
        assert!(!clock.finish(200_001));
        assert_eq!(clock.remaining(200_000), 0);
        clock.reset();
        assert_eq!(clock.remaining(500_000), 60_000);
    }
    #[test]
    fn stopwatch_laps_use_monotonic_time_and_are_bounded() {
        let mut clock = Clock::default();
        assert!(clock.lap(0).is_err());
        clock.start(100);
        assert_eq!(clock.lap(1100).unwrap(), 1000);
        clock.pause(2100);
        assert_eq!(clock.elapsed(9000), 2000);
        clock.start(9000);
        assert_eq!(clock.elapsed(8500), 2000);
        for _ in 1..1000 {
            clock.lap(10_000).unwrap();
        }
        assert!(clock.lap(10_000).is_err());
        clock.reset();
        assert!(clock.laps.is_empty());
    }
    #[test]
    fn timer_duration_validation_and_formatting() {
        assert!(Clock::timer(0).is_err());
        assert!(Clock::timer(7 * 24 * 60 * 60 * 1000 + 1).is_err());
        assert_eq!(format_time(3_661_999), "01:01:01");
    }
}

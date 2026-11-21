//! Tool: **Sleep Timer** (`sleep-timer`). Fake-clock scheduler; effects happen only after confirmation.
use crate::{
    CoreError, CoreResult,
    platform::{CommandSpec, Os},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PowerAction {
    Sleep,
    Shutdown,
    Restart,
    SignOut,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerEvent {
    None,
    Warning,
    Execute(PowerAction),
}
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimerStatus {
    pub action: PowerAction,
    pub deadline: u64,
}
#[derive(Default)]
pub struct SleepTimer {
    status: Option<TimerStatus>,
    warned: bool,
}
impl SleepTimer {
    pub fn arm(&mut self, now: u64, seconds: u64, action: PowerAction) -> CoreResult<()> {
        if seconds == 0 || seconds > 31 * 24 * 3600 {
            return Err(CoreError::InvalidInput(
                "Choose a delay between 1 second and 31 days.".into(),
            ));
        }
        let deadline = now
            .checked_add(seconds)
            .ok_or_else(|| CoreError::InvalidInput("Delay is too large.".into()))?;
        self.status = Some(TimerStatus { action, deadline });
        self.warned = false;
        Ok(())
    }
    pub fn status(&self) -> Option<TimerStatus> {
        self.status
    }
    /// The caller supplies the same monotonic scheduler clock used when arming and ticking.
    pub fn remaining(&self, now: u64) -> u64 {
        self.status
            .map_or(0, |status| status.deadline.saturating_sub(now))
    }
    pub fn cancel(&mut self) {
        self.status = None;
        self.warned = false;
    }
    pub fn tick(&mut self, now: u64) -> TimerEvent {
        let Some(status) = self.status else {
            return TimerEvent::None;
        };
        if now >= status.deadline {
            self.cancel();
            return TimerEvent::Execute(status.action);
        }
        if status.deadline.saturating_sub(now) <= 60 && !self.warned {
            self.warned = true;
            return TimerEvent::Warning;
        }
        TimerEvent::None
    }
}
pub fn execute(action: PowerAction) -> CoreResult<()> {
    crate::platform::execute_power(action)
}
pub fn command(os: Os, action: PowerAction) -> CoreResult<CommandSpec> {
    use PowerAction::*;
    Ok(match (os, action) {
        (Os::Windows, Sleep) => CommandSpec {
            program: "native:SetSuspendState",
            args: vec![],
        },
        (Os::Windows, Shutdown) => CommandSpec {
            program: "shutdown",
            args: vec!["/s", "/t", "0"],
        },
        (Os::Windows, Restart) => CommandSpec {
            program: "shutdown",
            args: vec!["/r", "/t", "0"],
        },
        (Os::Windows, SignOut) => CommandSpec {
            program: "shutdown",
            args: vec!["/l"],
        },
        (Os::Macos, Sleep) => CommandSpec {
            program: "pmset",
            args: vec!["sleepnow"],
        },
        (Os::Macos, Shutdown) => CommandSpec {
            program: "osascript",
            args: vec!["-e", "tell application \"System Events\" to shut down"],
        },
        (Os::Macos, Restart) => CommandSpec {
            program: "osascript",
            args: vec!["-e", "tell application \"System Events\" to restart"],
        },
        (Os::Macos, SignOut) => CommandSpec {
            program: "osascript",
            args: vec!["-e", "tell application \"System Events\" to log out"],
        },
        (Os::Linux, Sleep) => CommandSpec {
            program: "systemctl",
            args: vec!["suspend"],
        },
        (Os::Linux, Shutdown) => CommandSpec {
            program: "systemctl",
            args: vec!["poweroff"],
        },
        (Os::Linux, Restart) => CommandSpec {
            program: "systemctl",
            args: vec!["reboot"],
        },
        (Os::Linux, SignOut) => CommandSpec {
            program: "loginctl",
            args: vec!["terminate-session", ""],
        },
        (Os::Other, _) => return Err(CoreError::Unsupported(os.display_name())),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::Os;
    #[test]
    fn remaining_time_uses_the_scheduler_clock_and_clears_on_cancel() {
        let mut timer = SleepTimer::default();
        assert_eq!(timer.remaining(100), 0);
        timer.arm(100, 120, PowerAction::Sleep).unwrap();
        assert_eq!(timer.remaining(100), 120);
        assert_eq!(timer.remaining(159), 61);
        assert_eq!(timer.remaining(1000), 0);
        timer.cancel();
        assert_eq!(timer.remaining(159), 0);
    }
    #[test]
    fn cancellation_warning_and_due_use_fake_time() {
        let mut timer = SleepTimer::default();
        timer.arm(100, 120, PowerAction::Sleep).unwrap();
        assert_eq!(timer.tick(159), TimerEvent::None);
        assert_eq!(timer.tick(160), TimerEvent::Warning);
        assert_eq!(timer.tick(161), TimerEvent::None);
        timer.cancel();
        assert_eq!(timer.tick(220), TimerEvent::None);
        timer.arm(100, 2, PowerAction::Sleep).unwrap();
        assert_eq!(timer.tick(102), TimerEvent::Execute(PowerAction::Sleep));
        assert_eq!(timer.tick(103), TimerEvent::None);
    }
    #[test]
    fn rejects_invalid_delays_and_overflow() {
        let mut timer = SleepTimer::default();
        assert!(timer.arm(0, 0, PowerAction::Shutdown).is_err());
        assert!(timer.arm(u64::MAX, 2, PowerAction::Shutdown).is_err());
    }
    #[test]
    fn all_actions_have_explicit_platform_plans() {
        for action in [
            PowerAction::Sleep,
            PowerAction::Shutdown,
            PowerAction::Restart,
            PowerAction::SignOut,
        ] {
            for os in [Os::Windows, Os::Macos, Os::Linux] {
                let spec = command(os, action).unwrap();
                assert!(!spec.program.is_empty());
                assert!(!spec.args.iter().any(|a| a.contains(';')));
            }
        }
        assert!(command(Os::Other, PowerAction::Sleep).is_err());
    }
}

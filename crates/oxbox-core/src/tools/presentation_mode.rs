//! Tool: **Presentation Mode**. Owns independent awake/desktop leases and restores what it changed.
use super::{hide_desktop::DesktopState, keep_awake::AwakeController};
use crate::{CoreError, CoreResult};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresentationOptions {
    pub display: bool,
    pub hide_desktop: bool,
    pub auto_external: bool,
}
impl Default for PresentationOptions {
    fn default() -> Self {
        Self {
            display: true,
            hide_desktop: true,
            auto_external: false,
        }
    }
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresentationStatus {
    pub active: bool,
    pub automatic: bool,
    pub options: PresentationOptions,
    pub auto_suppressed: bool,
    pub notification_control: &'static str,
}
pub trait PresentationBackend {
    fn start_awake(&mut self, display: bool) -> CoreResult<()>;
    fn stop_awake(&mut self) -> CoreResult<()>;
    fn hide(&mut self) -> CoreResult<()>;
    fn restore(&mut self) -> CoreResult<()>;
}
#[derive(Default)]
pub struct NativePresentation {
    awake: Option<AwakeController>,
    desktop: DesktopState,
}
impl PresentationBackend for NativePresentation {
    fn start_awake(&mut self, display: bool) -> CoreResult<()> {
        let controller = AwakeController::new()?;
        controller.start(0, display, None, None)?;
        self.awake = Some(controller);
        Ok(())
    }
    fn stop_awake(&mut self) -> CoreResult<()> {
        if let Some(mut awake) = self.awake.take() {
            awake.shutdown();
        }
        Ok(())
    }
    fn hide(&mut self) -> CoreResult<()> {
        self.desktop.hide()
    }
    fn restore(&mut self) -> CoreResult<()> {
        self.desktop.restore()
    }
}
pub struct PresentationSession<B: PresentationBackend = NativePresentation> {
    backend: B,
    options: PresentationOptions,
    active: bool,
    automatic: bool,
    suppressed: bool,
    desktop_owned: bool,
}
impl Default for PresentationSession {
    fn default() -> Self {
        Self::with_backend(NativePresentation::default())
    }
}
impl<B: PresentationBackend> PresentationSession<B> {
    pub fn with_backend(backend: B) -> Self {
        Self {
            backend,
            options: PresentationOptions::default(),
            active: false,
            automatic: false,
            suppressed: false,
            desktop_owned: false,
        }
    }
    pub fn configure(&mut self, options: PresentationOptions) -> CoreResult<()> {
        if self.active {
            return Err(CoreError::InvalidInput(
                "Stop the current presentation before changing its preferences.".into(),
            ));
        }
        self.options = options;
        self.suppressed = false;
        Ok(())
    }
    pub fn start(&mut self, options: PresentationOptions, automatic: bool) -> CoreResult<()> {
        if self.active {
            return Ok(());
        }
        self.backend.start_awake(options.display)?;
        if options.hide_desktop {
            if let Err(error) = self.backend.hide() {
                let _ = self.backend.stop_awake();
                return Err(error);
            }
            self.desktop_owned = true;
        }
        self.options = options;
        self.active = true;
        self.automatic = automatic;
        Ok(())
    }
    pub fn stop(&mut self, manual: bool) -> CoreResult<()> {
        let mut failure = None;
        if self.desktop_owned {
            match self.backend.restore() {
                Ok(()) => self.desktop_owned = false,
                Err(error) => failure = Some(error),
            }
        }
        // Release independent resources even when restoring desktop icons fails.
        if self.active
            && let Err(error) = self.backend.stop_awake()
        {
            failure.get_or_insert(error);
        }
        if failure.is_none() {
            self.active = false;
            self.automatic = false;
        }
        if manual {
            self.suppressed = true;
        }
        failure.map_or(Ok(()), Err)
    }
    pub fn monitor_count(&mut self, count: usize) -> CoreResult<()> {
        if count <= 1 {
            self.suppressed = false;
            if self.automatic {
                self.stop(false)?;
            }
            return Ok(());
        }
        if self.options.auto_external && !self.active && !self.suppressed {
            self.start(self.options.clone(), true)?;
        }
        Ok(())
    }
    pub fn status(&self) -> PresentationStatus {
        PresentationStatus {
            active: self.active,
            automatic: self.automatic,
            options: self.options.clone(),
            auto_suppressed: self.suppressed,
            notification_control: "manual",
        }
    }
}
impl<B: PresentationBackend> Drop for PresentationSession<B> {
    fn drop(&mut self) {
        let _ = self.stop(false);
    }
}
pub fn connected_displays() -> CoreResult<usize> {
    xcap::Monitor::all()
        .map(|monitors| monitors.len())
        .map_err(|error| CoreError::InvalidInput(format!("Could not enumerate displays: {error}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Default)]
    struct Fake {
        awake: bool,
        hidden: bool,
        previous_hidden: bool,
        starts: u32,
        stops: u32,
        fail_hide: bool,
        fail_restore: bool,
    }
    impl PresentationBackend for Fake {
        fn start_awake(&mut self, _display: bool) -> crate::CoreResult<()> {
            self.awake = true;
            self.starts += 1;
            Ok(())
        }
        fn stop_awake(&mut self) -> crate::CoreResult<()> {
            self.awake = false;
            self.stops += 1;
            Ok(())
        }
        fn hide(&mut self) -> crate::CoreResult<()> {
            if self.fail_hide {
                return Err(crate::CoreError::InvalidInput("fake".into()));
            }
            self.previous_hidden = self.hidden;
            self.hidden = true;
            Ok(())
        }
        fn restore(&mut self) -> crate::CoreResult<()> {
            if self.fail_restore {
                return Err(crate::CoreError::InvalidInput(
                    "fake restore failure".into(),
                ));
            }
            self.hidden = self.previous_hidden;
            Ok(())
        }
    }
    #[test]
    fn manual_session_restores_original_state_and_rolls_back_partial_start() {
        let mut session = PresentationSession::with_backend(Fake {
            hidden: true,
            ..Fake::default()
        });
        session
            .start(PresentationOptions::default(), false)
            .unwrap();
        session.stop(false).unwrap();
        assert!(session.backend.hidden);
        assert!(!session.backend.awake);
        let mut session = PresentationSession::with_backend(Fake {
            fail_hide: true,
            ..Fake::default()
        });
        assert!(
            session
                .start(PresentationOptions::default(), false)
                .is_err()
        );
        assert!(!session.backend.awake);
        assert!(!session.status().active);
    }
    #[test]
    fn auto_external_respects_manual_stop_until_monitor_disconnects() {
        let mut session = PresentationSession::with_backend(Fake::default());
        session
            .configure(PresentationOptions {
                auto_external: true,
                ..PresentationOptions::default()
            })
            .unwrap();
        session.monitor_count(2).unwrap();
        assert!(session.status().automatic);
        session.stop(true).unwrap();
        session.monitor_count(2).unwrap();
        assert!(!session.status().active);
        session.monitor_count(1).unwrap();
        session.monitor_count(2).unwrap();
        assert!(session.status().active);
        session.monitor_count(1).unwrap();
        assert!(!session.status().active);
    }
    #[test]
    fn monitor_disconnect_does_not_stop_a_manual_presentation() {
        let mut session = PresentationSession::with_backend(Fake::default());
        session
            .start(
                PresentationOptions {
                    auto_external: true,
                    ..PresentationOptions::default()
                },
                false,
            )
            .unwrap();
        session.monitor_count(1).unwrap();
        assert!(session.status().active);
        assert!(!session.status().automatic);
        session.stop(true).unwrap();
        assert!(!session.backend.awake);
        assert!(!session.backend.hidden);
    }
    #[test]
    fn failed_desktop_restore_still_releases_awake_and_keeps_restore_retryable() {
        let mut session = PresentationSession::with_backend(Fake {
            fail_restore: true,
            ..Fake::default()
        });
        session
            .start(PresentationOptions::default(), false)
            .unwrap();
        assert!(session.stop(true).is_err());
        assert!(
            !session.backend.awake,
            "a failed icon restore must not retain the awake guard"
        );
        assert!(
            session.status().active,
            "the pending desktop restore must remain retryable"
        );
        session.backend.fail_restore = false;
        session.stop(true).unwrap();
        assert!(!session.status().active);
        assert!(!session.backend.hidden);
    }
}

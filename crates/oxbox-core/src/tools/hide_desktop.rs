//! Tool: **Hide Desktop** (`hide-desktop`). Restores the user's exact original visibility on exit.
use crate::{CoreResult, platform};

/// Win32 handles can be reused after Explorer exits. Restoration must prove the
/// saved handle is still the same desktop list view, rather than merely valid.
#[cfg(any(windows, test))]
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct DesktopWindowIdentity {
    pub handle: isize,
    pub process: u32,
    pub thread: u32,
    pub class_name: String,
    pub is_desktop: bool,
}

#[cfg(any(windows, test))]
pub(crate) fn restore_owned_window(
    saved: &DesktopWindowIdentity,
    current: Option<&DesktopWindowIdentity>,
    restore: impl FnOnce() -> CoreResult<()>,
) -> CoreResult<()> {
    if saved.handle == 0
        || saved.process == 0
        || saved.thread == 0
        || saved.class_name != "SysListView32"
        || !saved.is_desktop
        || current != Some(saved)
    {
        // Explorer's replacement desktop uses its own normal configuration;
        // it is not a lease acquired by this session.
        return Ok(());
    }
    restore()
}

/// Keeps the state machine testable without changing the real user's desktop.
pub trait DesktopBackend {
    type Snapshot;
    fn hide(&self) -> CoreResult<Self::Snapshot>;
    fn restore(&self, snapshot: &Self::Snapshot) -> CoreResult<()>;
}
pub struct NativeDesktop;
impl DesktopBackend for NativeDesktop {
    type Snapshot = platform::DesktopRestore;
    fn hide(&self) -> CoreResult<Self::Snapshot> {
        platform::hide_desktop()
    }
    fn restore(&self, snapshot: &Self::Snapshot) -> CoreResult<()> {
        platform::restore_desktop(snapshot)
    }
}
pub struct DesktopState<B: DesktopBackend = NativeDesktop> {
    backend: B,
    previous: Option<B::Snapshot>,
}
impl Default for DesktopState {
    fn default() -> Self {
        Self::with_backend(NativeDesktop)
    }
}
impl<B: DesktopBackend> DesktopState<B> {
    pub fn with_backend(backend: B) -> Self {
        Self {
            backend,
            previous: None,
        }
    }
    pub fn is_hidden(&self) -> bool {
        self.previous.is_some()
    }
    pub fn hide(&mut self) -> CoreResult<()> {
        if self.previous.is_none() {
            self.previous = Some(self.backend.hide()?);
        }
        Ok(())
    }
    pub fn restore(&mut self) -> CoreResult<()> {
        if let Some(state) = &self.previous {
            self.backend.restore(state)?;
            self.previous = None;
        }
        Ok(())
    }
}
impl<B: DesktopBackend> Drop for DesktopState<B> {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[test]
    fn restore_validates_desktop_window_identity_before_mutating_a_handle() {
        let saved = DesktopWindowIdentity {
            handle: 12,
            process: 30,
            thread: 40,
            class_name: "SysListView32".into(),
            is_desktop: true,
        };
        let mut calls = 0;
        restore_owned_window(&saved, Some(&saved), || {
            calls += 1;
            Ok(())
        })
        .unwrap();
        assert_eq!(calls, 1, "the still-owned desktop must remain restorable");
        for current in [
            None,
            Some(DesktopWindowIdentity {
                handle: 13,
                ..saved.clone()
            }),
            Some(DesktopWindowIdentity {
                process: 31,
                ..saved.clone()
            }),
            Some(DesktopWindowIdentity {
                thread: 41,
                ..saved.clone()
            }),
            Some(DesktopWindowIdentity {
                class_name: "Unrelated window".into(),
                ..saved.clone()
            }),
            Some(DesktopWindowIdentity {
                is_desktop: false,
                ..saved.clone()
            }),
        ] {
            restore_owned_window(&saved, current.as_ref(), || {
                panic!("a stale/reused/unrelated handle must never be mutated");
            })
            .unwrap();
        }
        for invalid in [
            DesktopWindowIdentity {
                handle: 0,
                ..saved.clone()
            },
            DesktopWindowIdentity {
                process: 0,
                ..saved.clone()
            },
            DesktopWindowIdentity {
                thread: 0,
                ..saved.clone()
            },
            DesktopWindowIdentity {
                class_name: "Other class".into(),
                ..saved.clone()
            },
            DesktopWindowIdentity {
                is_desktop: false,
                ..saved.clone()
            },
        ] {
            restore_owned_window(&invalid, Some(&invalid), || {
                panic!("an unproven original desktop identity must not authorize restoration");
            })
            .unwrap();
        }
        assert!(
            restore_owned_window(&saved, Some(&saved), || {
                Err(crate::CoreError::InvalidInput(
                    "generated restore refusal".into(),
                ))
            })
            .is_err()
        );
    }

    #[derive(Default)]
    struct FakeState {
        visible: bool,
        writes: usize,
        fail_restore: bool,
    }
    struct FakeDesktop(Arc<Mutex<FakeState>>);
    impl DesktopBackend for FakeDesktop {
        type Snapshot = bool;
        fn hide(&self) -> CoreResult<bool> {
            let mut state = self.0.lock().unwrap();
            let previous = state.visible;
            state.visible = false;
            state.writes += 1;
            Ok(previous)
        }
        fn restore(&self, previous: &bool) -> CoreResult<()> {
            let mut state = self.0.lock().unwrap();
            if state.fail_restore {
                return Err(crate::CoreError::InvalidInput(
                    "fake transient error".into(),
                ));
            }
            state.visible = *previous;
            state.writes += 1;
            Ok(())
        }
    }
    #[test]
    fn preserves_both_original_visibility_states_and_restores_on_drop() {
        for visible in [true, false] {
            let state = Arc::new(Mutex::new(FakeState {
                visible,
                ..FakeState::default()
            }));
            {
                let mut desktop = DesktopState::with_backend(FakeDesktop(state.clone()));
                desktop.hide().unwrap();
                desktop.hide().unwrap();
                assert!(!state.lock().unwrap().visible);
                assert_eq!(state.lock().unwrap().writes, 1);
            }
            assert_eq!(state.lock().unwrap().visible, visible);
            assert_eq!(state.lock().unwrap().writes, 2);
        }
    }
    #[test]
    fn failed_restore_keeps_original_state_for_retry() {
        let state = Arc::new(Mutex::new(FakeState {
            visible: true,
            ..FakeState::default()
        }));
        let mut desktop = DesktopState::with_backend(FakeDesktop(state.clone()));
        desktop.hide().unwrap();
        state.lock().unwrap().fail_restore = true;
        assert!(desktop.restore().is_err());
        assert!(desktop.is_hidden());
        state.lock().unwrap().fail_restore = false;
        desktop.restore().unwrap();
        desktop.restore().unwrap();
        assert!(state.lock().unwrap().visible);
        assert!(!desktop.is_hidden());
        assert_eq!(state.lock().unwrap().writes, 2);
    }
    #[test]
    fn restore_state_is_idempotent_without_platform_effects() {
        let mut s = DesktopState::default();
        assert!(!s.is_hidden());
        // No previous hide means restore must never change the user's desktop.
        s.restore().unwrap();
        s.restore().unwrap();
        assert!(!s.is_hidden());
    }
}

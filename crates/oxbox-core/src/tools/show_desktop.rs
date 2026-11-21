//! Minimize selected normal application windows without closing them.
use crate::{CoreError, CoreResult};
pub fn validate_process(pid: u32, our_pid: u32) -> CoreResult<()> {
    if pid == 0 || pid == our_pid {
        return Err(CoreError::InvalidInput(
            "Oxbox and system windows are excluded".into(),
        ));
    }
    Ok(())
}
pub fn minimize() -> CoreResult<Vec<(u64, u32)>> {
    Ok(minimize_with(super::window_top::windows()?, &mut |id| {
        crate::platform::desktop::minimize_window(id)
    }))
}
fn minimize_with(
    windows: Vec<super::window_top::WindowInfo>,
    effect: &mut dyn FnMut(u64) -> CoreResult<()>,
) -> Vec<(u64, u32)> {
    let mut changed = Vec::new();
    for window in windows {
        if !window.minimized && validate_process(window.pid, std::process::id()).is_ok() {
            // Individual windows can disappear or refuse the request. Retain
            // every successful change so the caller can restore that group.
            if effect(window.id).is_ok() {
                changed.push((window.id, window.pid));
            }
        }
    }
    changed
}
pub fn restore(windows: &[(u64, u32)]) -> CoreResult<usize> {
    Ok(restore_tracked(windows).restored)
}
pub struct RestoreOutcome {
    pub restored: usize,
    pub remaining: Vec<(u64, u32)>,
}
pub fn restore_tracked(windows: &[(u64, u32)]) -> RestoreOutcome {
    restore_with(
        windows,
        &mut super::window_top::window,
        &mut crate::platform::desktop::restore_window,
    )
}
fn restore_with(
    windows: &[(u64, u32)],
    get: &mut dyn FnMut(u64, u32) -> CoreResult<super::window_top::WindowInfo>,
    effect: &mut dyn FnMut(u64) -> CoreResult<()>,
) -> RestoreOutcome {
    let mut outcome = RestoreOutcome {
        restored: 0,
        remaining: Vec::new(),
    };
    for &(id, pid) in windows {
        if get(id, pid).is_ok_and(|window| window.pid == pid && window.minimized) {
            if effect(id).is_ok() {
                outcome.restored += 1;
            } else {
                outcome.remaining.push((id, pid));
            }
        }
    }
    outcome
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn excludes_our_process_and_invalid_selection() {
        assert!(validate_process(1, 1).is_err());
        assert!(validate_process(0, 1).is_err());
        assert!(validate_process(2, 1).is_ok());
    }

    #[test]
    fn later_failures_do_not_lose_earlier_minimized_windows() {
        use super::super::{window_manager::Rect, window_top::WindowInfo};
        let windows = (1..=3)
            .map(|id| WindowInfo {
                id,
                pid: u32::MAX,
                title: format!("Window {id}"),
                bounds: Rect {
                    x: 0,
                    y: 0,
                    width: 100,
                    height: 100,
                },
                topmost: false,
                minimized: false,
            })
            .collect();
        let changed = minimize_with(windows, &mut |id| {
            if id == 2 {
                Err(crate::CoreError::InvalidInput("window disappeared".into()))
            } else {
                Ok(())
            }
        });
        assert_eq!(changed, vec![(1, u32::MAX), (3, u32::MAX)]);
    }

    #[test]
    fn restore_skips_user_restored_windows_and_retains_only_refused_requests() {
        use super::super::{window_manager::Rect, window_top::WindowInfo};
        let mut requested = Vec::new();
        let outcome = restore_with(
            &[(1, 100), (2, 100), (3, 100)],
            &mut |id, pid| {
                Ok(WindowInfo {
                    id,
                    pid,
                    title: "Window".into(),
                    bounds: Rect {
                        x: 0,
                        y: 0,
                        width: 100,
                        height: 100,
                    },
                    topmost: false,
                    minimized: id != 2,
                })
            },
            &mut |id| {
                requested.push(id);
                if id == 3 {
                    Err(crate::CoreError::InvalidInput("refused".into()))
                } else {
                    Ok(())
                }
            },
        );
        assert_eq!(requested, vec![1, 3]);
        assert_eq!(outcome.restored, 1);
        assert_eq!(outcome.remaining, vec![(3, 100)]);
    }
}

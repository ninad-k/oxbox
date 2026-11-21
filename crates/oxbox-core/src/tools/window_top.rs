//! Keep a selected application window above other windows.
use super::window_manager::Rect;
use crate::{CoreError, CoreResult};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowInfo {
    pub id: u64,
    pub pid: u32,
    pub title: String,
    pub bounds: Rect,
    pub topmost: bool,
    pub minimized: bool,
}

pub fn validate_target(id: u64, expected_pid: u32, current_pid: u32) -> CoreResult<()> {
    if id == 0 || expected_pid == 0 || current_pid != expected_pid {
        return Err(CoreError::InvalidInput(
            "the selected window has closed or changed; refresh the list".into(),
        ));
    }
    Ok(())
}
pub fn windows() -> CoreResult<Vec<WindowInfo>> {
    crate::platform::desktop::windows()
}
pub fn window(id: u64, pid: u32) -> CoreResult<WindowInfo> {
    let current = crate::platform::desktop::window(id)?;
    validate_target(id, pid, current.pid)?;
    Ok(current)
}
pub fn set(id: u64, pid: u32, enabled: bool) -> CoreResult<()> {
    window(id, pid)?;
    crate::platform::desktop::set_topmost(id, enabled)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_invalid_or_reused_window_identifiers() {
        assert!(validate_target(0, 10, 10).is_err());
        assert!(validate_target(1, 10, 11).is_err());
        assert!(validate_target(1, 10, 10).is_ok());
    }
}

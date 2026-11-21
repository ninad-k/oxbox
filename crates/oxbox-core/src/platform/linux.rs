use crate::tools::sleep_timer::PowerAction;
use crate::{CoreError, CoreResult};
pub fn lock_session() -> CoreResult<()> {
    super::run_command(&crate::tools::lock::command(super::Os::Linux)?)
}
pub fn execute_power(action: PowerAction) -> CoreResult<()> {
    super::run_command(&crate::tools::sleep_timer::command(
        super::Os::Linux,
        action,
    )?)
}
pub struct DesktopRestore;
pub fn hide_desktop() -> CoreResult<DesktopRestore> {
    Err(CoreError::Unsupported("Linux"))
}
pub fn restore_desktop(_state: &DesktopRestore) -> CoreResult<()> {
    Ok(())
}
pub fn activity_paused(_pause_fullscreen: bool) -> bool {
    false
}

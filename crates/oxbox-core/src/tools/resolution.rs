//! Driver-enumerated display modes and timed rollback decisions.
use crate::{CoreError, CoreResult};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub struct DisplayMode {
    pub device: String,
    pub width: u32,
    pub height: u32,
    pub refresh: u32,
    pub bits_per_pixel: u32,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Display {
    pub device: String,
    pub name: String,
    pub current: DisplayMode,
    pub modes: Vec<DisplayMode>,
}
pub fn validate_mode(mode: &DisplayMode) -> CoreResult<()> {
    if mode.device.is_empty()
        || mode.device.len() > 100
        || mode.device.contains('\0')
        || !(320..=32768).contains(&mode.width)
        || !(200..=32768).contains(&mode.height)
        || mode.refresh > 1000
        || mode.bits_per_pixel != 32
    {
        return Err(CoreError::InvalidInput(
            "choose an available 32-bit display mode".into(),
        ));
    }
    Ok(())
}
pub fn rollback_deadline(now: u64) -> CoreResult<u64> {
    now.checked_add(15)
        .ok_or_else(|| CoreError::InvalidInput("display rollback deadline overflow".into()))
}
pub fn displays() -> CoreResult<Vec<Display>> {
    crate::platform::desktop::displays()
}
pub fn apply(mode: &DisplayMode) -> CoreResult<()> {
    validate_mode(mode)?;
    let display = displays()?
        .into_iter()
        .find(|d| d.device == mode.device)
        .ok_or_else(|| CoreError::InvalidInput("the display is no longer connected".into()))?;
    if !display.modes.contains(mode) {
        return Err(CoreError::InvalidInput(
            "the selected mode is no longer available".into(),
        ));
    }
    crate::platform::desktop::set_resolution(mode)
}
pub fn restore(mode: &DisplayMode) -> CoreResult<()> {
    validate_mode(mode)?;
    crate::platform::desktop::set_resolution(mode)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_display_modes_and_rollback_deadlines() {
        let mode = DisplayMode {
            device: "DISPLAY1".into(),
            width: 1920,
            height: 1080,
            refresh: 60,
            bits_per_pixel: 32,
        };
        assert!(validate_mode(&mode).is_ok());
        let mut bad = mode.clone();
        bad.width = 0;
        assert!(validate_mode(&bad).is_err());
        assert_eq!(rollback_deadline(100).unwrap(), 115);
        assert!(rollback_deadline(u64::MAX).is_err());
    }
}

//! User-confirmed safe ejection of removable volumes.
use crate::{CoreError, CoreResult};
use serde::Serialize;
#[derive(Debug, Clone, Serialize)]
pub struct Volume {
    pub root: String,
    pub label: String,
}
pub fn validate_drive(root: &str, system_drive: &str) -> CoreResult<char> {
    let bytes = root.as_bytes();
    if bytes.len() != 3
        || !bytes[0].is_ascii_alphabetic()
        || bytes[1] != b':'
        || bytes[2] != b'\\'
        || root[..2].eq_ignore_ascii_case(system_drive)
    {
        return Err(CoreError::InvalidInput(
            "choose a removable drive other than the system drive".into(),
        ));
    }
    Ok((bytes[0] as char).to_ascii_uppercase())
}
pub fn volumes() -> CoreResult<Vec<Volume>> {
    crate::platform::desktop::volumes()
}
pub fn eject(root: &str, confirmed: bool) -> CoreResult<()> {
    if !confirmed {
        return Err(CoreError::InvalidInput(
            "confirm the selected volume before ejection".into(),
        ));
    }
    let system = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into());
    validate_drive(root, &system)?;
    if !volumes()?.iter().any(|v| v.root.eq_ignore_ascii_case(root)) {
        return Err(CoreError::InvalidInput(
            "this drive is no longer removable".into(),
        ));
    }
    crate::platform::desktop::eject(root)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn drive_paths_cannot_select_system_or_arbitrary_device_names() {
        assert_eq!(validate_drive("E:\\", "C:").unwrap(), 'E');
        for path in ["C:\\", "E:\\folder", "\\\\.\\PhysicalDrive0", "/", ""] {
            assert!(validate_drive(path, "C:").is_err(), "{path}");
        }
    }
}

//! Tool: **Do Not Disturb**. Uses user-owned OS controls rather than undocumented registry writes.
use crate::{
    CoreError, CoreResult,
    platform::{CommandSpec, Os},
};
use serde::Serialize;
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ControlCapability {
    pub available: bool,
    pub automatic_toggle: bool,
    pub state: &'static str,
    pub settings_uri: Option<&'static str>,
    pub message: &'static str,
    pub action_label: &'static str,
}
pub fn capability(os: Os) -> ControlCapability {
    ControlCapability {
        available: matches!(os, Os::Windows | Os::Macos),
        automatic_toggle: false,
        state: "unknown",
        settings_uri: if os == Os::Windows {
            Some("ms-settings:notifications")
        } else {
            None
        },
        message: match os {
            Os::Windows => {
                "Open Notifications and turn Do Not Disturb on or off there. Windows 10 calls this Focus assist. Oxbox does not change or read this setting."
            }
            Os::Macos => {
                "Open System Settings (System Preferences on macOS 12), then choose Focus and enable Do Not Disturb. Oxbox does not change or read the Focus state."
            }
            _ => {
                "Use your desktop environment's notification controls. A portable Do Not Disturb setting is unavailable on this platform."
            }
        },
        action_label: if os == Os::Windows {
            "Open notification settings"
        } else {
            "Open system settings"
        },
    }
}
pub fn settings_command(os: Os) -> CoreResult<CommandSpec> {
    match os {
        Os::Windows => Ok(CommandSpec {
            program: "explorer.exe",
            args: vec!["ms-settings:notifications"],
        }),
        Os::Macos => Ok(CommandSpec {
            program: "open",
            args: vec!["-b", "com.apple.systempreferences"],
        }),
        _ => Err(CoreError::Unsupported(os.display_name())),
    }
}
pub fn open_settings() -> CoreResult<()> {
    crate::platform::run_command(&settings_command(Os::current())?)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn windows_controls_use_documented_local_uri_and_unknown_state() {
        let capability = capability(crate::platform::Os::Windows);
        assert_eq!(capability.settings_uri, Some("ms-settings:notifications"));
        assert!(!capability.automatic_toggle);
        assert_eq!(capability.state, "unknown");
        assert!(settings_command(crate::platform::Os::Other).is_err());
    }
}

//! Tool: **Airplane Mode**. Opens documented Windows radio controls; never silently disables connectivity.
use super::do_not_disturb::ControlCapability;
use crate::{
    CoreError, CoreResult,
    platform::{CommandSpec, Os},
};
pub fn capability(os: Os) -> ControlCapability {
    ControlCapability {
        available: os == Os::Windows,
        automatic_toggle: false,
        state: "unknown",
        settings_uri: if os == Os::Windows {
            Some("ms-settings:network-airplanemode")
        } else {
            None
        },
        message: if os == Os::Windows {
            "Open Windows Airplane mode settings and choose which radios to disable. Availability depends on your hardware. Oxbox does not read or change radio state."
        } else {
            "This operating system does not expose a supported system-wide airplane mode control here. Use its Wi-Fi and Bluetooth controls."
        },
        action_label: "Open airplane mode settings",
    }
}
pub fn settings_command(os: Os) -> CoreResult<CommandSpec> {
    if os != Os::Windows {
        return Err(CoreError::Unsupported(os.display_name()));
    }
    Ok(CommandSpec {
        program: "explorer.exe",
        args: vec!["ms-settings:network-airplanemode"],
    })
}
pub fn open_settings() -> CoreResult<()> {
    crate::platform::run_command(&settings_command(Os::current())?)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn radio_controls_never_claim_to_toggle_radios() {
        let capability = capability(crate::platform::Os::Windows);
        assert_eq!(
            capability.settings_uri,
            Some("ms-settings:network-airplanemode")
        );
        assert!(!capability.automatic_toggle);
        assert_eq!(capability.state, "unknown");
        assert!(settings_command(crate::platform::Os::Macos).is_err());
    }
}

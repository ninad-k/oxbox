use crate::CoreResult;
use crate::tools::sleep_timer::PowerAction;
pub fn lock_session() -> CoreResult<()> {
    super::run_command(&crate::tools::lock::command(super::Os::Macos)?)
}
pub fn execute_power(action: PowerAction) -> CoreResult<()> {
    super::run_command(&crate::tools::sleep_timer::command(
        super::Os::Macos,
        action,
    )?)
}
pub struct DesktopRestore {
    previous: Option<bool>,
}
pub fn hide_desktop() -> CoreResult<DesktopRestore> {
    let value = std::process::Command::new("defaults")
        .args(["read", "com.apple.finder", "CreateDesktop"])
        .output()?;
    let previous = if value.status.success() {
        Some(String::from_utf8_lossy(&value.stdout).trim() != "0")
    } else {
        None
    };
    super::run_command(&super::CommandSpec {
        program: "defaults",
        args: vec![
            "write",
            "com.apple.finder",
            "CreateDesktop",
            "-bool",
            "false",
        ],
    })?;
    if let Err(e) = super::run_command(&super::CommandSpec {
        program: "killall",
        args: vec!["Finder"],
    }) {
        let state = DesktopRestore { previous };
        let _ = restore_desktop(&state);
        return Err(e);
    }
    Ok(DesktopRestore { previous })
}
pub fn restore_desktop(state: &DesktopRestore) -> CoreResult<()> {
    let args = match state.previous {
        Some(true) => vec![
            "write",
            "com.apple.finder",
            "CreateDesktop",
            "-bool",
            "true",
        ],
        Some(false) => vec![
            "write",
            "com.apple.finder",
            "CreateDesktop",
            "-bool",
            "false",
        ],
        None => vec!["delete", "com.apple.finder", "CreateDesktop"],
    };
    super::run_command(&super::CommandSpec {
        program: "defaults",
        args,
    })?;
    super::run_command(&super::CommandSpec {
        program: "killall",
        args: vec!["Finder"],
    })
}
pub fn activity_paused(_pause_fullscreen: bool) -> bool {
    false
}

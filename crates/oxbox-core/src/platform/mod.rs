//! OS detection and the home for OS-specific code.
//!
//! When a tool needs an OS API, add a function to `windows.rs`, `macos.rs` and `linux.rs`
//! (create them as needed) behind `#[cfg(target_os = "...")]`, and return
//! [`crate::CoreError::Unsupported`] on platforms you cannot support yet.

use serde::Serialize;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
pub use linux::{
    DesktopRestore, activity_paused, execute_power, hide_desktop, lock_session, restore_desktop,
};
#[cfg(target_os = "macos")]
pub use macos::{
    DesktopRestore, activity_paused, execute_power, hide_desktop, lock_session, restore_desktop,
};
#[cfg(target_os = "windows")]
pub use windows::{
    DesktopRestore, activity_paused, execute_power, hide_desktop, lock_session, restore_desktop,
};

/// An explicit executable and argument vector; never interpreted by a shell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSpec {
    pub program: &'static str,
    pub args: Vec<&'static str>,
}

pub(crate) fn run_command(spec: &CommandSpec) -> crate::CoreResult<()> {
    let result = std::process::Command::new(spec.program)
        .args(&spec.args)
        .output()?;
    if !result.status.success() {
        return Err(crate::CoreError::InvalidInput(format!(
            "{} was refused by the operating system ({}).",
            spec.program, result.status
        )));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Os {
    Windows,
    Macos,
    Linux,
    Other,
}

impl Os {
    pub fn current() -> Self {
        if cfg!(target_os = "windows") {
            Os::Windows
        } else if cfg!(target_os = "macos") {
            Os::Macos
        } else if cfg!(target_os = "linux") {
            Os::Linux
        } else {
            Os::Other
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Os::Windows => "Windows",
            Os::Macos => "macOS",
            Os::Linux => "Linux",
            Os::Other => "this platform",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_os_matches_compile_target() {
        let os = Os::current();
        #[cfg(target_os = "windows")]
        assert_eq!(os, Os::Windows);
        #[cfg(target_os = "macos")]
        assert_eq!(os, Os::Macos);
        #[cfg(target_os = "linux")]
        assert_eq!(os, Os::Linux);
        assert!(!os.display_name().is_empty());
    }
}
pub mod desktop;

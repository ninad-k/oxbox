//! OS detection and the home for OS-specific code.
//!
//! When a tool needs an OS API, add a function to `windows.rs`, `macos.rs` and `linux.rs`
//! (create them as needed) behind `#[cfg(target_os = "...")]`, and return
//! [`crate::CoreError::Unsupported`] on platforms you cannot support yet.

use serde::Serialize;

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

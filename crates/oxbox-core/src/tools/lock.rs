//! Tool: **Lock Screen** (`lock-screen`).
use crate::{
    CoreError, CoreResult,
    platform::{CommandSpec, Os},
};
pub fn command(os: Os) -> CoreResult<CommandSpec> {
    Ok(match os {
        Os::Windows => CommandSpec {
            program: "native:LockWorkStation",
            args: vec![],
        },
        Os::Macos => CommandSpec {
            program: "pmset",
            args: vec!["displaysleepnow"],
        },
        Os::Linux => CommandSpec {
            program: "loginctl",
            args: vec!["lock-session"],
        },
        Os::Other => return Err(CoreError::Unsupported(os.display_name())),
    })
}
pub fn lock() -> CoreResult<()> {
    crate::platform::lock_session()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::Os;
    #[test]
    fn command_table_uses_literal_arguments() {
        assert_eq!(
            command(Os::Windows).unwrap().program,
            "native:LockWorkStation"
        );
        assert_eq!(command(Os::Macos).unwrap().args, ["displaysleepnow"]);
        assert_eq!(command(Os::Linux).unwrap().args, ["lock-session"]);
        assert!(command(Os::Other).is_err());
    }
}

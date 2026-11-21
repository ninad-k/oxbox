//! Show or hide ordinary hidden files in the system file manager.
use crate::CoreResult;
pub fn update_shell_flags(flags: i32, show: bool) -> i32 {
    if show { flags | 1 } else { flags & !1 }
}
pub fn status() -> CoreResult<bool> {
    crate::platform::desktop::hidden_files(None)
}
pub fn set(show: bool) -> CoreResult<bool> {
    crate::platform::desktop::hidden_files(Some(show))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn changes_only_show_all_objects_flag() {
        let original = 0b101010;
        assert_eq!(update_shell_flags(original, true), 0b101011);
        assert_eq!(update_shell_flags(original, false), original);
    }
}

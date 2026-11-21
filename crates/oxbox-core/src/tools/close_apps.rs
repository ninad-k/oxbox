//! Request normal window closure; never terminate processes.
use crate::{CoreError, CoreResult};
use std::collections::HashSet;
pub fn validate_selection(targets: &[(u64, u32)], confirmed: bool) -> CoreResult<()> {
    let mut ids = HashSet::new();
    if !confirmed
        || targets.is_empty()
        || targets.len() > 100
        || targets
            .iter()
            .any(|&(id, pid)| id == 0 || pid == 0 || !ids.insert(id))
    {
        return Err(CoreError::InvalidInput(
            "confirm 1–100 distinct application windows".into(),
        ));
    }
    Ok(())
}
pub fn close(targets: &[(u64, u32)], confirmed: bool) -> CoreResult<usize> {
    validate_selection(targets, confirmed)?;
    for &(id, pid) in targets {
        super::window_top::window(id, pid)?;
        super::show_desktop::validate_process(pid, std::process::id())?;
    }
    let mut count = 0;
    for &(id, _) in targets {
        crate::platform::desktop::close_window(id)?;
        count += 1;
    }
    Ok(count)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn confirmation_and_bounded_unique_selection_are_required() {
        assert!(validate_selection(&[(1, 1)], false).is_err());
        assert!(validate_selection(&[], true).is_err());
        assert!(validate_selection(&[(1, 1), (1, 1)], true).is_err());
        assert!(validate_selection(&[(1, 1), (2, 1)], true).is_ok());
    }
}

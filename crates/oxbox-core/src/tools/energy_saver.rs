//! Select an installed power profile using literal command arguments.
use crate::{CoreError, CoreResult};
use serde::Serialize;
#[derive(Debug, Clone, Serialize)]
pub struct PowerProfile {
    pub id: String,
    pub name: String,
    pub active: bool,
}
pub fn validate_profile(id: &str) -> CoreResult<()> {
    let b = id.as_bytes();
    if b.len() != 36
        || b.iter().enumerate().any(|(i, c)| {
            if [8, 13, 18, 23].contains(&i) {
                *c != b'-'
            } else {
                !c.is_ascii_hexdigit()
            }
        })
    {
        return Err(CoreError::InvalidInput(
            "choose an installed power profile".into(),
        ));
    }
    Ok(())
}
pub fn parse_profiles(text: &str) -> Vec<PowerProfile> {
    text.lines()
        .filter_map(|line| {
            let offset = line
                .as_bytes()
                .windows(36)
                .position(|b| std::str::from_utf8(b).is_ok_and(|s| validate_profile(s).is_ok()))?;
            let id = line[offset..offset + 36].to_ascii_lowercase();
            let name = line[offset + 36..]
                .trim()
                .trim_end_matches('*')
                .trim()
                .trim_start_matches('(')
                .trim_end_matches(')')
                .trim()
                .to_owned();
            Some(PowerProfile {
                id,
                name,
                active: line.contains('*'),
            })
        })
        .collect()
}
pub fn profiles() -> CoreResult<Vec<PowerProfile>> {
    crate::platform::desktop::power_profiles()
}
pub fn set(id: &str, confirmed: bool) -> CoreResult<()> {
    validate_profile(id)?;
    if !confirmed || !profiles()?.iter().any(|p| p.id.eq_ignore_ascii_case(id)) {
        return Err(CoreError::InvalidInput(
            "confirm an installed power profile".into(),
        ));
    }
    crate::platform::desktop::set_power_profile(id)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn guid_parser_is_localization_independent_and_validates_ids() {
        let id = "381b4222-f694-41f0-9685-ff5bb260df2e";
        let plans = parse_profiles(&format!("Energieschema-GUID: {id} (Balanced) *"));
        assert_eq!(plans.len(), 1);
        assert!(plans[0].active);
        assert_eq!(plans[0].id, id);
        assert!(validate_profile("x & shutdown").is_err());
        assert!(validate_profile(id).is_ok());
    }
}

//! Shared native and website catalogue, search and persisted launcher preferences.
use crate::{CoreError, CoreResult};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
#[derive(Debug, Clone, Deserialize)]
pub struct Category {
    pub id: String,
    pub label: String,
}
#[derive(Debug, Clone, Deserialize)]
pub struct Tool {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub keywords: Vec<String>,
    pub platforms: Vec<String>,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub icon: String,
}
impl Tool {
    pub fn supported(&self) -> bool {
        let os = match crate::platform::Os::current() {
            crate::platform::Os::Windows => "windows",
            crate::platform::Os::Macos => "macos",
            crate::platform::Os::Linux => "linux",
            crate::platform::Os::Other => "other",
        };
        self.platforms.iter().any(|value| value == os)
    }
}
#[derive(Debug, Clone, Deserialize)]
pub struct Catalog {
    pub categories: Vec<Category>,
    pub tools: Vec<Tool>,
}
impl Catalog {
    pub fn bundled() -> CoreResult<Self> {
        let value: Self = serde_json::from_str(include_str!("../../../catalog/tools.json"))
            .map_err(|_| CoreError::InvalidInput("The bundled tool catalogue is invalid".into()))?;
        let mut ids = HashSet::new();
        if value.tools.is_empty()
            || value.tools.iter().any(|tool| {
                tool.id.is_empty()
                    || !ids.insert(&tool.id)
                    || tool.platforms.is_empty()
                    || !value
                        .categories
                        .iter()
                        .any(|category| category.id == tool.category)
            })
        {
            return Err(CoreError::InvalidInput(
                "The bundled tool catalogue contains invalid entries".into(),
            ));
        }
        Ok(value)
    }
    pub fn search<'a>(&'a self, view: &str, query: &str, favorites: &[String]) -> Vec<&'a Tool> {
        let query = query.to_lowercase();
        let tokens: Vec<_> = query.split_whitespace().collect();
        if tokens.is_empty() {
            return match view {
                "all" => self.tools.iter().collect(),
                "favorites" => favorites
                    .iter()
                    .filter_map(|id| self.tools.iter().find(|tool| &tool.id == id))
                    .collect(),
                _ => self
                    .tools
                    .iter()
                    .filter(|tool| tool.category == view)
                    .collect(),
            };
        }
        let mut found: Vec<_> = self
            .tools
            .iter()
            .filter(|tool| {
                let category = self
                    .categories
                    .iter()
                    .find(|category| category.id == tool.category)
                    .map(|category| category.label.as_str())
                    .unwrap_or("");
                let text = format!(
                    "{} {} {} {}",
                    tool.name,
                    tool.description,
                    tool.keywords.join(" "),
                    category
                )
                .to_lowercase();
                tokens.iter().all(|token| text.contains(token))
            })
            .collect();
        found.sort_by_key(|tool| {
            let name = tool.name.to_lowercase();
            if name.starts_with(tokens[0]) {
                0
            } else if name.contains(tokens[0]) {
                1
            } else {
                2
            }
        });
        found
    }
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Preferences {
    pub theme: Theme,
    pub favorites: Vec<String>,
    pub world_zones: Vec<String>,
    pub shortcut: String,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            theme: Theme::System,
            favorites: Vec::new(),
            world_zones: ["UTC", "Asia/Dubai", "Europe/London", "America/New_York"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
            shortcut: "CommandOrControl+Alt+O".into(),
        }
    }
}
impl Preferences {
    pub fn from_legacy(value: &serde_json::Value) -> Self {
        let mut result = Self::default();
        if let Some(preferences) = value.get("preferences") {
            result.theme = match preferences.get("theme").and_then(|value| value.as_str()) {
                Some("light") => Theme::Light,
                Some("dark") => Theme::Dark,
                _ => Theme::System,
            };
            if let Some(favorites) = preferences
                .get("favorites")
                .and_then(|value| value.as_array())
            {
                result.favorites = favorites
                    .iter()
                    .filter_map(|value| value.as_str().map(str::to_owned))
                    .collect();
            }
            if let Some(zones) = preferences
                .get("worldZones")
                .and_then(|value| value.as_array())
            {
                result.world_zones = zones
                    .iter()
                    .filter_map(|value| value.as_str().map(str::to_owned))
                    .collect();
            }
        }
        if let Some(shortcut) = value
            .get("shortcut")
            .and_then(|value| value.as_str())
            .filter(|value| !value.trim().is_empty() && value.len() <= 100)
        {
            result.shortcut = shortcut.into();
        }
        result.sanitize();
        result
    }
    pub fn sanitize(&mut self) {
        for list in [&mut self.favorites, &mut self.world_zones] {
            let mut seen = HashSet::new();
            list.retain(|value| {
                !value.is_empty() && value.len() < 150 && seen.insert(value.clone())
            });
            list.truncate(200);
        }
        if self.shortcut.trim().is_empty() || self.shortcut.len() > 100 {
            self.shortcut = Self::default().shortcut;
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn registry_preserves_original_tools_and_adds_maintenance_routes() {
        let catalogue = Catalog::bundled().unwrap();
        let baseline: Vec<String> =
            serde_json::from_str(include_str!("../../../catalog/v01-tool-ids.json")).unwrap();
        assert_eq!(baseline.len(), 49);
        assert!(
            baseline
                .iter()
                .all(|id| catalogue.tools.iter().any(|tool| &tool.id == id))
        );
        assert_eq!(catalogue.tools.len(), 56);
        assert_eq!(catalogue.categories.len(), 8);
        assert_eq!(
            catalogue
                .tools
                .iter()
                .map(|tool| &tool.id)
                .collect::<std::collections::HashSet<_>>()
                .len(),
            56
        );
        assert!(
            catalogue
                .tools
                .iter()
                .all(|tool| !tool.platforms.is_empty())
        );
        assert_eq!(
            catalogue
                .tools
                .iter()
                .find(|tool| tool.id == "mouse-activity")
                .unwrap()
                .platforms,
            vec!["windows"]
        );
    }
    #[test]
    fn dashboard_only_shows_favorites_but_search_reaches_every_tool() {
        let catalogue = Catalog::bundled().unwrap();
        let favorites = vec!["qr-code".into(), "unit-converter".into()];
        assert_eq!(
            catalogue
                .search("favorites", "", &favorites)
                .iter()
                .map(|tool| tool.id.as_str())
                .collect::<Vec<_>>(),
            vec!["qr-code", "unit-converter"]
        );
        assert!(catalogue.search("favorites", "", &[]).is_empty());
        assert_eq!(
            catalogue.search("favorites", "checksum", &[])[0].id,
            "verify-checksum"
        );
        assert!(catalogue.search("all", "no-such-tool", &[]).is_empty());
    }
    #[test]
    fn legacy_settings_preserve_order_and_do_not_reenable_background_effects() {
        let value = serde_json::json!({"shortcut":"Control+Alt+O","preferences":{"theme":"light","favorites":["qr-code","qr-code","unit-converter"],"worldZones":["UTC","Asia/Dubai"]},"roadmapTime":{"alarms":[]}});
        let prefs = Preferences::from_legacy(&value);
        assert_eq!(prefs.theme, Theme::Light);
        assert_eq!(prefs.favorites, vec!["qr-code", "unit-converter"]);
        assert_eq!(prefs.shortcut, "Control+Alt+O");
        assert_eq!(prefs.world_zones, vec!["UTC", "Asia/Dubai"]);
        assert!(!serde_json::to_string(&prefs).unwrap().contains("mouse"));
        assert_eq!(
            Preferences::from_legacy(&serde_json::Value::Null).theme,
            Theme::System
        );
    }
    #[test]
    fn an_invalid_legacy_field_does_not_discard_other_valid_preferences() {
        let value = serde_json::json!({"preferences":{"theme":"future-theme","favorites":["qr-code",17,"unit-converter"],"worldZones":["UTC"]}});
        let prefs = Preferences::from_legacy(&value);
        assert_eq!(prefs.favorites, vec!["qr-code", "unit-converter"]);
        assert_eq!(prefs.world_zones, vec!["UTC"]);
        assert_eq!(prefs.theme, Theme::System);
    }
}

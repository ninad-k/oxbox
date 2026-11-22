//! Bounded saved configurations. Content, paths and active sessions are never preset fields.
use crate::{
    CoreError, CoreResult,
    tools::{checksum::Algorithm, unit_convert},
};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResizePreset {
    pub mode: String,
    pub percent: f64,
    pub width: u32,
    pub height: u32,
    pub format: String,
    pub quality: u32,
}
impl ResizePreset {
    pub fn validate(&self) -> CoreResult<()> {
        use crate::tools::resize::{self, ResizeMode};
        if !self.percent.is_finite()
            || !(0.1..=1000.0).contains(&self.percent)
            || !matches!(self.format.as_str(), "png" | "jpeg" | "webp")
            || !(1..=100).contains(&self.quality)
        {
            return Err(invalid("Invalid resize preset options."));
        }
        // Bound even inactive dimension fields using the same core validator,
        // without treating inactive width×height as an actual output allocation.
        resize::dimensions(
            1,
            1,
            &ResizeMode::Exact {
                width: self.width,
                height: 1,
            },
        )?;
        resize::dimensions(
            1,
            1,
            &ResizeMode::Exact {
                width: 1,
                height: self.height,
            },
        )?;
        let mode = match self.mode.as_str() {
            "percent" => ResizeMode::Percent {
                percent: self.percent,
            },
            "fit" => ResizeMode::Fit {
                width: self.width,
                height: self.height,
            },
            "exact" => ResizeMode::Exact {
                width: self.width,
                height: self.height,
            },
            _ => return Err(invalid("Invalid resize preset mode.")),
        };
        resize::dimensions(1, 1, &mode)?;
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChecksumPreset {
    pub algorithm: Algorithm,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimerPreset {
    pub seconds: u64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnitPreset {
    pub from: String,
    pub to: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "tool",
    content = "options",
    rename_all = "kebab-case",
    deny_unknown_fields
)]
pub enum PresetConfig {
    ResizeImages(ResizePreset),
    VerifyChecksum(ChecksumPreset),
    TimerStopwatch(TimerPreset),
    UnitConverter(UnitPreset),
}
impl PresetConfig {
    pub fn tool_id(&self) -> &'static str {
        match self {
            Self::ResizeImages(_) => "resize-images",
            Self::VerifyChecksum(_) => "verify-checksum",
            Self::TimerStopwatch(_) => "timer-stopwatch",
            Self::UnitConverter(_) => "unit-converter",
        }
    }
    pub fn validate(&self) -> CoreResult<()> {
        match self {
            Self::ResizeImages(p) => p.validate(),
            Self::VerifyChecksum(_) => Ok(()),
            Self::TimerStopwatch(p) => {
                if (1..=604800).contains(&p.seconds) {
                    Ok(())
                } else {
                    Err(invalid("Timer presets need 1 second to 7 days."))
                }
            }
            Self::UnitConverter(p) => unit_convert::convert(1.0, &p.from, &p.to).map(|_| ()),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedPreset {
    pub name: String,
    pub config: PresetConfig,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PresetBook {
    pub version: u32,
    pub items: Vec<SavedPreset>,
}
impl Default for PresetBook {
    fn default() -> Self {
        Self {
            version: 1,
            items: Vec::new(),
        }
    }
}
fn invalid(message: &str) -> CoreError {
    CoreError::InvalidInput(message.into())
}
fn name_valid(name: &str) -> bool {
    !name.trim().is_empty() && name.len() <= 64 && !name.chars().any(char::is_control)
}
impl PresetBook {
    pub fn validate(&self) -> CoreResult<()> {
        if self.version != 1 || self.items.len() > 64 {
            return Err(invalid("Unsupported or oversized preset store."));
        }
        let mut names = std::collections::HashSet::new();
        for p in &self.items {
            if !name_valid(&p.name) || !names.insert((p.config.tool_id(), p.name.to_lowercase())) {
                return Err(invalid(
                    "Preset names must be unique for each tool and at most 64 bytes.",
                ));
            }
            p.config.validate()?;
        }
        Ok(())
    }
    pub fn save(&mut self, name: &str, config: PresetConfig) -> CoreResult<()> {
        self.validate()?;
        config.validate()?;
        let name = name.trim();
        if !name_valid(name) {
            return Err(invalid("Choose a preset name of 1 to 64 bytes."));
        }
        let index = self.items.iter().position(|p| {
            p.config.tool_id() == config.tool_id() && p.name.to_lowercase() == name.to_lowercase()
        });
        let preset = SavedPreset {
            name: name.into(),
            config,
        };
        if let Some(index) = index {
            self.items[index] = preset;
        } else {
            if self.items.len() >= 64 {
                return Err(invalid("At most 64 presets can be saved."));
            }
            self.items.push(preset);
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn resize() -> ResizePreset {
        ResizePreset {
            mode: "fit".into(),
            percent: 50.0,
            width: 1600,
            height: 1200,
            format: "webp".into(),
            quality: 85,
        }
    }
    #[test]
    fn presets_validate_before_replacing_and_survive_round_trip_without_input_content() {
        let mut book = PresetBook::default();
        book.save("Web images", PresetConfig::ResizeImages(resize()))
            .unwrap();
        book.save(
            "Timer",
            PresetConfig::TimerStopwatch(TimerPreset { seconds: 600 }),
        )
        .unwrap();
        book.save(
            "SHA",
            PresetConfig::VerifyChecksum(ChecksumPreset {
                algorithm: crate::tools::checksum::Algorithm::Sha256,
            }),
        )
        .unwrap();
        let before = serde_json::to_vec(&book).unwrap();
        let mut bad = resize();
        bad.width = 0;
        assert!(
            book.save("Web images", PresetConfig::ResizeImages(bad))
                .is_err()
        );
        assert_eq!(serde_json::to_vec(&book).unwrap(), before);
        let restored: PresetBook = serde_json::from_slice(&before).unwrap();
        restored.validate().unwrap();
        assert_eq!(restored.items.len(), 3);
        let text = String::from_utf8(before).unwrap();
        assert!(
            !text.contains("password")
                && !text.contains("input_path")
                && !text.contains("destination")
        );
    }
    #[test]
    fn presets_refuse_unknown_sensitive_fields_bad_values_and_unbounded_collections() {
        assert!(serde_json::from_str::<ResizePreset>(r#"{"mode":"fit","percent":50,"width":100,"height":100,"format":"png","quality":80,"password":"secret"}"#).is_err());
        let mut book = PresetBook::default();
        assert!(
            book.save(" ", PresetConfig::ResizeImages(resize()))
                .is_err()
        );
        assert!(
            book.save(
                "x",
                PresetConfig::TimerStopwatch(TimerPreset { seconds: 0 })
            )
            .is_err()
        );
        assert!(
            book.save(
                "x",
                PresetConfig::UnitConverter(UnitPreset {
                    from: "m".into(),
                    to: "kg".into()
                })
            )
            .is_err()
        );
        let mut bad = resize();
        bad.percent = f64::INFINITY;
        assert!(bad.validate().is_err());
        for i in 0..64 {
            book.save(&format!("Preset{i}"), PresetConfig::ResizeImages(resize()))
                .unwrap();
        }
        assert!(
            book.save("Overflow", PresetConfig::ResizeImages(resize()))
                .is_err()
        );
        book.save("Preset0", PresetConfig::ResizeImages(resize()))
            .unwrap();
        assert_eq!(book.items.len(), 64);
    }
    #[test]
    fn resize_presets_reject_dimensions_above_core_pixel_budget() {
        for mode in ["fit", "exact"] {
            let mut preset = resize();
            preset.mode = mode.into();
            preset.width = 8000;
            preset.height = 8000;
            assert!(
                preset.validate().is_err(),
                "{mode} must respect the core 40M-pixel bound"
            );
        }
    }
    #[test]
    fn resize_presets_accept_core_supported_wide_narrow_images() {
        for mode in ["fit", "exact"] {
            let mut preset = resize();
            preset.mode = mode.into();
            preset.width = 20000;
            preset.height = 1;
            assert!(
                preset.validate().is_ok(),
                "{mode} should accept core-supported 20000×1"
            );
        }
        // Inactive dimensions cannot make an otherwise valid percentage mode unusable.
        let mut percentage = resize();
        percentage.mode = "percent".into();
        percentage.width = 8000;
        percentage.height = 8000;
        assert!(percentage.validate().is_ok());
    }
}

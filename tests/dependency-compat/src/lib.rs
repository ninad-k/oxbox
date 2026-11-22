//! Generated configuration compatibility fixtures for the approved TOML backport.
//! This standalone test workspace adds no application dependency or runtime work.

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use i18n_config::{Crate, I18nConfig, I18nConfigError, WhyNotCrate};

    fn cargo_manifest(directory: &Path) {
        fs::write(
            directory.join("Cargo.toml"),
            "# Generated fixture only\n[package]\nname = \"generated-example\"\nversion = \"1.2.3\"\nedition = \"2024\"\n",
        )
        .unwrap();
    }

    #[test]
    fn valid_cargo_and_fluent_configs_preserve_unicode_paths_and_optional_defaults() {
        let directory = tempfile::tempdir().unwrap();
        cargo_manifest(directory.path());
        fs::write(
            directory.path().join("i18n.toml"),
            "fallback_language = \"en-US\"\nsubcrates = [\"tools\", \"café\"]\n[fluent]\nassets_dir = \"i18n/café\"\ndomain = \"generated-domain\"\n",
        )
        .unwrap();
        let selected = Crate::from(directory.path(), None, "i18n.toml").unwrap();
        assert_eq!(selected.name, "generated-example");
        assert_eq!(selected.version, "1.2.3");
        assert_eq!(selected.module_name(), "generated_example");
        let config = selected.config_or_err().unwrap();
        assert_eq!(config.fallback_language.to_string(), "en-US");
        assert_eq!(
            config.subcrates,
            [
                Path::new("tools").to_path_buf(),
                Path::new("café").to_path_buf()
            ]
        );
        let fluent = config.fluent.as_ref().unwrap();
        assert_eq!(fluent.assets_dir, Path::new("i18n/café"));
        assert_eq!(fluent.domain.as_deref(), Some("generated-domain"));
        assert!(config.gettext.is_none());
    }

    #[test]
    fn invalid_cargo_syntax_types_and_required_fields_return_parse_errors() {
        let directory = tempfile::tempdir().unwrap();
        let filename = directory.path().join("Cargo.toml");
        for bytes in [
            "[package\n",
            "[package]\nname = 123\nversion = \"1.2.3\"\n",
            "[package]\nname = \"generated-example\"\n",
        ] {
            fs::write(&filename, bytes).unwrap();
            let error = Crate::from(directory.path(), None, "i18n.toml").unwrap_err();
            assert!(
                matches!(error, I18nConfigError::CannotDeserializeToml(ref path, _) if path == &filename),
                "unexpected error: {error}"
            );
        }
    }

    #[test]
    fn missing_cargo_is_not_a_crate_and_missing_optional_config_remains_distinct() {
        let directory = tempfile::tempdir().unwrap();
        assert!(matches!(
            Crate::from(directory.path(), None, "i18n.toml").unwrap_err(),
            I18nConfigError::NotACrate(_, WhyNotCrate::NoCargoToml)
        ));
        cargo_manifest(directory.path());
        let selected = Crate::from(directory.path(), None, "i18n.toml").unwrap();
        assert!(selected.i18n_config.is_none());
        assert!(selected.active_config().unwrap().is_none());
        assert!(matches!(
            selected.config_or_err().unwrap_err(),
            I18nConfigError::NoI18nConfig(_)
        ));
        let missing = directory.path().join("i18n.toml");
        assert!(matches!(
            I18nConfig::from_file(&missing).unwrap_err(),
            I18nConfigError::CannotReadFile(path, _, error)
                if path == missing && error.kind() == std::io::ErrorKind::NotFound
        ));
    }

    #[test]
    fn invalid_i18n_syntax_types_and_language_do_not_fall_back_to_empty_config() {
        let directory = tempfile::tempdir().unwrap();
        cargo_manifest(directory.path());
        let filename = directory.path().join("i18n.toml");
        for bytes in [
            "fallback_language = [\n",
            "fallback_language = 42\n",
            "fallback_language = \"not_a_language_tag!\"\n",
            "[fluent]\nassets_dir = \"i18n\"\n",
        ] {
            fs::write(&filename, bytes).unwrap();
            for result in [
                I18nConfig::from_file(&filename).map(|_| ()),
                Crate::from(directory.path(), None, "i18n.toml").map(|_| ()),
            ] {
                let error = result.unwrap_err();
                assert!(
                    matches!(error, I18nConfigError::CannotDeserializeToml(ref path, _) if path == &filename),
                    "unexpected error: {error}"
                );
            }
        }
    }

    #[test]
    fn missing_child_config_retains_explicit_parent_fallback_and_gettext_defaults() {
        let directory = tempfile::tempdir().unwrap();
        cargo_manifest(directory.path());
        fs::write(
            directory.path().join("i18n.toml"),
            "fallback_language = \"en-US\"\n[gettext]\ntarget_languages = [\"fr\", \"ja\"]\noutput_dir = \"generated-locales\"\n",
        )
        .unwrap();
        let parent = Crate::from(directory.path(), None, "i18n.toml").unwrap();
        let child_path = directory.path().join("generated-child");
        fs::create_dir(&child_path).unwrap();
        cargo_manifest(&child_path);
        let child = Crate::from(&child_path, Some(&parent), "i18n.toml").unwrap();
        assert!(child.i18n_config.is_none());
        let (owner, inherited) = child.active_config().unwrap().unwrap();
        assert_eq!(owner.path, parent.path);
        assert_eq!(inherited.fallback_language.to_string(), "en-US");
        let gettext = inherited.gettext.as_ref().unwrap();
        assert_eq!(gettext.output_dir, Path::new("generated-locales"));
        assert_eq!(gettext.target_languages, ["fr", "ja"]);
        assert!(!gettext.extract_to_parent);
        assert!(!gettext.collate_extracted_subcrates);
    }
}

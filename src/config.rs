use std::path::Path;

use serde::Deserialize;

use crate::cli::VersionSourceName;
use crate::error::FlophaError;
use crate::versioning::Increment;

/// The checked-in `flopha.toml` schema driving `flopha release`.
#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields, default)]
pub struct FlophaConfig {
    pub version: VersionConfig,
    pub changelog: ChangelogConfig,
    pub release: ReleaseConfig,
    #[serde(rename = "manifest")]
    pub manifests: Vec<ManifestTarget>,
}

impl FlophaConfig {
    pub fn load(path: &Path) -> Result<Self, FlophaError> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            FlophaError::Config(format!("failed to read '{}': {}", path.display(), e))
        })?;
        let config: FlophaConfig = toml::from_str(&content).map_err(|e| {
            FlophaError::Config(format!("failed to parse '{}': {}", path.display(), e))
        })?;
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<(), FlophaError> {
        if self.version.source == VersionSourceName::Branch {
            return Err(FlophaError::Config(
                "release only supports version.source = \"tag\" (manifest sync, commits, and \
                 GitHub Releases all assume tag-based versioning)"
                    .to_string(),
            ));
        }
        for target in &self.manifests {
            if target.kind == ManifestKind::Regex
                && (target.pattern.is_none() || target.replacement.is_none())
            {
                return Err(FlophaError::Config(format!(
                    "manifest '{}': type \"regex\" requires both 'pattern' and 'replacement'",
                    target.path
                )));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct VersionConfig {
    pub pattern: String,
    pub source: VersionSourceName,
    pub auto: bool,
    pub increment: Increment,
    pub rules: Vec<String>,
    pub pre: Option<String>,
    pub tag_message: Option<String>,
}

impl Default for VersionConfig {
    fn default() -> Self {
        Self {
            pattern: "v{major}.{minor}.{patch}".to_string(),
            source: VersionSourceName::Tag,
            auto: true,
            increment: Increment::Patch,
            rules: Vec::new(),
            pre: None,
            tag_message: None,
        }
    }
}

#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields, default)]
pub struct ChangelogConfig {
    pub enabled: bool,
    pub groups: Vec<String>,
    pub other: Option<String>,
    pub title: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields, default)]
pub struct ReleaseConfig {
    pub create: bool,
    pub draft: bool,
    /// Overrides the pre-release flag; defaults to whether `version.pre` is set.
    pub prerelease: Option<bool>,
    /// Title template for the GitHub Release. Supports `{tag}` and `{version}`. Defaults to the tag name.
    pub title: Option<String>,
    /// Body for the GitHub Release. Falls back to the generated changelog, then `generate_notes`.
    pub body: Option<String>,
    pub generate_notes: bool,
    /// `owner/repo` override. Defaults to parsing the `origin` remote URL.
    pub repo: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ManifestTarget {
    /// Path to the manifest file, relative to the repository root.
    pub path: String,
    #[serde(rename = "type")]
    pub kind: ManifestKind,
    /// `regex` targets only: pattern matched against the file content.
    #[serde(default)]
    pub pattern: Option<String>,
    /// `regex` targets only: replacement text; `{version}` is substituted with the new version.
    #[serde(default)]
    pub replacement: Option<String>,
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ManifestKind {
    Cargo,
    Npm,
    Pyproject,
    Regex,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defaults_when_all_sections_omitted() {
        let config: FlophaConfig = toml::from_str("").unwrap();
        assert_eq!(config.version.pattern, "v{major}.{minor}.{patch}");
        assert_eq!(config.version.source, VersionSourceName::Tag);
        assert!(config.version.auto);
        assert!(!config.changelog.enabled);
        assert!(!config.release.create);
        assert!(config.manifests.is_empty());
    }

    #[test]
    fn test_parses_full_config() {
        let toml_str = r#"
            [version]
            pattern = "v{major}.{minor}.{patch}"
            auto = false
            increment = "minor"
            pre = "beta"

            [changelog]
            enabled = true
            other = "Misc"

            [release]
            create = true
            draft = true
            title = "Release {tag}"

            [[manifest]]
            path = "Cargo.toml"
            type = "cargo"

            [[manifest]]
            path = "VERSION"
            type = "regex"
            pattern = "^version=.*$"
            replacement = "version={version}"
        "#;
        let config: FlophaConfig = toml::from_str(toml_str).unwrap();

        assert!(!config.version.auto);
        assert!(matches!(config.version.increment, Increment::Minor));
        assert_eq!(config.version.pre.as_deref(), Some("beta"));
        assert!(config.changelog.enabled);
        assert_eq!(config.changelog.other.as_deref(), Some("Misc"));
        assert!(config.release.create);
        assert!(config.release.draft);
        assert_eq!(config.manifests.len(), 2);
        assert_eq!(config.manifests[0].kind, ManifestKind::Cargo);
        assert_eq!(config.manifests[1].kind, ManifestKind::Regex);
    }

    #[test]
    fn test_branch_source_rejected() {
        let toml_str = r#"
            [version]
            source = "branch"
        "#;
        let err = FlophaConfig::load_from_str_for_test(toml_str).unwrap_err();
        assert!(err.to_string().contains("version.source"));
    }

    #[test]
    fn test_regex_manifest_without_pattern_rejected() {
        let toml_str = r#"
            [[manifest]]
            path = "VERSION"
            type = "regex"
        "#;
        let err = FlophaConfig::load_from_str_for_test(toml_str).unwrap_err();
        assert!(err.to_string().contains("requires both 'pattern'"));
    }

    #[test]
    fn test_unknown_field_rejected() {
        let result: Result<FlophaConfig, _> = toml::from_str("typo_field = true");
        assert!(result.is_err());
    }

    impl FlophaConfig {
        /// Test-only helper mirroring `load` without touching the filesystem.
        fn load_from_str_for_test(s: &str) -> Result<Self, FlophaError> {
            let config: FlophaConfig = toml::from_str(s)
                .map_err(|e| FlophaError::Config(format!("failed to parse: {}", e)))?;
            config.validate()?;
            Ok(config)
        }
    }
}

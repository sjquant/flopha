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
        let config: FlophaConfig =
            toml::from_str(&content).map_err(|e| FlophaError::parse(path, e))?;
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

    /// Whether the GitHub Release should be marked as a pre-release: an explicit
    /// `release.prerelease` always wins, otherwise it follows whether a
    /// `version.pre` channel is configured. Lives here (not on `ReleaseConfig`)
    /// since both fields are only in scope together at this level.
    pub(crate) fn is_prerelease(&self) -> bool {
        self.release
            .prerelease
            .unwrap_or(self.version.pre.is_some())
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
    use tempfile::TempDir;

    fn load(content: &str) -> Result<FlophaConfig, FlophaError> {
        let td = TempDir::new().unwrap();
        let path = td.path().join("flopha.toml");
        std::fs::write(&path, content).unwrap();
        FlophaConfig::load(&path)
    }

    /// It falls back to documented defaults when every section is omitted.
    #[test]
    fn test_defaults_when_all_sections_omitted() {
        // Given an empty config file
        // When loading it
        let config = load("").unwrap();

        // Then every field takes its documented default
        assert_eq!(config.version.pattern, "v{major}.{minor}.{patch}");
        assert_eq!(config.version.source, VersionSourceName::Tag);
        assert!(config.version.auto);
        assert!(!config.changelog.enabled);
        assert!(!config.release.create);
        assert!(config.manifests.is_empty());
    }

    /// It parses every section and manifest kind.
    #[test]
    fn test_parses_full_config() {
        // Given a config exercising every section
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

        // When loading it
        let config = load(toml_str).unwrap();

        // Then every configured value is reflected, including manifest targets
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

    /// It rejects `version.source = "branch"` since release assumes tag-based versioning.
    #[test]
    fn test_branch_source_rejected() {
        // Given a config that sets version.source to "branch"
        let toml_str = r#"
            [version]
            source = "branch"
        "#;

        // When loading it
        let err = load(toml_str).unwrap_err();

        // Then it's rejected with a message naming the offending setting
        assert!(err.to_string().contains("version.source"));
    }

    /// It rejects a `type = "regex"` manifest target missing `pattern`/`replacement`.
    #[test]
    fn test_regex_manifest_without_pattern_rejected() {
        // Given a regex manifest target with no pattern or replacement
        let toml_str = r#"
            [[manifest]]
            path = "VERSION"
            type = "regex"
        "#;

        // When loading it
        let err = load(toml_str).unwrap_err();

        // Then it's rejected explaining both fields are required
        assert!(err.to_string().contains("requires both 'pattern'"));
    }

    /// It rejects unknown top-level keys instead of silently ignoring typos.
    #[test]
    fn test_unknown_field_rejected() {
        // Given a config with a misspelled/unknown field
        // When parsing it
        let result: Result<FlophaConfig, _> = toml::from_str("typo_field = true");

        // Then it's rejected
        assert!(result.is_err());
    }
}

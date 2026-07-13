use std::path::{Path, PathBuf};

use regex::{NoExpand, Regex};

use crate::config::{ManifestKind, ManifestTarget};
use crate::error::FlophaError;

/// Computes `target`'s updated content for `version`, without writing to disk.
/// Returns the target's repo-relative path and new content when it differs
/// from what's on disk, or `None` when the file already contains the given
/// version. Kept separate from [`sync`] so callers syncing several targets
/// (see `release::release`) can validate every target before writing any of
/// them, instead of leaving earlier targets rewritten if a later one fails.
pub fn compute(
    base_dir: &Path,
    target: &ManifestTarget,
    version: &str,
) -> Result<Option<(PathBuf, String)>, FlophaError> {
    let path = base_dir.join(&target.path);
    let content = std::fs::read_to_string(&path)?;

    let updated = match target.kind {
        ManifestKind::Cargo => set_toml_field(&content, &path, "package", "version", version)?,
        ManifestKind::Pyproject => set_pyproject_version(&content, &path, version)?,
        ManifestKind::Npm => set_json_version(&content, &path, version)?,
        ManifestKind::Regex => set_regex_version(&content, target, version)?,
    };

    if updated == content {
        return Ok(None);
    }
    Ok(Some((PathBuf::from(&target.path), updated)))
}

/// Computes and writes `version` into `target` under `base_dir`, if the file's
/// contents change. Returns the target's repo-relative path when it was
/// rewritten, or `None` when the file already contained the given version.
pub fn sync(
    base_dir: &Path,
    target: &ManifestTarget,
    version: &str,
) -> Result<Option<PathBuf>, FlophaError> {
    match compute(base_dir, target, version)? {
        Some((rel, content)) => {
            std::fs::write(base_dir.join(&rel), content)?;
            Ok(Some(rel))
        }
        None => Ok(None),
    }
}

fn set_toml_field(
    content: &str,
    path: &Path,
    table: &str,
    key: &str,
    version: &str,
) -> Result<String, FlophaError> {
    let mut doc = content
        .parse::<toml_edit::DocumentMut>()
        .map_err(|e| FlophaError::Config(format!("failed to parse '{}': {}", path.display(), e)))?;

    let tbl = doc[table].as_table_mut().ok_or_else(|| {
        FlophaError::Config(format!("'{}': missing [{}] table", path.display(), table))
    })?;
    if !tbl.contains_key(key) {
        return Err(FlophaError::Config(format!(
            "'{}': no '{}' field in [{}]",
            path.display(),
            key,
            table
        )));
    }
    tbl[key] = toml_edit::value(version);
    Ok(doc.to_string())
}

fn set_pyproject_version(content: &str, path: &Path, version: &str) -> Result<String, FlophaError> {
    let mut doc = content
        .parse::<toml_edit::DocumentMut>()
        .map_err(|e| FlophaError::Config(format!("failed to parse '{}': {}", path.display(), e)))?;

    if doc.get("project").and_then(|t| t.get("version")).is_some() {
        doc["project"]["version"] = toml_edit::value(version);
        return Ok(doc.to_string());
    }
    if doc
        .get("tool")
        .and_then(|t| t.get("poetry"))
        .and_then(|t| t.get("version"))
        .is_some()
    {
        doc["tool"]["poetry"]["version"] = toml_edit::value(version);
        return Ok(doc.to_string());
    }
    Err(FlophaError::Config(format!(
        "'{}': no [project].version or [tool.poetry].version field found",
        path.display()
    )))
}

fn set_json_version(content: &str, path: &Path, version: &str) -> Result<String, FlophaError> {
    let mut value: serde_json::Value = serde_json::from_str(content)
        .map_err(|e| FlophaError::Config(format!("failed to parse '{}': {}", path.display(), e)))?;
    let obj = value.as_object_mut().ok_or_else(|| {
        FlophaError::Config(format!("'{}': expected a JSON object", path.display()))
    })?;
    if !obj.contains_key("version") {
        return Err(FlophaError::Config(format!(
            "'{}': no top-level 'version' field",
            path.display()
        )));
    }
    obj.insert(
        "version".to_string(),
        serde_json::Value::String(version.to_string()),
    );
    let mut out = serde_json::to_string_pretty(&value)?;
    out.push('\n');
    Ok(out)
}

fn set_regex_version(
    content: &str,
    target: &ManifestTarget,
    version: &str,
) -> Result<String, FlophaError> {
    // Validated by `FlophaConfig::load` — regex targets always carry both fields.
    let pattern = target.pattern.as_deref().unwrap();
    let replacement = target.replacement.as_deref().unwrap();

    let regex = Regex::new(pattern)
        .map_err(|e| FlophaError::Config(format!("invalid regex '{}': {}", pattern, e)))?;
    if !regex.is_match(content) {
        return Err(FlophaError::Config(format!(
            "pattern '{}' did not match any content in '{}'",
            pattern, target.path
        )));
    }
    let replacement = replacement.replace("{version}", version);
    // `NoExpand` treats the replacement as a literal string rather than a `$1`/`$name`
    // capture-group template, since `{version}` substitution above is already complete
    // and the version string is not under our control (e.g. `pre` channel names come
    // from flopha.toml). `replace_all` (not `replace`) so every match in the file is
    // updated, not just the first — manifests can legitimately repeat the pattern.
    Ok(regex
        .replace_all(content, NoExpand(&replacement))
        .into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write(td: &TempDir, name: &str, content: &str) {
        std::fs::write(td.path().join(name), content).unwrap();
    }

    fn target(path: &str, kind: ManifestKind) -> ManifestTarget {
        ManifestTarget {
            path: path.to_string(),
            kind,
            pattern: None,
            replacement: None,
        }
    }

    /// It sets [package].version while preserving other fields and formatting.
    #[test]
    fn test_sync_updates_cargo_toml_version_preserving_formatting() {
        // Given a Cargo.toml with a name field, dependencies section, and a version to bump
        let td = TempDir::new().unwrap();
        write(
            &td,
            "Cargo.toml",
            "[package]\nname = \"flopha\"\nversion = \"0.4.1\"\n\n[dependencies]\n",
        );

        // When syncing the new version
        let touched = sync(
            td.path(),
            &target("Cargo.toml", ManifestKind::Cargo),
            "0.5.0",
        )
        .unwrap()
        .unwrap();

        // Then only the version field changes; everything else is preserved
        assert_eq!(touched, PathBuf::from("Cargo.toml"));
        let content = std::fs::read_to_string(td.path().join("Cargo.toml")).unwrap();
        assert!(content.contains("version = \"0.5.0\""));
        assert!(
            content.contains("name = \"flopha\""),
            "should preserve other fields"
        );
        assert!(
            content.contains("[dependencies]"),
            "should preserve trailing sections"
        );
    }

    /// It returns `None` and leaves the file untouched when the version is already current.
    #[test]
    fn test_sync_cargo_toml_no_change_returns_none() {
        // Given a Cargo.toml already at the target version
        let td = TempDir::new().unwrap();
        write(&td, "Cargo.toml", "[package]\nversion = \"0.5.0\"\n");

        // When syncing the same version
        let touched = sync(
            td.path(),
            &target("Cargo.toml", ManifestKind::Cargo),
            "0.5.0",
        )
        .unwrap();

        // Then nothing is reported as touched
        assert!(touched.is_none());
    }

    /// It sets the top-level "version" field in package.json.
    #[test]
    fn test_sync_updates_package_json_version() {
        // Given a package.json with a name and version field
        let td = TempDir::new().unwrap();
        write(
            &td,
            "package.json",
            "{\n  \"name\": \"app\",\n  \"version\": \"1.0.0\"\n}\n",
        );

        // When syncing the new version
        sync(
            td.path(),
            &target("package.json", ManifestKind::Npm),
            "1.1.0",
        )
        .unwrap();

        // Then the version field is updated and other fields are preserved
        let content = std::fs::read_to_string(td.path().join("package.json")).unwrap();
        let v: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(v["version"], "1.1.0");
        assert_eq!(v["name"], "app");
    }

    /// It sets [project].version for PEP 621-style pyproject.toml files.
    #[test]
    fn test_sync_updates_pyproject_pep621_version() {
        // Given a pyproject.toml using the [project] table
        let td = TempDir::new().unwrap();
        write(
            &td,
            "pyproject.toml",
            "[project]\nname = \"app\"\nversion = \"1.0.0\"\n",
        );

        // When syncing the new version
        sync(
            td.path(),
            &target("pyproject.toml", ManifestKind::Pyproject),
            "1.2.0",
        )
        .unwrap();

        // Then [project].version is updated
        let content = std::fs::read_to_string(td.path().join("pyproject.toml")).unwrap();
        assert!(content.contains("version = \"1.2.0\""));
    }

    /// It falls back to [tool.poetry].version when there is no [project] table.
    #[test]
    fn test_sync_updates_pyproject_poetry_version() {
        // Given a pyproject.toml using the Poetry-style [tool.poetry] table
        let td = TempDir::new().unwrap();
        write(
            &td,
            "pyproject.toml",
            "[tool.poetry]\nname = \"app\"\nversion = \"1.0.0\"\n",
        );

        // When syncing the new version
        sync(
            td.path(),
            &target("pyproject.toml", ManifestKind::Pyproject),
            "1.2.0",
        )
        .unwrap();

        // Then [tool.poetry].version is updated
        let content = std::fs::read_to_string(td.path().join("pyproject.toml")).unwrap();
        assert!(content.contains("version = \"1.2.0\""));
    }

    /// It substitutes the `{version}` placeholder into the replacement template.
    #[test]
    fn test_sync_regex_target_substitutes_version_placeholder() {
        // Given a regex target matching a "version=" line
        let td = TempDir::new().unwrap();
        write(&td, "VERSION", "version=0.1.0\n");
        let mut t = target("VERSION", ManifestKind::Regex);
        t.pattern = Some(r"(?m)^version=.*$".to_string());
        t.replacement = Some("version={version}".to_string());

        // When syncing the new version
        sync(td.path(), &t, "0.2.0").unwrap();

        // Then the placeholder is replaced with the new version
        let content = std::fs::read_to_string(td.path().join("VERSION")).unwrap();
        assert_eq!(content, "version=0.2.0\n");
    }

    /// It updates every match, not just the first, since manifests can legitimately
    /// repeat the version string (e.g. a Dockerfile with several `ENV VERSION=` lines).
    #[test]
    fn test_sync_regex_target_replaces_all_matches() {
        // Given a file where the pattern matches on three separate lines
        let td = TempDir::new().unwrap();
        write(
            &td,
            "VERSION",
            "version=0.1.0\nversion=0.1.0\nversion=0.1.0\n",
        );
        let mut t = target("VERSION", ManifestKind::Regex);
        t.pattern = Some(r"(?m)^version=.*$".to_string());
        t.replacement = Some("version={version}".to_string());

        // When syncing the new version
        sync(td.path(), &t, "0.2.0").unwrap();

        // Then all three lines are updated, not just the first
        let content = std::fs::read_to_string(td.path().join("VERSION")).unwrap();
        assert_eq!(content, "version=0.2.0\nversion=0.2.0\nversion=0.2.0\n");
    }

    /// It treats the replacement as a literal string rather than expanding `$`
    /// capture-group references, since the surrounding template text isn't
    /// under flopha's control (it comes straight from flopha.toml).
    #[test]
    fn test_sync_regex_target_does_not_expand_dollar_signs() {
        // Given a replacement template containing a literal `$` character
        let td = TempDir::new().unwrap();
        write(&td, "VERSION", "version=0.1.0\n");
        let mut t = target("VERSION", ManifestKind::Regex);
        t.pattern = Some(r"(?m)^version=.*$".to_string());
        t.replacement = Some("version={version}-$1-build".to_string());

        // When syncing the new version
        sync(td.path(), &t, "0.2.0").unwrap();

        // Then "$1" is written out literally instead of being expanded as a capture group
        let content = std::fs::read_to_string(td.path().join("VERSION")).unwrap();
        assert_eq!(content, "version=0.2.0-$1-build\n");
    }

    /// It errors instead of silently no-op'ing when the pattern matches nothing.
    #[test]
    fn test_sync_regex_no_match_errors() {
        // Given a file that doesn't contain anything matching the configured pattern
        let td = TempDir::new().unwrap();
        write(&td, "VERSION", "no version here\n");
        let mut t = target("VERSION", ManifestKind::Regex);
        t.pattern = Some(r"(?m)^version=.*$".to_string());
        t.replacement = Some("version={version}".to_string());

        // When syncing the new version
        let result = sync(td.path(), &t, "0.2.0");

        // Then it errors
        assert!(result.is_err());
    }

    /// It errors instead of silently defaulting when the manifest has no version field.
    #[test]
    fn test_sync_cargo_toml_missing_version_field_errors() {
        // Given a Cargo.toml with no version field
        let td = TempDir::new().unwrap();
        write(&td, "Cargo.toml", "[package]\nname = \"flopha\"\n");

        // When syncing a new version
        let result = sync(
            td.path(),
            &target("Cargo.toml", ManifestKind::Cargo),
            "0.5.0",
        );

        // Then it errors
        assert!(result.is_err());
    }
}

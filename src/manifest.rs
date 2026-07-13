use std::path::{Path, PathBuf};

use regex::Regex;

use crate::config::{ManifestKind, ManifestTarget};
use crate::error::FlophaError;

/// Writes `version` into `target` under `base_dir`, if the file's contents change.
/// Returns the target's repo-relative path when it was rewritten, or `None` when
/// the file already contained the given version.
pub fn sync(
    base_dir: &Path,
    target: &ManifestTarget,
    version: &str,
) -> Result<Option<PathBuf>, FlophaError> {
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
    std::fs::write(&path, updated)?;
    Ok(Some(PathBuf::from(&target.path)))
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
    Ok(regex.replace(content, replacement.as_str()).into_owned())
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

    #[test]
    fn test_sync_updates_cargo_toml_version_preserving_formatting() {
        let td = TempDir::new().unwrap();
        write(
            &td,
            "Cargo.toml",
            "[package]\nname = \"flopha\"\nversion = \"0.4.1\"\n\n[dependencies]\n",
        );

        let touched = sync(
            td.path(),
            &target("Cargo.toml", ManifestKind::Cargo),
            "0.5.0",
        )
        .unwrap()
        .unwrap();

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

    #[test]
    fn test_sync_cargo_toml_no_change_returns_none() {
        let td = TempDir::new().unwrap();
        write(&td, "Cargo.toml", "[package]\nversion = \"0.5.0\"\n");

        let touched = sync(
            td.path(),
            &target("Cargo.toml", ManifestKind::Cargo),
            "0.5.0",
        )
        .unwrap();

        assert!(touched.is_none());
    }

    #[test]
    fn test_sync_updates_package_json_version() {
        let td = TempDir::new().unwrap();
        write(
            &td,
            "package.json",
            "{\n  \"name\": \"app\",\n  \"version\": \"1.0.0\"\n}\n",
        );

        sync(
            td.path(),
            &target("package.json", ManifestKind::Npm),
            "1.1.0",
        )
        .unwrap();

        let content = std::fs::read_to_string(td.path().join("package.json")).unwrap();
        let v: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(v["version"], "1.1.0");
        assert_eq!(v["name"], "app");
    }

    #[test]
    fn test_sync_updates_pyproject_pep621_version() {
        let td = TempDir::new().unwrap();
        write(
            &td,
            "pyproject.toml",
            "[project]\nname = \"app\"\nversion = \"1.0.0\"\n",
        );

        sync(
            td.path(),
            &target("pyproject.toml", ManifestKind::Pyproject),
            "1.2.0",
        )
        .unwrap();

        let content = std::fs::read_to_string(td.path().join("pyproject.toml")).unwrap();
        assert!(content.contains("version = \"1.2.0\""));
    }

    #[test]
    fn test_sync_updates_pyproject_poetry_version() {
        let td = TempDir::new().unwrap();
        write(
            &td,
            "pyproject.toml",
            "[tool.poetry]\nname = \"app\"\nversion = \"1.0.0\"\n",
        );

        sync(
            td.path(),
            &target("pyproject.toml", ManifestKind::Pyproject),
            "1.2.0",
        )
        .unwrap();

        let content = std::fs::read_to_string(td.path().join("pyproject.toml")).unwrap();
        assert!(content.contains("version = \"1.2.0\""));
    }

    #[test]
    fn test_sync_regex_target_substitutes_version_placeholder() {
        let td = TempDir::new().unwrap();
        write(&td, "VERSION", "version=0.1.0\n");
        let mut t = target("VERSION", ManifestKind::Regex);
        t.pattern = Some(r"(?m)^version=.*$".to_string());
        t.replacement = Some("version={version}".to_string());

        sync(td.path(), &t, "0.2.0").unwrap();

        let content = std::fs::read_to_string(td.path().join("VERSION")).unwrap();
        assert_eq!(content, "version=0.2.0\n");
    }

    #[test]
    fn test_sync_regex_no_match_errors() {
        let td = TempDir::new().unwrap();
        write(&td, "VERSION", "no version here\n");
        let mut t = target("VERSION", ManifestKind::Regex);
        t.pattern = Some(r"(?m)^version=.*$".to_string());
        t.replacement = Some("version={version}".to_string());

        let result = sync(td.path(), &t, "0.2.0");

        assert!(result.is_err());
    }

    #[test]
    fn test_sync_cargo_toml_missing_version_field_errors() {
        let td = TempDir::new().unwrap();
        write(&td, "Cargo.toml", "[package]\nname = \"flopha\"\n");

        let result = sync(
            td.path(),
            &target("Cargo.toml", ManifestKind::Cargo),
            "0.5.0",
        );

        assert!(result.is_err());
    }
}

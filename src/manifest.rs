use std::collections::BTreeMap;
use std::ops::Range;
use std::path::{Path, PathBuf};

use regex::{NoExpand, Regex};

use crate::config::{ManifestKind, ManifestTarget};
use crate::error::FlophaError;

/// Pending file edits for one release. Reads see earlier edits to the same file
/// (two Cargo targets can share one `Cargo.lock`), and nothing touches disk until
/// the caller writes [`Edits::changes`], so a failing target leaves no partial edits.
pub struct Edits {
    base_dir: PathBuf,
    files: BTreeMap<PathBuf, (String, String)>,
}

impl Edits {
    pub fn new(base_dir: &Path) -> Self {
        Self {
            base_dir: base_dir.to_path_buf(),
            files: BTreeMap::new(),
        }
    }

    /// Repo-relative paths and new contents of the files whose content changed.
    pub fn changes(self) -> Vec<(PathBuf, String)> {
        self.files
            .into_iter()
            .filter(|(_, (original, current))| original != current)
            .map(|(rel, (_, current))| (rel, current))
            .collect()
    }

    fn read(&mut self, rel: &Path) -> Result<String, FlophaError> {
        if let Some((_, current)) = self.files.get(rel) {
            return Ok(current.clone());
        }
        let content = std::fs::read_to_string(self.base_dir.join(rel))?;
        self.files
            .insert(rel.to_path_buf(), (content.clone(), content.clone()));
        Ok(content)
    }

    fn write(&mut self, rel: &Path, content: String) {
        if let Some((_, current)) = self.files.get_mut(rel) {
            *current = content;
        }
    }
}

/// Records `version` into `target` as pending edits. Cargo targets also update
/// the matching entries of the nearest `Cargo.lock`, so the tagged tree still
/// builds with `--locked`.
pub fn apply(edits: &mut Edits, target: &ManifestTarget, version: &str) -> Result<(), FlophaError> {
    let rel = PathBuf::from(&target.path);
    let path = edits.base_dir.join(&rel);
    let content = edits.read(&rel)?;

    match target.kind {
        ManifestKind::Cargo => {
            let (updated, bumped) = set_cargo_version(&content, &path, version)?;
            edits.write(&rel, updated);
            if let Some(lock_rel) = find_cargo_lock(&edits.base_dir, &rel) {
                let lock = edits.read(&lock_rel)?;
                let updated =
                    update_cargo_lock(&lock, &edits.base_dir.join(&lock_rel), &bumped, version)?;
                edits.write(&lock_rel, updated);
            }
        }
        ManifestKind::Pyproject => {
            edits.write(&rel, set_pyproject_version(&content, &path, version)?)
        }
        ManifestKind::Npm => edits.write(&rel, set_json_version(&content, &path, version)?),
        ManifestKind::Regex => edits.write(&rel, set_regex_version(&content, target, version)?),
    }
    Ok(())
}

/// Which `Cargo.lock` entries a Cargo.toml bump applies to: local (source-less)
/// packages at the previous version, limited to `name` unless the whole
/// workspace version moved.
struct LockMatch {
    name: Option<String>,
    old_version: String,
}

/// Sets `[package].version`, or `[workspace.package].version` when the package
/// inherits it (`version.workspace = true`), so inheritance isn't replaced by a
/// fixed string.
fn set_cargo_version(
    content: &str,
    path: &Path,
    version: &str,
) -> Result<(String, LockMatch), FlophaError> {
    let doc = parse_toml_document(content, path)?;
    let package_version = doc.get("package").and_then(|p| p.get("version"));

    if let Some(old) = package_version.and_then(|v| v.as_str()) {
        let name = doc["package"]
            .get("name")
            .and_then(|n| n.as_str())
            .map(str::to_string);
        let updated = set_toml_field(content, path, &["package"], "version", version)?;
        return Ok((
            updated,
            LockMatch {
                name,
                old_version: old.to_string(),
            },
        ));
    }
    let workspace_version = doc
        .get("workspace")
        .and_then(|w| w.get("package"))
        .and_then(|p| p.get("version"))
        .and_then(|v| v.as_str());
    if let Some(old) = workspace_version {
        let updated = set_toml_field(content, path, &["workspace", "package"], "version", version)?;
        return Ok((
            updated,
            LockMatch {
                name: None,
                old_version: old.to_string(),
            },
        ));
    }
    if package_version.is_some() {
        return Err(FlophaError::Config(format!(
            "'{}': [package].version is inherited from the workspace; point this target at \
             the workspace root Cargo.toml instead",
            path.display()
        )));
    }
    Err(FlophaError::Config(format!(
        "'{}': no [package].version or [workspace.package].version field found",
        path.display()
    )))
}

/// Finds the `Cargo.lock` governing `manifest`: the nearest one in its directory
/// or an ancestor, within the repository.
fn find_cargo_lock(base_dir: &Path, manifest: &Path) -> Option<PathBuf> {
    manifest
        .ancestors()
        .skip(1)
        .map(|dir| dir.join("Cargo.lock"))
        .find(|lock| base_dir.join(lock).is_file())
}

fn update_cargo_lock(
    content: &str,
    path: &Path,
    bumped: &LockMatch,
    version: &str,
) -> Result<String, FlophaError> {
    let mut doc = parse_toml_document(content, path)?;
    if let Some(packages) = doc
        .get_mut("package")
        .and_then(|p| p.as_array_of_tables_mut())
    {
        for package in packages.iter_mut() {
            let is_local = !package.contains_key("source");
            let at_old_version = package.get("version").and_then(|v| v.as_str())
                == Some(bumped.old_version.as_str());
            let name_matches = bumped
                .name
                .as_deref()
                .is_none_or(|name| package.get("name").and_then(|v| v.as_str()) == Some(name));
            if is_local && at_old_version && name_matches {
                package["version"] = toml_edit::value(version);
            }
        }
    }
    Ok(doc.to_string())
}

fn parse_toml_document(content: &str, path: &Path) -> Result<toml_edit::DocumentMut, FlophaError> {
    content
        .parse::<toml_edit::DocumentMut>()
        .map_err(|e| FlophaError::parse(path, e))
}

/// Sets the string `key` inside the table at `table_path` (e.g. `&["tool", "poetry"]`),
/// erroring if the table or a string value for `key` doesn't exist.
fn set_toml_field(
    content: &str,
    path: &Path,
    table_path: &[&str],
    key: &str,
    version: &str,
) -> Result<String, FlophaError> {
    let mut doc = parse_toml_document(content, path)?;

    let mut table: &mut dyn toml_edit::TableLike = doc.as_table_mut();
    for t in table_path {
        table = table
            .get_mut(t)
            .and_then(|item| item.as_table_like_mut())
            .ok_or_else(|| {
                FlophaError::Config(format!(
                    "'{}': missing [{}] table",
                    path.display(),
                    table_path.join(".")
                ))
            })?;
    }
    if table.get(key).and_then(|item| item.as_str()).is_none() {
        return Err(FlophaError::Config(format!(
            "'{}': no string '{}' field in [{}]",
            path.display(),
            key,
            table_path.join(".")
        )));
    }
    table.insert(key, toml_edit::value(version));
    Ok(doc.to_string())
}

fn set_pyproject_version(content: &str, path: &Path, version: &str) -> Result<String, FlophaError> {
    let doc = parse_toml_document(content, path)?;

    if doc.get("project").and_then(|t| t.get("version")).is_some() {
        return set_toml_field(content, path, &["project"], "version", version);
    }
    if doc
        .get("tool")
        .and_then(|t| t.get("poetry"))
        .and_then(|t| t.get("version"))
        .is_some()
    {
        return set_toml_field(content, path, &["tool", "poetry"], "version", version);
    }
    Err(FlophaError::Config(format!(
        "'{}': no [project].version or [tool.poetry].version field found",
        path.display()
    )))
}

/// Replaces only the top-level `"version"` string in place, so the file's
/// indentation and key order stay exactly as they were.
fn set_json_version(content: &str, path: &Path, version: &str) -> Result<String, FlophaError> {
    let value: serde_json::Value =
        serde_json::from_str(content).map_err(|e| FlophaError::parse(path, e))?;
    if value.get("version").and_then(|v| v.as_str()).is_none() {
        return Err(FlophaError::Config(format!(
            "'{}': no top-level string 'version' field",
            path.display()
        )));
    }
    let span = top_level_string_value_span(content, "version").ok_or_else(|| {
        FlophaError::Config(format!(
            "'{}': could not locate the top-level 'version' field",
            path.display()
        ))
    })?;
    Ok(format!(
        "{}{}{}",
        &content[..span.start],
        serde_json::to_string(version)?,
        &content[span.end..]
    ))
}

/// Byte range (quotes included) of the string value of top-level `key` in
/// already-validated JSON.
fn top_level_string_value_span(json: &str, key: &str) -> Option<Range<usize>> {
    let bytes = json.as_bytes();
    let mut depth = 0usize;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'{' | b'[' => depth += 1,
            b'}' | b']' => depth -= 1,
            b'"' => {
                let end = string_end(bytes, i)?;
                let after = skip_whitespace(bytes, end);
                if depth == 1 && bytes.get(after) == Some(&b':') && &json[i + 1..end - 1] == key {
                    let value_start = skip_whitespace(bytes, after + 1);
                    if bytes.get(value_start) != Some(&b'"') {
                        return None;
                    }
                    return Some(value_start..string_end(bytes, value_start)?);
                }
                i = end;
                continue;
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// Index just past the closing quote of the JSON string starting at `start`.
fn string_end(bytes: &[u8], start: usize) -> Option<usize> {
    let mut i = start + 1;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 2,
            b'"' => return Some(i + 1),
            _ => i += 1,
        }
    }
    None
}

fn skip_whitespace(bytes: &[u8], mut i: usize) -> usize {
    while bytes.get(i).is_some_and(u8::is_ascii_whitespace) {
        i += 1;
    }
    i
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

    /// Applies `target` and writes the resulting edits, returning the touched paths.
    fn sync(
        base: &Path,
        target: &ManifestTarget,
        version: &str,
    ) -> Result<Vec<PathBuf>, FlophaError> {
        let mut edits = Edits::new(base);
        apply(&mut edits, target, version)?;
        let changes = edits.changes();
        for (rel, content) in &changes {
            std::fs::write(base.join(rel), content).unwrap();
        }
        Ok(changes.into_iter().map(|(rel, _)| rel).collect())
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
        .unwrap();

        // Then only the version field changes; everything else is preserved
        assert_eq!(touched, vec![PathBuf::from("Cargo.toml")]);
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

    /// It reports nothing touched when the version is already current.
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
        assert!(touched.is_empty());
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

    /// It bumps the matching local package in Cargo.lock alongside Cargo.toml.
    #[test]
    fn test_sync_cargo_updates_cargo_lock() {
        // Given a crate at 0.4.1 with a lockfile that also pins a registry crate at 0.4.1
        let td = TempDir::new().unwrap();
        write(
            &td,
            "Cargo.toml",
            "[package]\nname = \"app\"\nversion = \"0.4.1\"\n",
        );
        write(
            &td,
            "Cargo.lock",
            "version = 4\n\n[[package]]\nname = \"app\"\nversion = \"0.4.1\"\n\n[[package]]\nname = \"dep\"\nversion = \"0.4.1\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\n",
        );

        // When syncing the new version
        let touched = sync(
            td.path(),
            &target("Cargo.toml", ManifestKind::Cargo),
            "0.5.0",
        )
        .unwrap();

        // Then the crate's lock entry moves with it, but the registry crate is untouched
        assert_eq!(touched.len(), 2);
        let lock = std::fs::read_to_string(td.path().join("Cargo.lock")).unwrap();
        assert!(
            lock.contains("name = \"app\"\nversion = \"0.5.0\""),
            "{lock}"
        );
        assert!(
            lock.contains("name = \"dep\"\nversion = \"0.4.1\""),
            "{lock}"
        );
    }

    /// It bumps [workspace.package].version when the root package inherits it.
    #[test]
    fn test_sync_cargo_workspace_root_bumps_workspace_version() {
        // Given a workspace root whose package inherits the workspace version
        let td = TempDir::new().unwrap();
        write(
            &td,
            "Cargo.toml",
            "[workspace]\nmembers = [\"crates/*\"]\n\n[workspace.package]\nversion = \"0.4.1\"\n\n[package]\nname = \"app\"\nversion.workspace = true\n",
        );

        // When syncing the new version
        sync(
            td.path(),
            &target("Cargo.toml", ManifestKind::Cargo),
            "0.5.0",
        )
        .unwrap();

        // Then the workspace version is bumped and the package keeps inheriting it
        let content = std::fs::read_to_string(td.path().join("Cargo.toml")).unwrap();
        assert!(
            content.contains("[workspace.package]\nversion = \"0.5.0\""),
            "{content}"
        );
        assert!(content.contains("version.workspace = true"), "{content}");
    }

    /// It refuses to overwrite an inherited version in a workspace member.
    #[test]
    fn test_sync_cargo_member_with_inherited_version_errors() {
        // Given a member crate that inherits its version from the workspace
        let td = TempDir::new().unwrap();
        write(
            &td,
            "Cargo.toml",
            "[package]\nname = \"member\"\nversion = { workspace = true }\n",
        );

        // When syncing a new version
        let err = sync(
            td.path(),
            &target("Cargo.toml", ManifestKind::Cargo),
            "0.5.0",
        )
        .unwrap_err();

        // Then it errors, pointing at the workspace root, and leaves the file alone
        assert!(err.to_string().contains("workspace root"), "{err}");
        let content = std::fs::read_to_string(td.path().join("Cargo.toml")).unwrap();
        assert!(content.contains("version = { workspace = true }"));
    }

    /// It changes only the version in package.json, keeping tab indentation intact.
    #[test]
    fn test_sync_package_json_preserves_formatting() {
        // Given a tab-indented package.json whose dependencies also have a "version"-like key
        let td = TempDir::new().unwrap();
        let original = "{\n\t\"name\": \"app\",\n\t\"config\": {\"version\": \"9.9.9\"},\n\t\"version\":   \"1.0.0\"\n}\n";
        write(&td, "package.json", original);

        // When syncing the new version
        sync(
            td.path(),
            &target("package.json", ManifestKind::Npm),
            "1.1.0",
        )
        .unwrap();

        // Then only the top-level version text changes
        let content = std::fs::read_to_string(td.path().join("package.json")).unwrap();
        assert_eq!(content, original.replace("\"1.0.0\"", "\"1.1.0\""));
    }
}

use std::path::Path;

use crate::changelog::{build_changelog, ChangelogRequest};
use crate::cli::ChangelogArgs;
use crate::error::FlophaError;
use crate::gitutils;

pub fn changelog(path: &Path, args: &ChangelogArgs) -> Result<(), FlophaError> {
    let repo = gitutils::get_repo(path)?;
    gitutils::try_fetch_origin(&repo);

    let pattern = super::pattern_or_default(&args.pattern);

    let from_tag: Option<String> = if let Some(ref from) = args.from {
        Some(from.clone())
    } else {
        let versioner = super::versioner_factory(&repo, pattern, &args.source);
        versioner.last_version().map(|v| v.tag)
    };

    let content = build_changelog(
        &repo,
        &ChangelogRequest {
            from_tag: from_tag.as_deref(),
            to: args.to.as_deref(),
            to_label: None,
            raw_groups: &args.group,
            other: args.other.as_deref(),
            title_template: args.title.as_deref(),
            format: &args.format,
        },
    )?;

    if let Some(ref output_path) = args.output {
        if !args.overwrite && std::path::Path::new(output_path).exists() {
            let existing = std::fs::read_to_string(output_path)?;
            // Write to a sibling temp file then rename so the original is never
            // left empty if the process is interrupted between the two operations.
            let tmp_path = format!("{}.flopha.tmp", output_path);
            std::fs::write(&tmp_path, format!("{}\n{}", content, existing))?;
            std::fs::rename(&tmp_path, output_path)?;
        } else {
            std::fs::write(output_path, &content)?;
        }
    } else {
        print!("{}", content);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{OutputFormat, VersionSourceName};
    use crate::gitutils;
    use crate::testutils::{self, create_new_remote_tag};

    #[test]
    fn test_changelog_categorizes_conventional_commits() {
        let (td, repo) = testutils::init_repo();
        let (_remote_td, mut remote) = testutils::init_remote(&repo);

        create_new_remote_tag(&repo, &mut remote, "v1.0.0", false);
        gitutils::checkout_tag(&repo, "v1.0.0").unwrap();
        gitutils::commit(&repo, "feat: add search").unwrap();
        gitutils::commit(&repo, "fix: crash on empty input").unwrap();
        gitutils::commit(&repo, "chore: update deps").unwrap();

        let args = ChangelogArgs {
            from: Some("v1.0.0".to_string()),
            pattern: Some("v{major}.{minor}.{patch}".to_string()),
            source: VersionSourceName::Tag,
            group: vec![],
            other: None,
            title: None,
            to: None,
            overwrite: false,
            output: None,
            format: OutputFormat::Text,
        };

        // Should not error and should produce non-empty output.
        changelog(td.path(), &args).unwrap();
    }

    #[test]
    fn test_changelog_custom_rules() {
        let (td, repo) = testutils::init_repo();
        let (_remote_td, mut remote) = testutils::init_remote(&repo);

        create_new_remote_tag(&repo, &mut remote, "v1.0.0", false);
        gitutils::checkout_tag(&repo, "v1.0.0").unwrap();
        gitutils::commit(&repo, "BUMP_MAJOR: api overhaul").unwrap();
        gitutils::commit(&repo, "ADD: new endpoint").unwrap();

        let args = ChangelogArgs {
            from: Some("v1.0.0".to_string()),
            pattern: Some("v{major}.{minor}.{patch}".to_string()),
            source: VersionSourceName::Tag,
            group: vec![
                "Breaking:BUMP_MAJOR:".to_string(),
                "Additions:^ADD:".to_string(),
            ],
            other: None,
            title: None,
            to: None,
            overwrite: false,
            output: None,
            format: OutputFormat::Text,
        };

        // Custom rules should categorize without error.
        changelog(td.path(), &args).unwrap();
    }

    /// It uses `{semver}` to find the changelog baseline version.
    #[test]
    fn test_changelog_uses_semver_alias_for_baseline_lookup() {
        // Given
        let (td, repo) = testutils::init_repo();
        let (_remote_td, mut remote) = testutils::init_remote(&repo);

        create_new_remote_tag(&repo, &mut remote, "mobile@26.33.0", false);
        gitutils::checkout_tag(&repo, "mobile@26.33.0").unwrap();
        gitutils::commit(&repo, "feat: add mobile checkout").unwrap();

        let output_path = format!("{}/out.txt", td.path().display());
        let args = ChangelogArgs {
            from: None,
            pattern: Some("mobile@{semver}".to_string()),
            source: VersionSourceName::Tag,
            group: vec![],
            other: None,
            title: None,
            to: None,
            overwrite: false,
            output: Some(output_path.clone()),
            format: OutputFormat::Text,
        };

        // When
        changelog(td.path(), &args).unwrap();

        // Then
        let content = std::fs::read_to_string(output_path).unwrap();
        assert!(content.contains("Changelog since mobile@26.33.0"));
        assert!(content.contains("add mobile checkout"));
    }

    #[test]
    fn test_changelog_historical_range() {
        let (td, repo) = testutils::init_repo();
        let (_remote_td, mut remote) = testutils::init_remote(&repo);

        create_new_remote_tag(&repo, &mut remote, "v1.0.0", false);
        gitutils::commit(&repo, "✨ Add feature A").unwrap();
        gitutils::commit(&repo, "🐛 Fix bug B").unwrap();
        create_new_remote_tag(&repo, &mut remote, "v1.1.0", false);
        // commits after v1.1.0 — should NOT appear in the v1.0.0..v1.1.0 slice
        gitutils::commit(&repo, "✨ Add feature C").unwrap();

        let args = ChangelogArgs {
            from: Some("v1.0.0".to_string()),
            to: Some("v1.1.0".to_string()),
            pattern: Some("v{major}.{minor}.{patch}".to_string()),
            source: VersionSourceName::Tag,
            group: vec![],
            other: None,
            title: None,
            overwrite: false,
            output: Some(format!("{}/out.txt", td.path().display())),
            format: OutputFormat::Text,
        };

        changelog(td.path(), &args).unwrap();

        let content = std::fs::read_to_string(format!("{}/out.txt", td.path().display())).unwrap();
        assert!(
            content.contains("feature A"),
            "should include commits up to v1.1.0"
        );
        assert!(
            content.contains("bug B"),
            "should include commits up to v1.1.0"
        );
        assert!(
            !content.contains("feature C"),
            "should exclude commits after v1.1.0"
        );
    }

    #[test]
    fn test_changelog_no_prior_tag() {
        let (td, repo) = testutils::init_repo();
        let (_remote_td, _remote) = testutils::init_remote(&repo);

        gitutils::commit(&repo, "✨ Add initial feature").unwrap();
        gitutils::commit(&repo, "🐛 Fix startup crash").unwrap();

        let args = ChangelogArgs {
            from: None,
            pattern: Some("v{major}.{minor}.{patch}".to_string()),
            source: VersionSourceName::Tag,
            group: vec!["New Features:^✨".to_string(), "Bug Fixes:^🐛".to_string()],
            other: None,
            title: None,
            to: None,
            overwrite: false,
            output: None,
            format: OutputFormat::Text,
        };

        let out_path = format!("{}/out.txt", td.path().display());
        let args = ChangelogArgs {
            output: Some(out_path.clone()),
            ..args
        };
        changelog(td.path(), &args).unwrap();

        let content = std::fs::read_to_string(&out_path).unwrap();
        assert!(
            content.contains("initial feature"),
            "should include all commits when no prior tag"
        );
        assert!(
            content.contains("startup crash"),
            "should include all commits when no prior tag"
        );
    }

    #[test]
    fn test_changelog_no_prior_tag_json_from_is_null() {
        let (td, repo) = testutils::init_repo();
        let (_remote_td, _remote) = testutils::init_remote(&repo);

        gitutils::commit(&repo, "✨ Add initial feature").unwrap();

        let args = ChangelogArgs {
            from: None,
            pattern: Some("v{major}.{minor}.{patch}".to_string()),
            source: VersionSourceName::Tag,
            group: vec![],
            other: None,
            title: None,
            to: None,
            overwrite: false,
            output: Some(format!("{}/out.json", td.path().display())),
            format: OutputFormat::Json,
        };

        changelog(td.path(), &args).unwrap();

        let content = std::fs::read_to_string(format!("{}/out.json", td.path().display())).unwrap();
        let v: serde_json::Value = serde_json::from_str(&content).expect("valid JSON");
        assert!(v["from"].is_null(), "from should be null when no prior tag");
    }

    #[test]
    fn test_changelog_to_nonexistent_tag_errors() {
        let (td, repo) = testutils::init_repo();
        let (_remote_td, _remote) = testutils::init_remote(&repo);

        gitutils::commit(&repo, "✨ Add feature").unwrap();

        let args = ChangelogArgs {
            from: None,
            pattern: Some("v{major}.{minor}.{patch}".to_string()),
            source: VersionSourceName::Tag,
            group: vec![],
            other: None,
            title: None,
            to: Some("v99.0.0".to_string()),
            overwrite: false,
            output: None,
            format: OutputFormat::Text,
        };

        assert!(changelog(td.path(), &args).is_err());
    }
}

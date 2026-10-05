use std::path::Path;

use crate::cli::{LastVersionArgs, OutputFormat};
use crate::error::FlophaError;
use crate::gitutils;

pub fn last_version(path: &Path, args: &LastVersionArgs) -> Result<Option<String>, FlophaError> {
    let repo = gitutils::get_repo(path)?;
    gitutils::try_fetch_origin(&repo);
    let pattern = super::pattern_or_default(&args.pattern);
    let versioner = super::versioner_factory(&repo, pattern, &args.source);
    if let Some(version) = versioner.last_version() {
        match args.format {
            OutputFormat::Json => println!("{}", serde_json::json!({"version": version.tag})),
            OutputFormat::Text => println!("{}", version.tag),
        }

        if args.checkout {
            let version_source = super::version_source_factory(&args.source);
            version_source.checkout(&repo, &version.tag)?;
        }

        Ok(Some(version.tag))
    } else {
        super::print_none(&args.format, "No version found");
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{OutputFormat, VersionSourceName};
    use crate::testutils::{self, create_new_remote_branch, create_new_remote_tag};

    #[test]
    fn test_last_version_tag_returns_latest_matching_pattern() {
        let (td, repo) = testutils::init_repo();
        let (_remote_td, mut remote) = testutils::init_remote(&repo);

        let tags = vec![
            "flopha@0.1.0",
            "flopha@1.0.0",
            "flopha@1.0.1",
            "flopha@1.1.1",
            "flopha@1.1.9",
            "flopha@2.10.11",
            "flopha@1.1.10",
            "flopha@2.9.9",
            "flopha@2.10.10",
            "v3.9.9",
        ];
        for tag in tags {
            create_new_remote_tag(&repo, &mut remote, tag, true);
        }

        let args = LastVersionArgs {
            pattern: Some("flopha@{major}.{minor}.{patch}".to_string()),
            source: VersionSourceName::Tag,
            checkout: false,
            format: OutputFormat::Text,
        };

        let result = last_version(td.path(), &args).unwrap();

        assert_eq!(result, Some("flopha@2.10.11".to_string()));
    }

    #[test]
    fn test_last_version_tag_returns_none_without_match() {
        let (td, repo) = testutils::init_repo();
        let (_remote_td, mut remote) = testutils::init_remote(&repo);

        let tags = vec!["v0.1.0", "v1.0.0", "v1.0.1"];
        for tag in tags {
            create_new_remote_tag(&repo, &mut remote, tag, true);
        }

        let args = LastVersionArgs {
            pattern: Some("flopha@{major}.{minor}.{patch}".to_string()),
            source: VersionSourceName::Tag,
            checkout: false,
            format: OutputFormat::Text,
        };
        let result = last_version(td.path(), &args).unwrap();

        assert_eq!(result, None);
    }

    #[test]
    fn test_last_version_tag_checkout_works() {
        let (td, repo) = testutils::init_repo();
        let (_remote_td, mut remote) = testutils::init_remote(&repo);

        let tags = vec![
            "flopha@0.1.0",
            "flopha@1.0.0",
            "flopha@1.0.1",
            "flopha@1.1.1",
            "flopha@1.1.2",
            "flopha@0.4.5",
        ];
        for tag in tags {
            create_new_remote_tag(&repo, &mut remote, tag, true);
        }

        let args = LastVersionArgs {
            pattern: Some("flopha@{major}.{minor}.{patch}".to_string()),
            source: VersionSourceName::Tag,
            checkout: true,
            format: OutputFormat::Text,
        };
        last_version(td.path(), &args).unwrap();

        let tag_id = repo.revparse_single("refs/tags/flopha@1.1.2").unwrap().id();
        let head_id = repo.head().unwrap().peel_to_commit().unwrap().id();
        assert_eq!(tag_id, head_id);
    }

    #[test]
    fn test_last_version_tag_returns_none_with_non_matching_pattern() {
        let (td, repo) = testutils::init_repo();
        let (_remote_td, mut remote) = testutils::init_remote(&repo);

        let tags = vec!["v1.0.0", "v1.1.0", "v2.0.0"];
        for tag in tags {
            create_new_remote_tag(&repo, &mut remote, tag, false);
        }

        let args = LastVersionArgs {
            pattern: Some("release-{major}.{minor}.{patch}".to_string()),
            source: VersionSourceName::Tag,
            checkout: false,
            format: OutputFormat::Text,
        };

        let result = last_version(td.path(), &args).unwrap();

        assert_eq!(result, None);
    }

    #[test]
    fn test_last_version_returns_last_version_with_given_pattern_for_branches() {
        let (td, repo) = testutils::init_repo();
        let (_remote_td, mut remote) = testutils::init_remote(&repo);

        let branches = vec![
            "release/0.1.0",
            "release/1.0.0",
            "release/1.0.1",
            "release/1.1.1",
            "release/1.1.9",
            "release/2.10.11",
            "release/1.1.10",
            "release/2.9.9",
            "release/2.10.10",
        ];
        for branch in branches {
            create_new_remote_branch(&repo, &mut remote, branch);
        }

        let args = LastVersionArgs {
            pattern: Some("release/{major}.{minor}.{patch}".to_string()),
            source: VersionSourceName::Branch,
            checkout: false,
            format: OutputFormat::Text,
        };

        let result = last_version(td.path(), &args).unwrap();

        assert_eq!(result, Some("release/2.10.11".to_string()));
    }

    #[test]
    fn test_last_version_branch_returns_latest_matching_pattern() {
        let (td, repo) = testutils::init_repo();
        let (_remote_td, mut remote) = testutils::init_remote(&repo);

        let branches = vec![
            "release/1.0.0",
            "release/1.1.0",
            "release/2.0.0",
            "main",
            "develop",
        ];
        for branch in branches {
            create_new_remote_branch(&repo, &mut remote, branch);
        }

        let args = LastVersionArgs {
            pattern: Some("release/{major}.{minor}.{patch}".to_string()),
            source: VersionSourceName::Branch,
            checkout: false,
            format: OutputFormat::Text,
        };

        let result = last_version(td.path(), &args).unwrap();

        assert_eq!(result, Some("release/2.0.0".to_string()));
    }

    #[test]
    fn test_last_version_branch_checkout_works() {
        let (td, repo) = testutils::init_repo();
        let (_remote_td, mut remote) = testutils::init_remote(&repo);

        let branches = vec![
            "release/1.0.0",
            "release/1.1.0",
            "release/2.0.0",
            "release/2.1.0",
        ];
        for branch in branches {
            create_new_remote_branch(&repo, &mut remote, branch);
        }

        let args = LastVersionArgs {
            pattern: Some("release/{major}.{minor}.{patch}".to_string()),
            source: VersionSourceName::Branch,
            checkout: true,
            format: OutputFormat::Text,
        };
        last_version(td.path(), &args).unwrap();

        let branch_id = repo
            .revparse_single("refs/heads/release/2.1.0")
            .unwrap()
            .id();
        let head_id = repo.head().unwrap().peel_to_commit().unwrap().id();
        assert_eq!(branch_id, head_id);
    }

    #[test]
    fn test_version_json_escapes_special_chars() {
        // Verifies the serde_json::json! pattern used in last_version / next_version.
        let tricky = r#"v1.0.0"snapshot""#;
        let json = serde_json::json!({"version": tricky}).to_string();
        let v: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
        assert_eq!(v["version"], tricky);
    }
}

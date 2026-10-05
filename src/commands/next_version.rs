use std::path::Path;

use crate::cli::{NextVersionArgs, OutputFormat, VersionSourceName};
use crate::error::FlophaError;
use crate::gitutils;
use crate::versioning::{self, Versioner};

pub fn next_version(path: &Path, args: &NextVersionArgs) -> Result<Option<String>, FlophaError> {
    if args.tag_message.is_some() {
        if let VersionSourceName::Branch = args.source {
            return Err(FlophaError::InvalidArgs(
                "--tag-message is only valid with --source tag".to_string(),
            ));
        }
    }

    let repo = gitutils::get_repo(path)?;
    gitutils::try_fetch_origin(&repo);
    let pattern = super::pattern_or_default(&args.pattern);

    let version_source = super::version_source_factory(&args.source);
    let versioner = Versioner::new(version_source.fetch_all(&repo), pattern);

    let increment = super::resolve_increment(
        &repo,
        versioner.last_version().as_ref(),
        args.auto,
        &args.rule,
        args.increment.clone(),
    )?;

    let next = match versioner.next_version(increment)? {
        Some(v) => v,
        None => {
            super::print_none(&args.format, "No version found");
            return Ok(None);
        }
    };

    let final_tag = if let Some(channel) = &args.pre {
        let n = super::next_pre_release_number(&repo, &next.tag, channel);
        versioning::pre_release(&next.tag, channel, n)
    } else {
        next.tag.clone()
    };

    match args.format {
        OutputFormat::Json => println!("{}", serde_json::json!({"version": final_tag})),
        OutputFormat::Text => println!("{}", final_tag),
    }

    if args.create {
        version_source.create(&repo, &final_tag, args.tag_message.as_deref())?;

        if args.push {
            let mut remote = gitutils::get_remote(&repo, "origin")?;
            match args.source {
                VersionSourceName::Tag => gitutils::push_tag(&mut remote, &final_tag)?,
                VersionSourceName::Branch => {
                    let mut branch = repo.find_branch(&final_tag, git2::BranchType::Local)?;
                    gitutils::push_branch(&mut remote, &mut branch)?;
                }
            }
        }
    }

    Ok(Some(final_tag))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{OutputFormat, VersionSourceName};
    use crate::gitutils;
    use crate::testutils::{self, create_new_remote_branch, create_new_remote_tag};
    use crate::versioning::Increment;

    #[test]
    fn test_next_version_returns_next_version_with_given_pattern() {
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
            create_new_remote_tag(&repo, &mut remote, tag, false);
        }
        gitutils::checkout_tag(&repo, "flopha@2.10.11").unwrap();
        gitutils::commit(&repo, "New commit").unwrap();

        let args = NextVersionArgs {
            pattern: Some("flopha@{major}.{minor}.{patch}".to_string()),
            increment: Increment::Patch,
            auto: false,
            rule: vec![],
            pre: None,
            source: VersionSourceName::Tag,
            create: false,
            format: OutputFormat::Text,
            tag_message: None,
            push: false,
        };
        let result = next_version(td.path(), &args).unwrap();

        assert_eq!(result, Some("flopha@2.10.12".to_string()))
    }

    #[test]
    fn test_next_version_with_tag_create_action() {
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
            create_new_remote_tag(&repo, &mut remote, tag, false);
        }
        gitutils::checkout_tag(&repo, "flopha@1.1.2").unwrap();
        gitutils::commit(&repo, "New commit").unwrap();

        let args = NextVersionArgs {
            pattern: Some("flopha@{major}.{minor}.{patch}".to_string()),
            increment: Increment::Patch,
            auto: false,
            rule: vec![],
            pre: None,
            source: VersionSourceName::Tag,
            create: true,
            format: OutputFormat::Text,
            tag_message: None,
            push: false,
        };
        next_version(td.path(), &args).unwrap();

        let tag_id = repo.revparse_single("refs/tags/flopha@1.1.3").unwrap().id();
        let head_id = repo.head().unwrap().peel_to_commit().unwrap().id();
        assert_eq!(tag_id, head_id);
    }

    #[test]
    fn next_version_branch_returns_next_version_with_pattern() {
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
        gitutils::checkout_branch(&repo, "release/2.10.11", false).unwrap();
        gitutils::commit(&repo, "New commit").unwrap();

        let args = NextVersionArgs {
            pattern: Some("release/{major}.{minor}.{patch}".to_string()),
            increment: Increment::Patch,
            auto: false,
            rule: vec![],
            pre: None,
            source: VersionSourceName::Branch,
            create: false,
            format: OutputFormat::Text,
            tag_message: None,
            push: false,
        };
        let result = next_version(td.path(), &args).unwrap();

        assert_eq!(result, Some("release/2.10.12".to_string()))
    }

    #[test]
    fn test_next_version_branch_returns_none_without_match() {
        let (td, repo) = testutils::init_repo();
        let (_remote_td, mut remote) = testutils::init_remote(&repo);

        let branches = vec!["main", "develop", "feature/new-feature"];
        for branch in branches {
            create_new_remote_branch(&repo, &mut remote, branch);
        }

        let args = NextVersionArgs {
            pattern: Some("release/{major}.{minor}.{patch}".to_string()),
            increment: Increment::Patch,
            auto: false,
            rule: vec![],
            pre: None,
            source: VersionSourceName::Branch,
            create: false,
            format: OutputFormat::Text,
            tag_message: None,
            push: false,
        };

        let result = next_version(td.path(), &args).unwrap();

        assert_eq!(result, None);
    }

    #[test]
    fn test_next_version_branch_with_create_action() {
        let (td, repo) = testutils::init_repo();
        let (_remote_td, mut remote) = testutils::init_remote(&repo);

        let branches = vec!["release/1.0.0", "release/1.1.0", "release/2.0.0"];
        for branch in branches {
            create_new_remote_branch(&repo, &mut remote, branch);
        }
        gitutils::checkout_branch(&repo, "release/2.0.0", false).unwrap();
        gitutils::commit(&repo, "New commit").unwrap();

        let args = NextVersionArgs {
            pattern: Some("release/{major}.{minor}.{patch}".to_string()),
            increment: Increment::Minor,
            auto: false,
            rule: vec![],
            pre: None,
            source: VersionSourceName::Branch,
            create: true,
            format: OutputFormat::Text,
            tag_message: None,
            push: false,
        };
        let result = next_version(td.path(), &args).unwrap();

        assert_eq!(result, Some("release/2.1.0".to_string()));

        let branches = repo.branches(Some(git2::BranchType::Local)).unwrap();
        assert!(branches.into_iter().any(|b| {
            let (branch, _) = b.unwrap();
            branch.name().unwrap() == Some("release/2.1.0")
        }));
    }

    #[test]
    fn test_next_version_auto_detects_feat_as_minor() {
        let (td, repo) = testutils::init_repo();
        let (_remote_td, mut remote) = testutils::init_remote(&repo);

        let tags = vec!["v1.0.0", "v1.1.0"];
        for tag in tags {
            create_new_remote_tag(&repo, &mut remote, tag, false);
        }
        gitutils::checkout_tag(&repo, "v1.1.0").unwrap();
        gitutils::commit(&repo, "feat: add new command").unwrap();

        let args = NextVersionArgs {
            pattern: Some("v{major}.{minor}.{patch}".to_string()),
            increment: Increment::Patch,
            auto: true,
            rule: vec![],
            pre: None,
            source: VersionSourceName::Tag,
            create: false,
            format: OutputFormat::Text,
            tag_message: None,
            push: false,
        };
        let result = next_version(td.path(), &args).unwrap();

        assert_eq!(result, Some("v1.2.0".to_string()));
    }

    #[test]
    fn test_next_version_pre_release_starts_at_1() {
        let (td, repo) = testutils::init_repo();
        let (_remote_td, mut remote) = testutils::init_remote(&repo);

        let tags = vec!["v1.0.0"];
        for tag in tags {
            create_new_remote_tag(&repo, &mut remote, tag, false);
        }
        gitutils::checkout_tag(&repo, "v1.0.0").unwrap();
        gitutils::commit(&repo, "fix: something").unwrap();

        let args = NextVersionArgs {
            pattern: Some("v{major}.{minor}.{patch}".to_string()),
            increment: Increment::Patch,
            auto: false,
            rule: vec![],
            pre: Some("alpha".to_string()),
            source: VersionSourceName::Tag,
            create: false,
            format: OutputFormat::Text,
            tag_message: None,
            push: false,
        };
        let result = next_version(td.path(), &args).unwrap();

        assert_eq!(result, Some("v1.0.1-alpha.1".to_string()));
    }

    #[test]
    fn test_next_version_pre_release_increments() {
        let (td, repo) = testutils::init_repo();
        let (_remote_td, mut remote) = testutils::init_remote(&repo);

        let tags = vec!["v1.0.0", "v1.0.1-alpha.1"];
        for tag in tags {
            create_new_remote_tag(&repo, &mut remote, tag, false);
        }
        gitutils::checkout_tag(&repo, "v1.0.0").unwrap();
        gitutils::commit(&repo, "fix: something").unwrap();

        let args = NextVersionArgs {
            pattern: Some("v{major}.{minor}.{patch}".to_string()),
            increment: Increment::Patch,
            auto: false,
            rule: vec![],
            pre: Some("alpha".to_string()),
            source: VersionSourceName::Tag,
            create: false,
            format: OutputFormat::Text,
            tag_message: None,
            push: false,
        };
        let result = next_version(td.path(), &args).unwrap();

        assert_eq!(result, Some("v1.0.1-alpha.2".to_string()));
    }

    #[test]
    fn test_next_version_auto_with_custom_rules() {
        let (td, repo) = testutils::init_repo();
        let (_remote_td, mut remote) = testutils::init_remote(&repo);

        let tags = vec!["v1.0.0"];
        for tag in tags {
            create_new_remote_tag(&repo, &mut remote, tag, false);
        }
        gitutils::checkout_tag(&repo, "v1.0.0").unwrap();
        gitutils::commit(&repo, "feat: add thing").unwrap();

        let args = NextVersionArgs {
            pattern: Some("v{major}.{minor}.{patch}".to_string()),
            increment: Increment::Patch,
            auto: true,
            rule: vec!["major:BUMP_MAJOR:".to_string()],
            pre: None,
            source: VersionSourceName::Tag,
            create: false,
            format: OutputFormat::Text,
            tag_message: None,
            push: false,
        };
        let result = next_version(td.path(), &args).unwrap();

        assert_eq!(result, Some("v1.0.1".to_string()));
    }

    #[test]
    fn test_next_version_annotated_tag_created_when_tag_message_set() {
        let (td, repo) = testutils::init_repo();
        let (_remote_td, mut remote) = testutils::init_remote(&repo);

        create_new_remote_tag(&repo, &mut remote, "v1.0.0", false);
        gitutils::checkout_tag(&repo, "v1.0.0").unwrap();
        gitutils::commit(&repo, "fix: something").unwrap();

        let args = NextVersionArgs {
            pattern: Some("v{major}.{minor}.{patch}".to_string()),
            increment: Increment::Patch,
            auto: false,
            rule: vec![],
            pre: None,
            source: VersionSourceName::Tag,
            create: true,
            tag_message: Some("Release v1.0.1".to_string()),
            push: false,
            format: OutputFormat::Text,
        };
        next_version(td.path(), &args).unwrap();

        let tag_obj = repo.revparse_single("refs/tags/v1.0.1").unwrap();
        assert_eq!(
            tag_obj.kind(),
            Some(git2::ObjectType::Tag),
            "expected annotated tag object"
        );
    }

    #[test]
    fn test_tag_message_with_branch_source_returns_error() {
        let (td, repo) = testutils::init_repo();
        let (_remote_td, mut remote) = testutils::init_remote(&repo);

        create_new_remote_branch(&repo, &mut remote, "release/1.0.0");
        gitutils::checkout_branch(&repo, "release/1.0.0", false).unwrap();
        gitutils::commit(&repo, "New commit").unwrap();

        let args = NextVersionArgs {
            pattern: Some("release/{major}.{minor}.{patch}".to_string()),
            increment: Increment::Patch,
            auto: false,
            rule: vec![],
            pre: None,
            source: VersionSourceName::Branch,
            create: true,
            tag_message: Some("should fail".to_string()),
            push: false,
            format: OutputFormat::Text,
        };
        let result = next_version(td.path(), &args);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("--tag-message is only valid with --source tag"));
    }

    #[test]
    fn test_next_version_push_tag_reaches_remote() {
        let (td, repo) = testutils::init_repo();
        let (remote_td, mut remote) = testutils::init_remote(&repo);

        create_new_remote_tag(&repo, &mut remote, "v1.0.0", false);
        gitutils::checkout_tag(&repo, "v1.0.0").unwrap();
        gitutils::commit(&repo, "fix: something").unwrap();

        let args = NextVersionArgs {
            pattern: Some("v{major}.{minor}.{patch}".to_string()),
            increment: Increment::Patch,
            auto: false,
            rule: vec![],
            pre: None,
            source: VersionSourceName::Tag,
            create: true,
            tag_message: None,
            push: true,
            format: OutputFormat::Text,
        };
        next_version(td.path(), &args).unwrap();

        let remote_repo = git2::Repository::open(remote_td.path()).unwrap();
        let tag_names = remote_repo.tag_names(None).unwrap();
        assert!(
            tag_names.iter().any(|t| t == Some("v1.0.1")),
            "v1.0.1 tag should have been pushed to remote"
        );
    }

    #[test]
    fn test_next_version_push_branch_reaches_remote() {
        let (td, repo) = testutils::init_repo();
        let (remote_td, mut remote) = testutils::init_remote(&repo);

        for branch in ["release/1.0.0", "release/2.0.0"] {
            create_new_remote_branch(&repo, &mut remote, branch);
        }
        gitutils::checkout_branch(&repo, "release/2.0.0", false).unwrap();
        gitutils::commit(&repo, "New commit").unwrap();

        let args = NextVersionArgs {
            pattern: Some("release/{major}.{minor}.{patch}".to_string()),
            increment: Increment::Minor,
            auto: false,
            rule: vec![],
            pre: None,
            source: VersionSourceName::Branch,
            create: true,
            tag_message: None,
            push: true,
            format: OutputFormat::Text,
        };
        next_version(td.path(), &args).unwrap();

        let remote_repo = git2::Repository::open(remote_td.path()).unwrap();
        let branches = remote_repo.branches(Some(git2::BranchType::Local)).unwrap();
        assert!(
            branches.into_iter().any(|b| {
                let (branch, _) = b.unwrap();
                branch.name().unwrap() == Some("release/2.1.0")
            }),
            "release/2.1.0 branch should have been pushed to remote"
        );
    }
}

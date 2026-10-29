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

    let next = versioner.next_version(increment, args.initial_version.as_deref())?
        .ok_or_else(|| FlophaError::InvalidArgs(
            "no version matches --pattern; supply --initial-version <MAJOR.MINOR.PATCH> to create the first version".to_string()
        ))?;

    let final_tag = if let Some(channel) = &args.pre {
        let n = super::next_pre_release_number(&repo, version_source.as_ref(), &next.tag, channel);
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

    /// A supplied initial version creates the first matching tag or branch without an auto bump.
    #[test]
    fn initial_version_creates_first_reference() {
        use crate::cli::{Cli, Commands};
        use clap::Parser;

        for (source, pattern, expected) in [
            ("tag", "v{semver}", "v2.3.4"),
            ("tag", "mobile@{major}.{minor}.{patch}", "mobile@2.3.4"),
            ("branch", "release/{semver}", "release/2.3.4"),
        ] {
            // Given a repository with unrelated tags and a breaking change
            let (td, repo) = testutils::init_repo();
            testutils::tag_head(&repo, "unrelated@8.0.0");
            gitutils::commit(&repo, "feat!: first feature").unwrap();
            let cli = Cli::try_parse_from([
                "flopha",
                "next-version",
                "--initial-version",
                "2.3.4",
                "--source",
                source,
                "--pattern",
                pattern,
                "--auto",
                "--create",
            ])
            .unwrap();
            let Some(Commands::NextVersion(args)) = cli.command else {
                panic!("expected next-version")
            };

            // When creating the first version
            let result = next_version(td.path(), &args).unwrap();

            // Then the initial version is rendered through the pattern and points at HEAD
            assert_eq!(result.as_deref(), Some(expected));
            let reference = format!(
                "refs/{}/{}",
                if source == "tag" { "tags" } else { "heads" },
                expected
            );
            assert_eq!(
                repo.revparse_single(&reference)
                    .unwrap()
                    .peel_to_commit()
                    .unwrap()
                    .id(),
                repo.head().unwrap().peel_to_commit().unwrap().id()
            );
        }
    }

    /// Successive initial pre-release branches have distinct counters and point at each release's commit.
    #[test]
    fn initial_pre_release_branches_track_each_commit() {
        use crate::cli::{Cli, Commands};
        use clap::Parser;

        // Given a repository without any release branches or tags
        let (td, repo) = testutils::init_repo();
        let first_commit = repo.head().unwrap().peel_to_commit().unwrap().id();
        let cli = Cli::try_parse_from([
            "flopha",
            "next-version",
            "--source",
            "branch",
            "--pattern",
            "release/{semver}",
            "--initial-version",
            "2.3.4",
            "--pre",
            "beta",
            "--create",
        ])
        .unwrap();
        let Some(Commands::NextVersion(args)) = cli.command else {
            panic!("expected next-version")
        };

        // When creating the first branch and then releasing a new commit from main
        assert_eq!(
            next_version(td.path(), &args).unwrap().as_deref(),
            Some("release/2.3.4-beta.1")
        );
        gitutils::checkout_branch(&repo, "main", false).unwrap();
        let second_commit = gitutils::commit(&repo, "feat: second release").unwrap();
        assert_eq!(
            next_version(td.path(), &args).unwrap().as_deref(),
            Some("release/2.3.4-beta.2")
        );

        // Then each branch preserves its intended commit and HEAD selects the new release
        assert_eq!(
            repo.find_branch("release/2.3.4-beta.1", git2::BranchType::Local)
                .unwrap()
                .get()
                .target(),
            Some(first_commit)
        );
        assert_eq!(
            repo.find_branch("release/2.3.4-beta.2", git2::BranchType::Local)
                .unwrap()
                .get()
                .target(),
            Some(second_commit)
        );
        assert_eq!(repo.head().unwrap().target(), Some(second_commit));
        assert_eq!(
            repo.head().unwrap().name(),
            Some("refs/heads/release/2.3.4-beta.2")
        );
        assert!(repo.tag_names(None).unwrap().is_empty());
    }

    /// Initial pre-releases continue the channel counter and can later produce the stable initial tag.
    #[test]
    fn initial_pre_release_then_stable() {
        use crate::cli::{Cli, Commands};
        use clap::Parser;

        // Given pre-release tags but no matching stable tag
        let (td, repo) = testutils::init_repo();
        testutils::tag_head(&repo, "mobile@2.3.4-beta.2");
        testutils::tag_head(&repo, "mobile@2.3.4-alpha.9");
        repo.branch(
            "mobile@2.3.4-beta.99",
            &repo.head().unwrap().peel_to_commit().unwrap(),
            false,
        )
        .unwrap();
        let cli = Cli::try_parse_from([
            "flopha",
            "next-version",
            "--initial-version",
            "2.3.4",
            "--pre",
            "beta",
            "--pattern",
            "mobile@{semver}",
            "--increment",
            "major",
            "--create",
        ])
        .unwrap();
        let Some(Commands::NextVersion(mut args)) = cli.command else {
            panic!("expected next-version")
        };

        // When creating another pre-release and then the stable version
        assert_eq!(
            next_version(td.path(), &args).unwrap().as_deref(),
            Some("mobile@2.3.4-beta.3")
        );
        args.pre = None;
        assert_eq!(
            next_version(td.path(), &args).unwrap().as_deref(),
            Some("mobile@2.3.4")
        );

        // Then the stable tag exists and future versions use the requested bump
        assert!(repo.revparse_single("refs/tags/mobile@2.3.4").is_ok());
        args.create = false;
        assert_eq!(
            next_version(td.path(), &args).unwrap().as_deref(),
            Some("mobile@3.0.0")
        );
    }

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
            initial_version: Some("9.0.0".to_string()),
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
            initial_version: None,
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
            initial_version: None,
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

    /// Creating a release branch without a prior match requires an explicit initial version.
    #[test]
    fn test_next_version_branch_requires_initial_without_match() {
        let (td, repo) = testutils::init_repo();
        let (_remote_td, mut remote) = testutils::init_remote(&repo);

        let branches = vec!["main", "develop", "feature/new-feature"];
        for branch in branches {
            create_new_remote_branch(&repo, &mut remote, branch);
        }

        let args = NextVersionArgs {
            initial_version: None,
            pattern: Some("release/{major}.{minor}.{patch}".to_string()),
            increment: Increment::Patch,
            auto: true,
            rule: vec![],
            pre: None,
            source: VersionSourceName::Branch,
            create: true,
            format: OutputFormat::Text,
            tag_message: None,
            push: false,
        };

        let err = next_version(td.path(), &args).unwrap_err();

        assert!(err.to_string().contains("--initial-version"));
        assert!(repo
            .find_branch("release/0.1.0", git2::BranchType::Local)
            .is_err());
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
            initial_version: None,
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
            initial_version: None,
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
            initial_version: None,
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
            initial_version: None,
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
            initial_version: None,
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
            initial_version: None,
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
            initial_version: None,
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
            initial_version: None,
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
            initial_version: None,
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

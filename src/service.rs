use std::path::Path;

use crate::cli::{
    LastVersionAction, LastVersionArgs, NextVersionAction, NextVersionArgs, VersionSourceName,
};
use crate::gitutils::{self, CommandOptions};
use crate::version_source::{BranchVersionSource, TagVersionSource, VersionSource};
use crate::versioning::Versioner;

pub fn last_version(path: &Path, args: &LastVersionArgs) -> Option<String> {
    let repo = gitutils::get_repo(path);
    let mut remote = gitutils::get_remote(&repo, "origin");
    let opts = CommandOptions {
        verbose: args.verbose,
    };
    gitutils::fetch_all(&mut remote, Some(&opts)).expect("Failed to fetch from remote");
    let pattern = args
        .pattern
        .clone()
        .unwrap_or("v{major}.{minor}.{patch}".to_string());
    let versioner = versioner_factory(&repo, pattern, &args.source);
    if let Some(version) = versioner.last_version() {
        match args.action {
            LastVersionAction::Checkout => {
                let version_source = version_source_factory(&args.source);
                version_source
                    .checkout(&repo, &version.tag)
                    .expect("Failed to checkout version");
            }
            LastVersionAction::Print => {
                println!("{}", version.tag);
            }
        }

        Some(version.tag)
    } else {
        println!("No version found");
        None
    }
}

pub fn next_version(path: &Path, args: &NextVersionArgs) -> Option<String> {
    let repo = gitutils::get_repo(path);
    let mut remote = gitutils::get_remote(&repo, "origin");
    let opts = CommandOptions {
        verbose: args.verbose,
    };
    gitutils::fetch_all(&mut remote, Some(&opts)).expect("Failed to fetch from remote");
    let pattern = args
        .pattern
        .clone()
        .unwrap_or("v{major}.{minor}.{patch}".to_string());
    let versioner = versioner_factory(&repo, pattern, &args.source);
    if let Some(version) = versioner.next_version(args.increment.clone()) {
        match args.action {
            NextVersionAction::Create => {
                let version_source = version_source_factory(&args.source);
                version_source
                    .create(&repo, &version.tag)
                    .expect("Failed to create new version");
            }
            NextVersionAction::Print => {
                println!("{}", version.tag);
            }
        }
        Some(version.tag)
    } else {
        println!("No version found");
        None
    }
}

fn version_source_factory(source: &VersionSourceName) -> Box<dyn VersionSource> {
    match source {
        VersionSourceName::Branch => Box::new(BranchVersionSource),
        VersionSourceName::Tag => Box::new(TagVersionSource),
    }
}

fn versioner_factory(
    repo: &git2::Repository,
    pattern: String,
    source: &VersionSourceName,
) -> Versioner {
    let version_source = version_source_factory(source);
    let versions = version_source.fetch_all(repo);
    Versioner::new(versions, pattern)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::LastVersionArgs;
    use crate::versioning::Increment;
    use crate::{gitutils, testutils};

    // Tests for last_version function
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
            verbose: false,
            source: VersionSourceName::Tag,
            action: LastVersionAction::Print,
        };

        let result = last_version(td.path(), &args);

        assert_eq!(result.unwrap(), "flopha@2.10.11");
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
            verbose: false,
            source: VersionSourceName::Tag,
            action: LastVersionAction::Print,
        };
        let result = last_version(td.path(), &args);

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

        // When
        let args = LastVersionArgs {
            pattern: Some("flopha@{major}.{minor}.{patch}".to_string()),
            verbose: false,
            source: VersionSourceName::Tag,
            action: LastVersionAction::Checkout,
        };
        last_version(td.path(), &args);

        // Then
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
            verbose: false,
            source: VersionSourceName::Tag,
            action: LastVersionAction::Print,
        };

        let result = last_version(td.path(), &args);

        assert_eq!(result, None);
    }

    #[test]
    fn test_last_version_returns_last_version_with_given_pattern_for_branches() {
        // Given
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

        // When
        let args = LastVersionArgs {
            pattern: Some("release/{major}.{minor}.{patch}".to_string()),
            verbose: false,
            source: VersionSourceName::Branch,
            action: LastVersionAction::Print,
        };

        let result = last_version(td.path(), &args);

        // Then
        assert_eq!(result.unwrap(), "release/2.10.11");
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
            verbose: false,
            source: VersionSourceName::Branch,
            action: LastVersionAction::Print,
        };

        let result = last_version(td.path(), &args);

        assert_eq!(result.unwrap(), "release/2.0.0");
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

        // When
        let args = LastVersionArgs {
            pattern: Some("release/{major}.{minor}.{patch}".to_string()),
            verbose: false,
            source: VersionSourceName::Branch,
            action: LastVersionAction::Checkout,
        };
        last_version(td.path(), &args);

        // Then
        let branch_id = repo
            .revparse_single("refs/heads/release/2.1.0")
            .unwrap()
            .id();
        let head_id = repo.head().unwrap().peel_to_commit().unwrap().id();
        assert_eq!(branch_id, head_id);
    }

    // Tests for next_version function
    #[test]
    fn test_next_version_returns_next_version_with_given_pattern() {
        // Given
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
        gitutils::checkout_tag(&repo, "flopha@2.10.11", None).unwrap();
        gitutils::commit(&repo, "New commit").unwrap();

        // When
        let args = NextVersionArgs {
            pattern: Some("flopha@{major}.{minor}.{patch}".to_string()),
            increment: Increment::Patch,
            verbose: false,
            source: VersionSourceName::Tag,
            action: NextVersionAction::Print,
        };
        let result = next_version(td.path(), &args);

        // Then
        assert_eq!(result.unwrap(), "flopha@2.10.12")
    }

    #[test]
    fn test_next_version_with_tag_create_action() {
        // Given
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
        gitutils::checkout_tag(&repo, "flopha@1.1.2", None).unwrap();
        gitutils::commit(&repo, "New commit").unwrap();

        // When
        let args = NextVersionArgs {
            pattern: Some("flopha@{major}.{minor}.{patch}".to_string()),
            increment: Increment::Patch,
            verbose: false,
            source: VersionSourceName::Tag,
            action: NextVersionAction::Create,
        };
        next_version(td.path(), &args);

        // Then
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
        gitutils::checkout_branch(&repo, "release/2.10.11", false, None).unwrap();
        gitutils::commit(&repo, "New commit").unwrap();

        // When
        let args = NextVersionArgs {
            pattern: Some("release/{major}.{minor}.{patch}".to_string()),
            increment: Increment::Patch,
            verbose: false,
            source: VersionSourceName::Branch,
            action: NextVersionAction::Print,
        };
        let result = next_version(td.path(), &args);

        // Then
        assert_eq!(result.unwrap(), "release/2.10.12")
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
            verbose: false,
            source: VersionSourceName::Branch,
            action: NextVersionAction::Print,
        };

        let result = next_version(td.path(), &args);

        assert_eq!(result, None);
    }

    #[test]
    fn test_next_version_branch_with_create_action() {
        // Given
        let (td, repo) = testutils::init_repo();
        let (_remote_td, mut remote) = testutils::init_remote(&repo);

        let branches = vec!["release/1.0.0", "release/1.1.0", "release/2.0.0"];
        for branch in branches {
            create_new_remote_branch(&repo, &mut remote, branch);
        }
        gitutils::checkout_branch(&repo, "release/2.0.0", false, None).unwrap();
        gitutils::commit(&repo, "New commit").unwrap();

        // When
        let args = NextVersionArgs {
            pattern: Some("release/{major}.{minor}.{patch}".to_string()),
            increment: Increment::Minor,
            verbose: false,
            source: VersionSourceName::Branch,
            action: NextVersionAction::Create,
        };
        let result = next_version(td.path(), &args);

        // Then
        assert_eq!(result.unwrap(), "release/2.1.0");

        // Verify that the new branch was created
        let branches = repo.branches(Some(git2::BranchType::Local)).unwrap();

        assert!(branches.into_iter().any(|b| {
            let (branch, _) = b.unwrap();
            branch.name().unwrap() == Some("release/2.1.0")
        }));
    }

    fn create_new_remote_tag(
        repo: &git2::Repository,
        remote: &mut git2::Remote,
        tag: &str,
        should_delete: bool,
    ) {
        let commit_id = gitutils::commit(&repo, "New commit").unwrap();
        gitutils::tag_oid(&repo, commit_id, tag).unwrap();
        remote.push(&[format!("refs/tags/{}", tag)], None).unwrap();

        if should_delete {
            repo.tag_delete(tag).unwrap(); // delete local tag
        }
    }

    fn create_new_remote_branch(repo: &git2::Repository, remote: &mut git2::Remote, branch: &str) {
        gitutils::checkout_branch(repo, branch, true, None).unwrap();
        gitutils::commit(repo, "New commit").unwrap();
        let mut branch = repo.find_branch(branch, git2::BranchType::Local).unwrap();
        gitutils::push_branch(remote, &mut branch, None).unwrap();
    }
}

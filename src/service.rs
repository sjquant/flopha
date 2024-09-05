use std::path::Path;

use crate::cli::{LastVersionArgs, NextVersionArgs};
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
    let versioner = versioner_factory(&repo, pattern, args.branch);
    if let Some(version) = versioner.last_version() {
        if args.checkout {
            let version_source = version_source_factory(args.branch);
            version_source
                .checkout(&repo, &version.tag)
                .expect("Failed to checkout version");
        }
        println!("{}", version.tag);
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
    let versioner = versioner_factory(&repo, pattern, args.branch);

    if let Some(version) = versioner.next_version(args.increment.clone()) {
        if args.tag || args.branch {
            let version_source = version_source_factory(args.branch);
            version_source
                .create_new(&repo, &version.tag)
                .expect("Failed to create new version");
        }
        println!("{}", version.tag);
        Some(version.tag)
    } else {
        println!("No version found");
        None
    }
}

fn version_source_factory(use_branches: bool) -> Box<dyn VersionSource> {
    if use_branches {
        Box::new(BranchVersionSource)
    } else {
        Box::new(TagVersionSource)
    }
}

fn versioner_factory(repo: &git2::Repository, pattern: String, use_branches: bool) -> Versioner {
    let version_source = version_source_factory(use_branches);
    let versions = version_source.get_all_versions(repo);
    Versioner::new(versions, pattern)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::LastVersionArgs;
    use crate::versioning::Increment;
    use crate::{gitutils, testutils};

    #[test]
    fn test_last_version_returns_last_version_with_given_pattern() {
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
            create_new_remote_tag(&repo, &mut remote, tag, true);
        }

        // When
        let args = LastVersionArgs {
            pattern: Some("flopha@{major}.{minor}.{patch}".to_string()),
            checkout: false,
            verbose: false,
            branch: false,
            tag: true,
        };

        let result = last_version(td.path(), &args);

        // Then
        assert_eq!(result.unwrap(), "flopha@2.10.11");
    }

    #[test]
    fn test_last_version_without_matching_version_returns_none() {
        // Given
        let (td, repo) = testutils::init_repo();
        let (_remote_td, mut remote) = testutils::init_remote(&repo);

        let tags = vec!["v0.1.0", "v1.0.0", "v1.0.1"];
        for tag in tags {
            create_new_remote_tag(&repo, &mut remote, tag, true);
        }

        // When
        let args = LastVersionArgs {
            pattern: Some("flopha@{major}.{minor}.{patch}".to_string()),
            checkout: false,
            verbose: false,
            branch: false,
            tag: true,
        };
        let result = last_version(td.path(), &args);

        // Then
        assert_eq!(result, None);
    }

    #[test]
    fn test_last_version_with_checkout_option() {
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
            create_new_remote_tag(&repo, &mut remote, tag, true);
        }

        // When
        let args = LastVersionArgs {
            pattern: Some("flopha@{major}.{minor}.{patch}".to_string()),
            checkout: true,
            verbose: false,
            branch: false,
            tag: true,
        };
        last_version(td.path(), &args);

        // Then
        let tag_id = repo.revparse_single("refs/tags/flopha@1.1.2").unwrap().id();
        let head_id = repo.head().unwrap().peel_to_commit().unwrap().id();
        assert_eq!(tag_id, head_id);
    }

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
            tag: false,
            verbose: false,
            branch: false,
        };
        let result = next_version(td.path(), &args);

        // Then
        assert_eq!(result.unwrap(), "flopha@2.10.12")
    }

    #[test]
    fn test_next_version_with_tag_option() {
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
            tag: true,
            verbose: false,
            branch: false,
        };
        next_version(td.path(), &args);

        // Then
        let tag_id = repo.revparse_single("refs/tags/flopha@1.1.3").unwrap().id();
        let head_id = repo.head().unwrap().peel_to_commit().unwrap().id();
        assert_eq!(tag_id, head_id);
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
            checkout: false,
            verbose: false,
            tag: false,
            branch: true,
        };

        let result = last_version(td.path(), &args);

        // Then
        assert_eq!(result.unwrap(), "release/2.10.11");
    }

    #[test]
    fn test_next_version_returns_next_version_with_given_pattern_for_branches() {
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
        gitutils::checkout_branch(&repo, "release/2.10.11", false, None).unwrap();
        gitutils::commit(&repo, "New commit").unwrap();

        // When
        let args = NextVersionArgs {
            pattern: Some("release/{major}.{minor}.{patch}".to_string()),
            increment: Increment::Patch,
            tag: false,
            verbose: false,
            branch: true,
        };
        let result = next_version(td.path(), &args);

        // Then
        assert_eq!(result.unwrap(), "release/2.10.12")
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

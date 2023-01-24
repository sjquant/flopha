use std::path::Path;

use crate::cli::{LastVersionArgs, NextVersionArgs};
use crate::gitutils;
use crate::versioning::Versioner;

pub fn last_version(path: &Path, args: &LastVersionArgs) -> Option<String> {
    let repo = gitutils::get_repo(path);
    let mut remote = gitutils::get_remote(&repo, "origin");
    gitutils::fetch_all(&mut remote).expect("Failed to fetch from remote");
    let pattern = args
        .pattern
        .clone()
        .unwrap_or("v{major}.{minor}.{patch}".to_string());
    let versioner = get_versioner(&repo, pattern);
    if let Some(version) = versioner.last_version() {
        if args.checkout {
            gitutils::checkout_tag(&repo, &version.tag).expect("Failed to checkout tag");
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
    gitutils::fetch_all(&mut remote).expect("Failed to fetch from remote");
    let pattern = args
        .pattern
        .clone()
        .unwrap_or("v{major}.{minor}.{patch}".to_string());
    let versioner = get_versioner(&repo, pattern);

    if let Some(version) = versioner.next_version(args.version_part.clone()) {
        if args.tag {
            let head = repo.head().unwrap();
            let head_id = head.target().unwrap();
            gitutils::tag_oid(&repo, head_id, version.tag.as_str()).expect("Failed to tag");
        }
        println!("{}", version.tag);
        Some(version.tag)
    } else {
        println!("No version found");
        None
    }
}

fn get_versioner(repo: &git2::Repository, pattern: String) -> Versioner {
    let tag_names = repo
        .tag_names(Some("*"))
        .expect("Failed to fetch tags")
        .iter()
        .map(|s| s.unwrap().to_string())
        .collect::<Vec<_>>();
    let versioner = Versioner::new(tag_names, pattern);
    versioner
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::LastVersionArgs;
    use crate::versioning::VersionPart;
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
        gitutils::checkout_tag(&repo, "flopha@2.10.11").unwrap();
        gitutils::commit(&repo, "New commit").unwrap();

        // When
        let args = NextVersionArgs {
            pattern: Some("flopha@{major}.{minor}.{patch}".to_string()),
            version_part: VersionPart::Patch,
            tag: false,
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
        gitutils::checkout_tag(&repo, "flopha@1.1.2").unwrap();
        gitutils::commit(&repo, "New commit").unwrap();

        // When
        let args = NextVersionArgs {
            pattern: Some("flopha@{major}.{minor}.{patch}".to_string()),
            version_part: VersionPart::Patch,
            tag: true,
        };
        next_version(td.path(), &args);

        // Then
        let tag_id = repo.revparse_single("refs/tags/flopha@1.1.3").unwrap().id();
        let head_id = repo.head().unwrap().peel_to_commit().unwrap().id();
        assert_eq!(tag_id, head_id);
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
}

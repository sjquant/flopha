use std::path::Path;

use crate::cli::LastVersionArgs;
use crate::gitutils;
use crate::versioning::Versioner;

pub fn last_version(path: &Path, args: &LastVersionArgs, mut writer: impl std::io::Write) {
    let repo = gitutils::get_repo(path);
    let mut remote = gitutils::get_remote(&repo, "origin");
    gitutils::fetch_all(&mut remote).expect("Failed to fetch from remote");
    let tag_names = repo
        .tag_names(Some("*"))
        .expect("Failed to fetch tags")
        .iter()
        .map(|s| s.unwrap().to_string())
        .collect::<Vec<_>>();
    let pattern = args
        .pattern
        .clone()
        .unwrap_or("v{major}.{minor}.{patch}".to_string());
    let versioner = Versioner::new(tag_names, pattern);
    let last_version = versioner.last_version();
    _ = writeln!(
        writer,
        "{}",
        last_version.unwrap_or("No version found".to_string())
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::LastVersionArgs;
    use crate::{gitutils, testutils};

    #[test]
    fn test_last_version_print_last_version_with_given_pattern() {
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
            create_new_remote_tag(&repo, &mut remote, tag);
        }

        // When
        let mut result = Vec::new();
        let args = LastVersionArgs {
            pattern: Some("flopha@{major}.{minor}.{patch}".to_string()),
            checkout: false,
        };

        last_version(td.path(), &args, &mut result);

        // Then
        assert_eq!(result, b"flopha@2.10.11\n");
    }

    #[test]
    fn test_last_version_without_matching_version_print_no_version_found() {
        // Given
        let (td, repo) = testutils::init_repo();
        let (_remote_td, mut remote) = testutils::init_remote(&repo);

        let tags = vec!["v0.1.0", "v1.0.0", "v1.0.1"];
        for tag in tags {
            create_new_remote_tag(&repo, &mut remote, tag);
        }

        // When
        let mut result = Vec::new();
        let args = LastVersionArgs {
            pattern: Some("flopha@{major}.{minor}.{patch}".to_string()),
            checkout: false,
        };
        last_version(td.path(), &args, &mut result);

        // Then
        assert_eq!(result, b"No version found\n");
    }

    fn create_new_remote_tag(repo: &git2::Repository, remote: &mut git2::Remote, tag: &str) {
        let commit_id = gitutils::commit(&repo, "New commit").unwrap();
        gitutils::tag_oid(&repo, commit_id, tag, false).unwrap();
        remote.push(&[format!("refs/tags/{}", tag)], None).unwrap();
        repo.tag_delete(tag).unwrap(); // delete local tag
    }
}

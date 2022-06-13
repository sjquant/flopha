use std::path::Path;

use git2::{Repository, FetchOptions, AutotagOption};

use crate::gitutils::{checkout_branch, checkout_tag};
use crate::cli::{StartCommand};


pub fn start_hotfix(path: &Path, _command: &StartCommand) {
    let repo = Repository::open(path).expect("Repository not found");
    let mut remote = repo.find_remote("origin").expect("origin not found");
    let mut fo = FetchOptions::new();
    fo.download_tags(AutotagOption::All);
    let _ = remote.fetch(&["refs/heads/*:refs/heads/*"],  Some(&mut fo), None);

    let tag_names = repo.tag_names(Some("*")).unwrap();
    let max_tag = tag_names.iter().map(|x| x.unwrap()).max().unwrap();
    checkout_tag(&repo, max_tag).unwrap();

}

pub fn start_feature(path: &Path, command: &StartCommand) {
    let repo = Repository::open(path).expect("Repository not found");
    let branch_name = command.branch.as_ref().expect("Branch not found").as_str();
    let branch = repo.find_branch(branch_name, git2::BranchType::Local);
    if let Err(_) = branch {
        let branch_name = command.branch.as_ref().unwrap().as_str();
        let commit = repo.head().unwrap().peel_to_commit().unwrap();
        repo.branch(branch_name, &commit, true).unwrap();
    }
    checkout_branch(&repo, branch_name, true).unwrap();
}


#[cfg(test)]
mod tests {
    use crate::{testutils, gitutils::{commit, tag_oid}};
    use super::*;
    use git2::{BranchType};

    #[test]
    fn feature_start_creates_new_branch_if_not_exists() {
        // Given
        let (td, repo) = testutils::init_repo();
        
        // When
        let command = StartCommand {
            name: "feature".to_string(),
            branch: Some("new-feature".to_string()),
        };
        start_feature(td.path(), &command);

        // Then
        let head = repo.head().unwrap();
        let current_branch_name = head.name().unwrap();
        assert_eq!(current_branch_name, "refs/heads/new-feature")
    }

    #[test]
    fn feature_start_moves_to_the_branch_if_exists() {
        // Given
        let (td, repo) = testutils::init_repo();
        checkout_branch(&repo, "existing-feature", true).unwrap();
        commit(&repo, "commit on existing feature branch").unwrap();
        checkout_branch(&repo, "main", false).unwrap();

        // When
        let command = StartCommand {
            name: "feature".to_string(),
            branch: Some("existing-feature".to_string()),
        };
        start_feature(td.path(), &command);

        // Then
        let branch = repo.find_branch("existing-feature", BranchType::Local).unwrap();
        let commit = branch.into_reference().peel_to_commit().unwrap();
        let head = repo.head().unwrap();
        let current_branch_name = head.name().unwrap();
        assert_eq!(current_branch_name, "refs/heads/existing-feature");
        assert_eq!(commit.message().unwrap(), "commit on existing feature branch");
    }

     #[test]
    fn hotfix_start_checkout_to_remote_latest_tag() {
        // Given
        let (td, repo) = testutils::init_repo();
        let (_remote_td, mut remote)= testutils::init_remote(&repo);
        // 1. Tag the commit v0.1.0, and push to remote
        let id = repo.head().unwrap().target().unwrap();
        tag_oid(&repo, id, "v0.1.0").unwrap();
        remote.push(&["refs/tags/v0.1.0"], None).unwrap();
        // 2. Add a commit to tag v0.1.0, tag the commit v0.1.1, and push to remote
        checkout_tag(&repo, "v0.1.0").unwrap();
        let commit_id = commit(&repo, "commit v0.1.1").unwrap();
        tag_oid(&repo, commit_id, "v0.1.1").unwrap();
        remote.push(&["refs/tags/v0.1.1"], None).unwrap();
        // 3. Add new commit to main, and push to remote
        checkout_branch(&repo, "main", false).unwrap();
        commit(&repo, "new commit").unwrap();
        remote.push(&["refs/heads/main"], None).unwrap();
        // 4. Remove all tags from local
        repo.tag_delete("v0.1.0").unwrap();
        repo.tag_delete("v0.1.1").unwrap();

        // When
        let command = StartCommand {
            name: "hotfix".to_string(),
            branch: None,
        };
        start_hotfix(td.path(), &command);

        // Then
        let tag_id = repo.revparse_single("refs/tags/v0.1.1").unwrap().id();
        let head_id = repo.head().unwrap().peel_to_commit().unwrap().id();
        assert_eq!(tag_id, head_id);
    }
}

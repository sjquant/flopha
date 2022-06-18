use std::path::Path;

use git2::Repository;

use crate::gitutils::{checkout_branch, checkout_tag, get_head_branch, fetch_all};
use crate::cli::{StartCommand, FinishCommand};


fn get_repo(path: &Path) -> Repository {
    let repo = Repository::open(path).expect("Repository not found");
    repo
}


fn get_remote(repo: &Repository) -> git2::Remote {
    let remote = repo.find_remote("origin").expect("Remote 'origin' not found");
    remote
}

pub fn start_feature(path: &Path, command: &StartCommand) {
    let repo = get_repo(path);
    let branch_name = command.branch.as_ref().expect("'branch' is required to start feature").as_str();
    let branch = repo.find_branch(branch_name, git2::BranchType::Local);
    if let Err(_) = branch {
        let branch_name = command.branch.as_ref().unwrap().as_str();
        let commit = repo.head().unwrap().peel_to_commit().unwrap();
        repo.branch(branch_name, &commit, true).unwrap();
    }
    checkout_branch(&repo, branch_name, true).unwrap();
}

pub fn finish_feature(path: &Path, command: &FinishCommand) {
    let repo = get_repo(path);
    let mut remote = get_remote(&repo);
    let branch = get_head_branch(&repo).expect("Branch not found");
    let branch_name = branch.name().unwrap().expect("Failed to get branch name");
    let refspec = format!("refs/heads/{}:refs/heads/{}", branch_name, branch_name);
    remote.push(&[&refspec], None).expect("Failed to push branch");
}

pub fn start_hotfix(path: &Path, _command: &StartCommand) {
    let repo = get_repo(path);
    let mut remote = get_remote(&repo);
    fetch_all(&mut remote).expect("Failed to fetch from remote");
    let tag_names = repo.tag_names(Some("*")).expect("Failed to get tags");
    let max_tag = tag_names.iter().map(|x| x.unwrap()).max().unwrap();
    checkout_tag(&repo, max_tag).expect("Failed to checkout");
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
    fn feature_fisish_should_push_commits_to_remote() {
        // Given
        let (td, repo) = testutils::init_repo();
        let (_remote_td, mut remote) = testutils::init_remote(&repo);
        
        checkout_branch(&repo, "a-feature", true).unwrap();
        commit(&repo, "first commit on feature branch").unwrap();
        commit(&repo, "second commit on feature branch").unwrap();

        // When
        let command = FinishCommand {
            name: "feature".to_string(),
        };
        finish_feature(td.path(), &command);

        // Then
        let conn = remote.connect_auth(git2::Direction::Fetch, None, None).unwrap();
        let remote_branch_head = conn.list().unwrap().iter().find(|x| x.name() == "refs/heads/a-feature");
        assert!(remote_branch_head.is_some());
        
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

use std::path::Path;

use git2::Repository;
use regex::Regex;

use crate::cli::{FinishFeatureArgs, FinishHotfixArgs, StartFeatureArgs, StartHotfixArgs};
use crate::gitutils::{
    checkout_branch, checkout_tag, fetch_all, get_head_branch, get_last_tag_name, push_branch,
    push_tag, tag_oid,
};

fn get_repo(path: &Path) -> Repository {
    let repo = Repository::open(path).expect("Repository not found");
    repo
}

fn get_remote(repo: &Repository) -> git2::Remote {
    let remote = repo
        .find_remote("origin")
        .expect("Remote 'origin' not found");
    remote
}

pub fn start_feature(path: &Path, args: &StartFeatureArgs) {
    let repo = get_repo(path);
    let branch_name = args.branch.as_str();
    let branch = repo.find_branch(branch_name, git2::BranchType::Local);
    if let Err(_) = branch {
        let branch_name = args.branch.as_str();
        let commit = repo.head().unwrap().peel_to_commit().unwrap();
        repo.branch(branch_name, &commit, true).unwrap();
    }
    checkout_branch(&repo, branch_name, true).unwrap();
}

pub fn finish_feature(path: &Path, _args: &FinishFeatureArgs) {
    let repo = get_repo(path);
    let mut remote = get_remote(&repo);
    let branch = get_head_branch(&repo).expect("Branch not found");
    let branch_name = branch.name().unwrap().expect("Failed to get branch name");
    push_branch(&mut remote, branch_name).expect("Failed to push feature");
}

pub fn start_hotfix(path: &Path, _args: &StartHotfixArgs) {
    let repo = get_repo(path);
    let mut remote = get_remote(&repo);
    fetch_all(&mut remote).expect("Failed to fetch from remote");
    let tag_names = repo.tag_names(Some("*")).expect("Failed to fetch tags");
    if tag_names.len() == 0 {
        panic!("No tags found");
    }
    let max_version = get_max_version(tag_names);
    checkout_tag(&repo, max_version.as_str()).expect("Failed to checkout");
}

fn get_max_version(tag_names: git2::string_array::StringArray) -> String {
    let mut max_version = "";
    for tag_name in tag_names.iter() {
        let tag_name = tag_name.unwrap();
        let re = Regex::new(r"^v?(\d+)\.(\d+)\.(\d+)").unwrap();
        let max_captures = re.captures(max_version);

        if max_captures.is_none() {
            max_version = tag_name;
            continue;
        }

        if let Some(captures) = re.captures(tag_name) {
            let major = captures.get(1).unwrap().as_str().parse::<i32>().unwrap();
            let minor = captures.get(2).unwrap().as_str().parse::<i32>().unwrap();
            let patch = captures.get(3).unwrap().as_str().parse::<i32>().unwrap();
            
            let max_captures = max_captures.unwrap();
            let max_major = max_captures.get(1).unwrap().as_str().parse::<i32>().unwrap();
            let max_minor = max_captures.get(2).unwrap().as_str().parse::<i32>().unwrap();
            let max_patch = max_captures.get(3).unwrap().as_str().parse::<i32>().unwrap();

            if major > max_major {
                max_version = tag_name;
            } else if major == max_major && minor > max_minor {
                max_version = tag_name;
            } else if major == max_major && minor == max_minor && patch > max_patch {
                max_version = tag_name;
            }
        }
    }
    if max_version == "" {
        panic!("No version tags found");
    }
    max_version.to_string()
}

pub fn finish_hotfix(path: &Path, args: &FinishHotfixArgs) {
    let repo = get_repo(path);
    let last_tag = get_last_tag_name(&repo).expect("Failed to get last tag");
    let tag_parts = last_tag.rsplit_once(".").expect("Failed to parse tag");
    let next_tag = format!(
        "{}.{}",
        tag_parts.0,
        tag_parts.1.parse::<u32>().expect("Failed to parse tag") + 1
    );

    if args.force {
        push_hotfix(&repo, next_tag.as_str());
        return;
    }

    println!("Do you want to release hotfix as '{}'?", next_tag);
    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .expect("Failed to read input");
    if input.to_lowercase().trim() == "y" {
        push_hotfix(&repo, next_tag.as_str());
    }
}

fn push_hotfix(repo: &Repository, tag_name: &str) {
    let mut remote = get_remote(&repo);
    let head = repo.head().unwrap();
    let head_id = head.target().unwrap();
    let is_on_branch = head.is_branch();

    // TODO: later on, we should allow it on prod branch used for release)
    if is_on_branch {
        panic!("Cannot push hotfix on branch. Please checkout to tag");
    }

    tag_oid(&repo, head_id, tag_name, false).expect("Failed to create tag");
    push_tag(&mut remote, tag_name).expect("Failed to push tag");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        gitutils::{commit, tag_oid},
        testutils,
    };
    use git2::BranchType;

    #[test]
    fn feature_start_creates_new_branch_if_not_exists() {
        // Given
        let (td, repo) = testutils::init_repo();

        // When
        let command = StartFeatureArgs {
            branch: "new-feature".to_string(),
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
        let args = StartFeatureArgs {
            branch: "existing-feature".to_string(),
        };
        start_feature(td.path(), &args);

        // Then
        let branch = repo
            .find_branch("existing-feature", BranchType::Local)
            .unwrap();
        let commit = branch.into_reference().peel_to_commit().unwrap();
        let head = repo.head().unwrap();
        let current_branch_name = head.name().unwrap();
        assert_eq!(current_branch_name, "refs/heads/existing-feature");
        assert_eq!(
            commit.message().unwrap(),
            "commit on existing feature branch"
        );
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
        let args = FinishFeatureArgs {};
        finish_feature(td.path(), &args);

        // Then
        let conn = remote
            .connect_auth(git2::Direction::Fetch, None, None)
            .unwrap();
        let remote_branch_head = conn
            .list()
            .unwrap()
            .iter()
            .find(|x| x.name() == "refs/heads/a-feature");
        assert!(remote_branch_head.is_some());
    }

    #[test]
    fn hotfix_start_checkout_to_remote_latest_tag() {
        // Given
        let (td, repo) = testutils::init_repo();
        let (_remote_td, mut remote) = testutils::init_remote(&repo);
        // 1. Tag the commit v0.1.9, and push to remote
        let id = repo.head().unwrap().target().unwrap();
        tag_oid(&repo, id, "v0.1.9", false).unwrap();
        remote.push(&["refs/tags/v0.1.9"], None).unwrap();
        // 2. Add a commit to tag v0.1.9, tag the commit v0.1.10, and push to remote
        checkout_tag(&repo, "v0.1.9").unwrap();
        let commit_id = commit(&repo, "commit v0.1.10").unwrap();
        tag_oid(&repo, commit_id, "v0.1.10", false).unwrap();
        remote.push(&["refs/tags/v0.1.10"], None).unwrap();
        // 3. Add a commit to tag v0.1.10, tag the commit zzzzz, and push to remote
        let commit_id = commit(&repo, "commit zzzzz").unwrap();
        tag_oid(&repo, commit_id, "zzzzz", false).unwrap();
        remote.push(&["refs/tags/zzzzz"], None).unwrap();
        // 4. Add new commit to main, and push to remote
        checkout_branch(&repo, "main", false).unwrap();
        commit(&repo, "new commit").unwrap();
        remote.push(&["refs/heads/main"], None).unwrap();
        // 5. Remove all tags from local
        repo.tag_delete("v0.1.9").unwrap();
        repo.tag_delete("v0.1.10").unwrap();

        // When
        let args = StartHotfixArgs {};
        start_hotfix(td.path(), &args);

        // Then
        let tag_id = repo.revparse_single("refs/tags/v0.1.10").unwrap().id();
        let head_id = repo.head().unwrap().peel_to_commit().unwrap().id();
        assert_eq!(tag_id, head_id);
    }

    #[test]
    fn hotfix_finish_should_push_new_tag_to_remote() {
        // Given
        let (td, repo) = testutils::init_repo();
        let (_remote_td, mut remote) = testutils::init_remote(&repo);
        // 1. Tag the commit v0.1.0, and push to remote
        let id = repo.head().unwrap().target().unwrap();
        tag_oid(&repo, id, "v0.1.0", false).unwrap();
        remote.push(&["refs/tags/v0.1.0"], None).unwrap();
        // 2. Checkout to tag v0.1.0
        checkout_tag(&repo, "v0.1.0").unwrap();
        // 3. Add commits to tag v0.1.0
        commit(&repo, "First fix").unwrap();
        commit(&repo, "Second fix").unwrap();

        // When
        let args = FinishHotfixArgs { force: true };
        finish_hotfix(td.path(), &args);

        // Then
        let conn = remote
            .connect_auth(git2::Direction::Fetch, None, None)
            .unwrap();
        let remote_tag_head = conn
            .list()
            .unwrap()
            .iter()
            .find(|x| x.name() == "refs/tags/v0.1.1");
        assert!(remote_tag_head.is_some());
    }

    #[test]
    fn cannot_finish_hotfix_on_branch() {
        // Given
        let (td, repo) = testutils::init_repo();
        let (_remote_td, mut remote) = testutils::init_remote(&repo);
        // 1. Tag the commit v0.1.0, and push to remote
        let id = repo.head().unwrap().target().unwrap();
        tag_oid(&repo, id, "v0.1.0", false).unwrap();
        remote.push(&["refs/tags/v0.1.0"], None).unwrap();
        // 2. Checkout to a branch
        checkout_branch(&repo, "a-branch", true).unwrap();
        // 3. Add commits to the branch
        commit(&repo, "First fix").unwrap();
        commit(&repo, "Second fix").unwrap();

        // When
        let args = FinishHotfixArgs { force: true };
        let result = std::panic::catch_unwind(|| finish_hotfix(td.path(), &args));

        // Then
        assert!(result.is_err());
    }
}

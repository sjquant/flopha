use std::path::Path;

use git2::{PushOptions, Remote, Repository, RepositoryInitOptions};
use tempfile::TempDir;
use url::Url;

use crate::gitutils::{self, commit};

pub fn init_repo() -> (TempDir, Repository) {
    let td = TempDir::new().unwrap();
    let mut opts = RepositoryInitOptions::new();
    opts.initial_head("main");
    let repo = Repository::init_opts(td.path(), &opts).unwrap();
    {
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "name").unwrap();
        config.set_str("user.email", "email").unwrap();
        commit(&repo, "Initial commit").unwrap();
    }
    (td, repo)
}

pub fn init_remote(repo: &Repository) -> (TempDir, Remote<'_>) {
    let td = TempDir::new().unwrap();
    let url = path2url(td.path());
    let mut opts = RepositoryInitOptions::new();
    opts.bare(true);
    opts.initial_head("main");
    Repository::init_opts(td.path(), &opts).unwrap();
    let mut remote = repo.remote("origin", &url).unwrap();
    let mut push_options = PushOptions::new();
    remote
        .push(&["refs/heads/main"], Some(&mut push_options))
        .unwrap();
    (td, remote)
}

fn path2url(path: &Path) -> String {
    Url::from_file_path(path).unwrap().to_string()
}

pub fn create_new_remote_tag(
    repo: &git2::Repository,
    remote: &mut git2::Remote,
    tag: &str,
    should_delete: bool,
) {
    let commit_id = gitutils::commit(repo, "New commit").unwrap();
    gitutils::tag_oid(repo, commit_id, tag).unwrap();
    remote.push(&[format!("refs/tags/{}", tag)], None).unwrap();

    if should_delete {
        repo.tag_delete(tag).unwrap();
    }
}

pub fn create_new_remote_branch(repo: &git2::Repository, remote: &mut git2::Remote, branch: &str) {
    gitutils::checkout_branch(repo, branch, true).unwrap();
    gitutils::commit(repo, "New commit").unwrap();
    let mut branch = repo.find_branch(branch, git2::BranchType::Local).unwrap();
    gitutils::push_branch(remote, &mut branch).unwrap();
}

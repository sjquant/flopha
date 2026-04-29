use crate::gitutils;
use git2::Repository;

pub trait VersionSource {
    fn fetch_all(&self, repo: &Repository) -> Vec<String>;
    fn checkout(&self, repo: &Repository, version: &str) -> Result<(), git2::Error>;
    fn create(&self, repo: &Repository, version: &str) -> Result<(), git2::Error>;
}

pub struct TagVersionSource;
pub struct BranchVersionSource;

impl VersionSource for TagVersionSource {
    fn fetch_all(&self, repo: &Repository) -> Vec<String> {
        match repo.tag_names(Some("*")) {
            Ok(names) => names.iter().filter_map(|s| s.map(|s| s.to_string())).collect(),
            Err(e) => {
                log::warn!("Failed to fetch tags: {}", e);
                Vec::new()
            }
        }
    }

    fn checkout(&self, repo: &Repository, version: &str) -> Result<(), git2::Error> {
        gitutils::checkout_tag(repo, version)
    }

    fn create(&self, repo: &Repository, version: &str) -> Result<(), git2::Error> {
        let commit = repo.head()?.peel_to_commit()?;
        gitutils::tag_oid(repo, commit.id(), version)?;
        Ok(())
    }
}

impl VersionSource for BranchVersionSource {
    fn fetch_all(&self, repo: &Repository) -> Vec<String> {
        match repo.branches(Some(git2::BranchType::Local)) {
            Ok(branches) => branches
                .filter_map(|b| b.ok())
                .filter_map(|(branch, _)| branch.name().ok().flatten().map(|s| s.to_string()))
                .collect(),
            Err(e) => {
                log::warn!("Failed to fetch branches: {}", e);
                Vec::new()
            }
        }
    }

    fn checkout(&self, repo: &Repository, version: &str) -> Result<(), git2::Error> {
        gitutils::checkout_branch(repo, version, false)
    }

    fn create(&self, repo: &Repository, version: &str) -> Result<(), git2::Error> {
        gitutils::checkout_branch(repo, version, true)
    }
}

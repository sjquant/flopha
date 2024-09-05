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
        repo.tag_names(Some("*"))
            .expect("Failed to fetch tags")
            .iter()
            .filter_map(|s| s.map(|s| s.to_string()))
            .collect()
    }

    fn checkout(&self, repo: &Repository, version: &str) -> Result<(), git2::Error> {
        gitutils::checkout_tag(repo, version, None)
    }

    fn create(&self, repo: &Repository, version: &str) -> Result<(), git2::Error> {
        let head = repo.head()?;
        let head_id = head.target().unwrap();
        gitutils::tag_oid(repo, head_id, version)?;
        Ok(())
    }
}

impl VersionSource for BranchVersionSource {
    fn fetch_all(&self, repo: &Repository) -> Vec<String> {
        repo.branches(Some(git2::BranchType::Local))
            .expect("Failed to fetch branches")
            .filter_map(|b| b.ok())
            .filter_map(|(branch, _)| branch.name().ok().flatten().map(|s| s.to_string()))
            .collect()
    }

    fn checkout(&self, repo: &Repository, version: &str) -> Result<(), git2::Error> {
        gitutils::checkout_branch(repo, version, false, None)
    }

    fn create(&self, repo: &Repository, version: &str) -> Result<(), git2::Error> {
        gitutils::checkout_branch(repo, version, true, None)
    }
}

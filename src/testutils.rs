use std::path::Path;

use git2::{Repository, RepositoryInitOptions, PushOptions, Remote};
use tempfile::TempDir;
use url::Url;

use crate::gitutils::commit;


pub fn init_repo() -> (TempDir, Repository) {
        let td = TempDir::new().unwrap();
        let mut opts = RepositoryInitOptions::new();
        opts.initial_head("main");
        let repo = Repository::init_opts(td.path(), &opts).unwrap();
        {
            let mut config = repo.config().unwrap();
            config.set_str("user.name", "name").unwrap();
            config.set_str("user.email", "email").unwrap();
            commit(&repo,  "Initial commit").unwrap();
        }
        (td, repo)
    }

    
pub fn init_remote(repo: &Repository) -> (TempDir, Remote) {
    let td = TempDir::new().unwrap();
    let url = path2url(td.path());
    let mut opts = RepositoryInitOptions::new();
    opts.bare(true);
    opts.initial_head("main");
    Repository::init_opts(td.path(), &opts).unwrap();
    let mut remote = repo.remote("origin", &url).unwrap();
    let mut push_options = PushOptions::new();
    remote.push(&["refs/heads/main"], Some(&mut push_options)).unwrap();
    (td, remote)
}


fn path2url(path: &Path) -> String {
    Url::from_file_path(path).unwrap().to_string()
}
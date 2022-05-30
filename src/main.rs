use std::path::Path;

use clap::{Args, Parser, Subcommand};
use git2::{Repository, FetchOptions, AutotagOption};

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
#[clap(propagate_version = true)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Start(StartCommand),
    Finish(FinishCommand),
}

#[derive(Args)]
struct StartCommand {
    name: String,

    /// Feature branch name to move to or create if not exists
    #[clap(short, long)]
    branch: Option<String>,
}

#[derive(Args)]
struct FinishCommand {
    name: String,
}

fn main() {
    let cli = Cli::parse();
    match &cli.command {
        Commands::Start(command) => {
            on_start(command, Path::new("."));
        }
        Commands::Finish(command) => {
            on_finish(command);
        }
    }
}

fn on_start(command: &StartCommand, path: &Path) {
    match command.name.to_lowercase().as_str() {
        "feature" => {
            start_feature(path, command);
        }
        "hotfix" => {
            start_hotfix(path, command);
        }
        _ => {
            println!("feature and hotfix are only valid names");
            std::process::exit(1);
        }
    }
}

fn start_hotfix(path: &Path, command: &StartCommand) {
    let repo = Repository::open(path).expect("Repository not found");
    let mut remote = repo.find_remote("origin").expect("origin not found");
    let mut fo = FetchOptions::new();
    fo.download_tags(AutotagOption::All);
    let _ = remote.fetch(&["refs/heads/*:refs/heads/*"],  Some(&mut fo), None);

    let tag_names = repo.tag_names(Some("*")).unwrap();
    let max_tag = tag_names.iter().map(|x| x.unwrap()).max().unwrap();
    let (object, reference) = repo.revparse_ext(max_tag).unwrap();
    repo.checkout_tree(&object, None).expect("Failed to checkout");
    let _ = repo.set_head(reference.unwrap().name().unwrap());

}

fn start_feature(path: &Path, command: &StartCommand) {
    let repo = Repository::open(path).expect("Repository not found");
    let branch_name = command.branch.as_ref().expect("Branch not found").as_str();
    let branch = repo.find_branch(branch_name, git2::BranchType::Local);
    if let Err(_) = branch {
        let branch_name = command.branch.as_ref().unwrap().as_str();
        let commit = repo.head().unwrap().peel_to_commit().unwrap();
        repo.branch(branch_name, &commit, true).unwrap();
    }
    let (object, reference) = repo.revparse_ext(branch_name).expect("Object not found");
    repo.checkout_tree(&object, None).expect("Failed to checkout");
    let _ = repo.set_head(reference.unwrap().name().unwrap());
}

fn on_finish(command: &FinishCommand) {
    match command.name.to_lowercase().as_str() {
        "feature" => {
            println!("Push branch to remote");
            println!("Create a pull request to main branch if not already done");
        }
        "hotfix" => {
            println!("Validate whether it's from tag, not feature branch");
            println!("Tag the commit");
            println!("Push the tag to the remote");
        }
        _ => {
            println!("feature and hotfix are only valid names");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::{BranchType, RepositoryInitOptions, PushOptions, ResetType};
    use tempfile::TempDir;
    use url::Url;

    fn repo_init() -> (TempDir, Repository) {
        let td = TempDir::new().unwrap();
        let mut opts = RepositoryInitOptions::new();
        opts.initial_head("main");
        let repo = Repository::init_opts(td.path(), &opts).unwrap();
        {
            let mut config = repo.config().unwrap();
            config.set_str("user.name", "name").unwrap();
            config.set_str("user.email", "email").unwrap();
            let mut index = repo.index().unwrap();
            let id = index.write_tree().unwrap();
            let tree = repo.find_tree(id).unwrap();
            let sig = repo.signature().unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "initial\n\nbody", &tree, &[])
                .unwrap();
        }
        (td, repo)
    }

    fn path2url(path: &Path) -> String {
        Url::from_file_path(path).unwrap().to_string()
    }
    

    #[test]
    fn feature_start_creates_new_branch_if_not_exists() {
        // Given
        let (td, repo) = repo_init();
        
        // When
        let command = StartCommand {
            name: "feature".to_string(),
            branch: Some("new-feature".to_string()),
        };
        on_start(&command, td.path());

        // Then
        let head = repo.head().unwrap();
        let current_branch_name = head.name().unwrap();
        assert_eq!(current_branch_name, "refs/heads/new-feature")
    }

    #[test]
    fn feature_start_moves_to_the_branch_if_exists() {
        // Given
        let (td, repo) = repo_init();
        let head_commit = repo.head().unwrap().peel_to_commit().unwrap();
        let branch = repo.branch("existing-feature", &head_commit, true).unwrap();
        let mut index = repo.index().unwrap();
        let id = index.write_tree().unwrap();
        let tree = repo.find_tree(id).unwrap();
        let sig = repo.signature().unwrap();
        let branch_ref = branch.into_reference();
        let parent = branch_ref.peel_to_commit().unwrap();
        repo.commit(Some(branch_ref.name().unwrap()), &sig, &sig, "commit to existing branch", &tree, &[&parent])
            .unwrap();

        // When
        let command = StartCommand {
            name: "feature".to_string(),
            branch: Some("existing-feature".to_string()),
        };
        on_start(&command, td.path());

        // Then
        let branch = repo.find_branch("existing-feature", BranchType::Local).unwrap();
        let commit = branch.into_reference().peel_to_commit().unwrap();
        let head = repo.head().unwrap();
        let current_branch_name = head.name().unwrap();
        assert_eq!(current_branch_name, "refs/heads/existing-feature");
        assert_eq!(commit.message().unwrap(), "commit to existing branch");
    }

     #[test]
    fn hotfix_start_cherrypick_remote_base_commits_on_latest_tag() {
        // Given
        let (td, repo) = repo_init();
        
        // Create remote
        let remote_td = TempDir::new().unwrap();
        let url = path2url(remote_td.path());
        let mut opts = RepositoryInitOptions::new();
        opts.bare(true);
        opts.initial_head("main");
        Repository::init_opts(remote_td.path(), &opts).unwrap();
        let mut remote = repo.remote("origin", &url).unwrap();
        let mut push_options = PushOptions::new();
        remote.push(&["refs/heads/main"], Some(&mut push_options)).unwrap();
        
        // Tag the commit v0.1.0, and push to remote
        let id = repo.head().unwrap().target().unwrap();
        let obj = repo.find_object(id, None).unwrap();
        let tag_id = repo.tag_lightweight("v0.1.0", &obj, false).unwrap();

        remote.push(&["refs/tags/v0.1.0"], Some(&mut push_options)).unwrap();

        // Add a commit to tag v0.1.0, tag the commit v0.1.1, and push to remote
        let mut index = repo.index().unwrap();
        let id = index.write_tree().unwrap();
        let tree = repo.find_tree(id).unwrap();
        let obj = repo.find_object(tag_id, None).unwrap();
        let sig = repo.signature().unwrap();
        let oid = repo.commit(Some("refs/tags/v0.1.0"), &sig, &sig, "commit v0.1.1", &tree, &[obj.as_commit().unwrap()]).unwrap();

        let obj = repo.find_object(oid, None).unwrap();
        repo.tag_lightweight("v0.1.1", &obj, false).unwrap();
        remote.push(&["refs/tags/v0.1.1"], Some(&mut push_options)).unwrap();

        // Add new commit, and push to remote
        let mut index = repo.index().unwrap();
        let id = index.write_tree().unwrap();
        let tree = repo.find_tree(id).unwrap();
        let parent = repo.head().unwrap().peel_to_commit().unwrap();
        let sig = repo.signature().unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "new commit", &tree, &[&parent])
            .unwrap();
        remote.push(&["refs/heads/main"], Some(&mut push_options)).unwrap();
        
        // Remove tag v0.1.0 and v0.1.1
        repo.tag_delete("v0.1.0").unwrap();
        repo.tag_delete("v0.1.1").unwrap();

        // Reset --hard to HEAD~1
        let head = repo.head().unwrap();
        let head_commit = head.peel_to_commit().unwrap();
        let head_parent = head_commit.parent(0).unwrap();
        repo.reset(head_parent.as_object(), ResetType::Hard, None).unwrap();

        // When
        let command = StartCommand {
            name: "hotfix".to_string(),
            branch: None,
        };
        on_start(&command, td.path());

        // Then

        // Move to latest origin tag
        let tag_id = repo.revparse_single("refs/tags/v0.1.1").unwrap().id();
        let head_id = repo.head().unwrap().peel_to_commit().unwrap().id();
        assert_eq!(tag_id, head_id);

    }
}

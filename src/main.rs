use std::path::Path;

use clap::{Args, Parser, Subcommand};
use git2::Repository;

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
            let repo = Repository::open(path).unwrap();
            let branch_name = command.branch.as_ref().unwrap().as_str();
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
        "hotfix" => {
            println!("Move to the latest tag");
            println!("Cherry-pick changes from the feature branch (Suggest commits)");
        }
        _ => {
            println!("feature and hotfix are only valid names");
            std::process::exit(1);
        }
    }
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
    use git2::{BranchType, RepositoryInitOptions};
    use tempfile::TempDir;

    pub fn repo_init() -> (TempDir, Repository) {
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
        let target = repo.head().unwrap().peel_to_commit().unwrap();
        let branch = repo.branch("existing-feature", &target, true).unwrap();
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
}

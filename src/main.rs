use clap::{Args, Parser, Subcommand};
use git2::{Branch, Repository};

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
#[clap(propagate_version = true)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Start(Start),
    Finish(Finish),
}

#[derive(Args)]
struct Start {
    name: String,

    /// Feature branch name to move to or create if not exists
    #[clap(short, long)]
    branch: Option<String>,
}

#[derive(Args)]
struct Finish {
    name: String,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Start(command) => {
            on_start(command);
        }
        Commands::Finish(command) => {
            on_finish(command);
        }
    }
}

fn on_start(command: Start) {
    match command.name.to_lowercase().as_str() {
        "feature" => {
            handle_feature_start(command);
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

fn handle_feature_start(command: Start) {
    todo!()
}

fn on_finish(command: Finish) {
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
    use git2::BranchType;
    use tempfile::TempDir;

    #[test]
    fn feature_start_creates_new_branch_if_not_exists() {
        // Given
        let td = TempDir::new().unwrap();
        let path = td.path();
        let repo = Repository::init(path).unwrap();
        let command = Start {
            name: "feature".to_string(),
            branch: Some("new-feature".to_string()),
        };

        // When
        on_start(command);

        // Then
        assert!(repo.find_branch("new-feature", BranchType::Local).is_ok())
    }
}

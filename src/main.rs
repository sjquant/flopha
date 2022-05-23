use clap::{Args, Parser, Subcommand};

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
            println!("Create or Move to the feature branch");
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

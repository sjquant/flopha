use std::path::Path;

use clap::Parser;
use flopha::{cli::{Cli, FinishCommand, Commands, StartCommand}, service::{start_feature, start_hotfix}};



fn main() {
    let cli = Cli::parse();
    match &cli.command {
        Commands::Start(command) => {
            on_start(command);
        }
        Commands::Finish(command) => {
            on_finish(command);
        }
    }
}

fn on_start(command: &StartCommand) {
    let path = Path::new(".");
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
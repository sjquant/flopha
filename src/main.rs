use std::path::Path;

use clap::Parser;
use flopha::{
    cli::{Cli, Commands},
    service::{finish_feature, finish_hotfix, start_feature, start_hotfix},
};

fn main() {
    let cli = Cli::parse();
    let path = Path::new(".");
    match &cli.command {
        Commands::StartFeature(args) => {
            start_feature(path, args);
        }
        Commands::FinishFeature(args) => {
            finish_feature(path, args);
        }
        Commands::StartHotfix(args) => {
            start_hotfix(path, args);
        }
        Commands::FinishHotfix(args) => {
            finish_hotfix(path, args);
        }
    }
}

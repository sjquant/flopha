use std::path::Path;

use clap::Parser;
use flopha::cli::{Cli, Commands};
use flopha::service::{last_version, next_version};

fn main() {
    let cli = Cli::parse();
    let path = Path::new(".");
    match &cli.command {
        Commands::LastVersion(args) | Commands::Lv(args) => {
            last_version(path, args);
        }
        Commands::NextVersion(args) | Commands::Nv(args) => {
            next_version(path, args);
        }
    }
}

use std::path::Path;

use clap::{IntoApp, Parser};
use flopha::cli::{Cli, Commands};
use flopha::service::{last_version, next_version};

fn main() {
    let cli = Cli::parse();
    let path = Path::new(".");
    match &cli.command {
        Some(Commands::LastVersion(args)) => {
            last_version(path, args);
        }
        Some(Commands::NextVersion(args)) => {
            next_version(path, args);
        }
        None => {
            if cli.version {
                println!("{}", env!("CARGO_PKG_VERSION"));
            } else {
                Cli::command().print_help().unwrap();
            }
        }
    }
}

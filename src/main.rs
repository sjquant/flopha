use std::path::Path;

use clap::{CommandFactory, Parser};
use flopha::cli::{Cli, Commands};
use flopha::service::{last_version, next_version};

fn main() {
    let cli = Cli::parse();

    let verbose = match &cli.command {
        Some(Commands::LastVersion(a)) => a.verbose,
        Some(Commands::NextVersion(a)) => a.verbose,
        None => false,
    };
    env_logger::Builder::new()
        .filter_level(if verbose {
            log::LevelFilter::Debug
        } else {
            log::LevelFilter::Warn
        })
        .init();

    let path = Path::new(".");
    let result = match &cli.command {
        Some(Commands::LastVersion(args)) => last_version(path, args),
        Some(Commands::NextVersion(args)) => next_version(path, args),
        None => {
            if cli.version {
                println!("{}", env!("CARGO_PKG_VERSION"));
            } else {
                Cli::command().print_help().unwrap();
            }
            return;
        }
    };
    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

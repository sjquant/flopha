use std::path::Path;

use clap::Parser;
use flopha::cli::{Cli, Commands};
use flopha::service::last_version;

fn main() {
    let cli = Cli::parse();
    let path = Path::new(".");
    match &cli.command {
        Commands::LastVersion(args) => {
            last_version(path, args, std::io::stdout());
        }
        Commands::NextVersion(args) => {
            println!("{:?}", args)
        }
    }
}

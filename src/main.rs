use std::path::Path;

use clap::Parser;
use flopha::cli::{Cli, Commands};

fn main() {
    let cli = Cli::parse();
    let path = Path::new(".");
    match &cli.command {
        Commands::Versioning(args) => {
            println!("{:?}", args)
        }
        Commands::Teleport(args) => {
            println!("{:?}", args)
        }
    }
}

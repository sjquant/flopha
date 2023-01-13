use std::path::Path;

use clap::Parser;
use flopha::cli::{Cli, Commands};

fn main() {
    let cli = Cli::parse();
    let path = Path::new(".");
    match &cli.command {
        Commands::NextVersion(args) => {
            println!("{:?}", args)
        }
        Commands::LastVersion(args) => {
            println!("{:?}", args)
        }
    }
}

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
#[clap(propagate_version = true)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Start { name: String },
    Finish { name: String },
    Release,
}

fn main() {
    let cli = Cli::parse();
    match &cli.command {
        Commands::Start { name } => {
            println!("Start {}", name)
        }
        Commands::Finish { name } => {
            println!("Finish {}", name)
        }
        Commands::Release => {
            println!("Release")
        }
    }
}

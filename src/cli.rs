use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
#[clap(propagate_version = true)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    #[clap(about = "Versioning based on the current tag")]
    Versioning(VersioningArgs),
    #[clap(about = "Move to the last version")]
    Teleport(TeleportArgs),
}

#[derive(Args, Debug)]
pub struct VersioningArgs {
    #[clap(long, action)]
    pub major: bool,
    #[clap(long, action)]
    pub minor: bool,
    #[clap(long, action)]
    pub patch: bool,
    #[clap(long)]
    pub pattern: Option<String>,
}

#[derive(Args, Debug)]
pub struct TeleportArgs {
    #[clap(long, action)]
    pub major: bool,
    #[clap(long, action)]
    pub minor: bool,
    #[clap(long, action)]
    pub patch: bool,
    #[clap(long)]
    pub pattern: Option<String>,
}

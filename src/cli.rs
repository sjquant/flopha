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
    Start(StartCommand),
    Finish(FinishCommand),
}

#[derive(Args)]
pub struct StartCommand {
    pub name: String,

    /// Feature branch name to move to or create if not exists
    #[clap(short, long)]
    pub branch: Option<String>,
}

#[derive(Args)]
pub struct FinishCommand {
    pub name: String,
}
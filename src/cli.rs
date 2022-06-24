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
    StartFeature(StartFeatureArgs),
    FinishFeature(FinishFeatureArgs),
    StartHotfix(StartHotfixArgs),
    FinishHotfix(FinishHotfixArgs),
}

#[derive(Args)]
pub struct StartFeatureArgs {
    /// Feature branch name to move to or create if not exists
    #[clap(short, long)]
    pub branch: String,
}

#[derive(Args)]
pub struct FinishFeatureArgs {
}

#[derive(Args)]
pub struct StartHotfixArgs {
}


#[derive(Args)]
pub struct FinishHotfixArgs {
}

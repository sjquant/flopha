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
    #[clap(about = "Related to next version actions")]
    NextVersion(NextVersionArgs),
    #[clap(about = "Related to last version actions")]
    LastVersion(LastVersionArgs),
}

#[derive(Args, Debug)]
pub struct NextVersionArgs {
    #[clap(help = "Get next major version", long, action)]
    pub major: bool,
    #[clap(help = "Get next minor version", long, action)]
    pub minor: bool,
    #[clap(help = "Get next patch version", long, action)]
    pub patch: bool,
    #[clap(help = "Get next version based on pattern", long)]
    pub pattern: Option<String>,
    #[clap(help = "Tag current commit as next version", long, action)]
    pub tag: bool,
    #[clap(
        help = "Tag current commit as next version and push to origin",
        long,
        action
    )]
    pub publish: bool,
}

#[derive(Args, Debug)]
pub struct LastVersionArgs {
    #[clap(
        help = "Get last version in the context of major version",
        long,
        action
    )]
    pub major: bool,
    #[clap(
        help = "Get last version in the context of minor version",
        long,
        action
    )]
    pub minor: bool,
    #[clap(
        help = "Get last version in the context of patch version",
        long,
        action
    )]
    pub patch: bool,
    #[clap(help = "Get last version based on last version", long)]
    pub pattern: Option<String>,
    #[clap(help = "Check out to last version", long, action)]
    pub checkout: bool,
}

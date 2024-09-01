use clap::{Args, Parser, Subcommand};

use crate::versioning::Increment;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
#[clap(propagate_version = true)]
#[clap(version_short = 'v')]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    #[clap(
        about = "Calculates the next version number based on the latest matching tag. (alias: nv)",
        alias = "nv"
    )]
    NextVersion(NextVersionArgs),
    #[clap(
        about = "Finds the latest version tag in the repository matching a given pattern. (alias: lv)",
        alias = "lv"
    )]
    LastVersion(LastVersionArgs),
}

#[derive(Args, Debug)]
pub struct NextVersionArgs {
    #[clap(
        help = "Specify the version part to increment: major, minor, or patch",
        long,
        arg_enum,
        default_value = "patch",
        short = 'i'
    )]
    pub increment: Increment,
    #[clap(help = "Get next version based on pattern", long, short = 'p')]
    pub pattern: Option<String>,
    #[clap(help = "Tag current commit as next version", long, short = 't', action)]
    pub tag: bool,
    #[clap(help = "Verbose output", long, short = 'V', action)]
    pub verbose: bool,
}

#[derive(Args, Debug)]
pub struct LastVersionArgs {
    #[clap(help = "Get last version based on last version", long, short = 'p')]
    pub pattern: Option<String>,
    #[clap(help = "Check out to last version", long, short = 'c', action)]
    pub checkout: bool,
    #[clap(help = "Verbose output", long, short = 'V', action)]
    pub verbose: bool,
}

use clap::{Args, Parser, Subcommand, ValueEnum};

use crate::versioning::Increment;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
pub struct Cli {
    #[clap(short = 'v', long = "version", action)]
    pub version: bool,

    #[clap(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    #[clap(
        about = "Calculates the next version number based on the latest matching tag or branch. (alias: nv)",
        alias = "nv"
    )]
    NextVersion(NextVersionArgs),
    #[clap(
        about = "Finds the latest version tag or branch in the repository matching a given pattern. (alias: lv)",
        alias = "lv"
    )]
    LastVersion(LastVersionArgs),
}

#[derive(Args, Debug)]
pub struct NextVersionArgs {
    #[clap(
        help = "Specify the version part to increment: major, minor, or patch",
        long,
        value_enum,
        default_value = "patch",
        short = 'i'
    )]
    pub increment: Increment,
    #[clap(
        help = "Enable verbose output for detailed information",
        long,
        short = 'v',
        action
    )]
    pub verbose: bool,
    #[clap(
        help = "Specify a custom pattern for version matching and generation. \
                Use {major}, {minor}, and {patch} as placeholders. \
                Example: 'v{major}.{minor}.{patch}' or 'release-{major}.{minor}.{patch}'",
        long,
        short = 'p'
    )]
    pub pattern: Option<String>,
    #[clap(
        help = "Create a new tag or branch with the next version",
        long,
        action
    )]
    pub create: bool,
    #[clap(
        help = "Specify the source for versioning: tag (default) or branch",
        long,
        short = 's',
        value_enum,
        default_value = "tag"
    )]
    pub source: VersionSourceName,
}

#[derive(Args, Debug)]
pub struct LastVersionArgs {
    #[clap(
        help = "Get last version based on a given pattern (e.g., 'v{major}.{minor}.{patch}')",
        long,
        short = 'p'
    )]
    pub pattern: Option<String>,
    #[clap(
        help = "Enable verbose output for detailed information",
        long,
        short = 'v',
        action
    )]
    pub verbose: bool,
    #[clap(
        help = "Specify the source for versioning: tag (default) or branch",
        long,
        value_enum,
        default_value = "tag",
        short = 's'
    )]
    pub source: VersionSourceName,
    #[clap(help = "Checkout the last version", long, action)]
    pub checkout: bool,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum VersionSourceName {
    Tag,
    Branch,
}

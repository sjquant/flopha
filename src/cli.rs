use clap::{Args, Parser, Subcommand};

use crate::versioning::VersionPart;

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
    #[clap(
        help = "major, minor or patch",
        long,
        arg_enum,
        default_value = "patch"
    )]
    pub version_part: VersionPart,
    #[clap(help = "Get next version based on pattern", long)]
    pub pattern: Option<String>,
    #[clap(help = "Tag current commit as next version", long, action)]
    pub tag: bool,
}

#[derive(Args, Debug)]
pub struct LastVersionArgs {
    #[clap(help = "Get last version based on last version", long)]
    pub pattern: Option<String>,
    #[clap(help = "Check out to last version", long, action)]
    pub checkout: bool,
}

use clap::{Args, Parser, Subcommand, ValueEnum};

use crate::versioning::Increment;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
pub struct Cli {
    #[clap(short = 'V', long = "version", action)]
    pub version: bool,

    #[clap(
        short = 'v',
        long,
        action,
        global = true,
        help = "Enable verbose output"
    )]
    pub verbose: bool,

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
    #[clap(
        about = "Shows a timeline of all version tags matching a pattern. (alias: lg)",
        alias = "lg"
    )]
    Log(LogArgs),
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
        help = "Auto-detect bump level from conventional commit messages since last tag. \
                feat→minor, feat!/BREAKING CHANGE→major, else patch. \
                Cannot be combined with --increment.",
        long,
        action,
        conflicts_with = "increment"
    )]
    pub auto: bool,
    #[clap(
        help = "Custom bump rule as '<level>:<regex>' matched against commit messages. \
                Repeatable; when any --rule flags are provided they replace the built-in \
                conventional-commit defaults entirely. \
                Levels: major | minor | patch. \
                Example: --rule 'major:BREAKING CHANGE' --rule 'minor:^feat'",
        long,
        value_name = "LEVEL:PATTERN",
        requires = "auto"
    )]
    pub rule: Vec<String>,
    #[clap(
        help = "Create a pre-release version on the given channel (e.g. alpha, beta, rc). \
                Example: --pre alpha produces v1.2.3-alpha.1",
        long
    )]
    pub pre: Option<String>,
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

#[derive(Args, Debug)]
pub struct LogArgs {
    #[clap(
        help = "Pattern for version matching (e.g., 'v{major}.{minor}.{patch}')",
        long,
        short = 'p'
    )]
    pub pattern: Option<String>,
    #[clap(
        help = "Specify the source for versioning: tag (default) or branch",
        long,
        short = 's',
        value_enum,
        default_value = "tag"
    )]
    pub source: VersionSourceName,
    #[clap(
        help = "Maximum number of versions to show (default: all)",
        long,
        short = 'n'
    )]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum VersionSourceName {
    Tag,
    Branch,
}

use clap::{Args, Parser, Subcommand, ValueEnum};

use crate::versioning::Increment;

#[derive(Parser)]
#[clap(author, version, about, long_about = None, disable_version_flag = true)]
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
    #[clap(
        about = "Generates a changelog from commits since the last (or a given) version. (alias: cl)",
        alias = "cl"
    )]
    Changelog(ChangelogArgs),
}

#[derive(Debug, Clone, ValueEnum, PartialEq)]
pub enum OutputFormat {
    Text,
    Json,
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
        help = "Annotated tag message; when set an annotated tag is created instead of a lightweight one",
        long,
        requires = "create"
    )]
    pub tag_message: Option<String>,
    #[clap(
        help = "Push the tag or branch created by --create to 'origin'",
        long,
        action,
        requires = "create"
    )]
    pub push: bool,
    #[clap(
        help = "Specify the source for versioning: tag (default) or branch",
        long,
        short = 's',
        value_enum,
        default_value = "tag"
    )]
    pub source: VersionSourceName,
    #[clap(
        help = "Output format: text (default) or json",
        long,
        short = 'f',
        value_enum,
        default_value = "text"
    )]
    pub format: OutputFormat,
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
    #[clap(
        help = "Output format: text (default) or json",
        long,
        short = 'f',
        value_enum,
        default_value = "text"
    )]
    pub format: OutputFormat,
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
    #[clap(
        help = "Output format: text (default) or json",
        long,
        short = 'f',
        value_enum,
        default_value = "text"
    )]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct ChangelogArgs {
    #[clap(
        help = "Tag to start the changelog from (default: latest version matching pattern). \
                Defines which commits are included — commits reachable from HEAD that are \
                not ancestors of this tag.",
        long
    )]
    pub from: Option<String>,
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
        help = "Group definition as 'TITLE:REGEX'. Repeatable; groups appear in the order \
                given. When any --group flags are provided they replace the built-in defaults \
                entirely. Commits not matched by any group are collected under 'Other Changes'. \
                The title may contain spaces; the regex starts after the first colon and may \
                itself contain colons. \
                Example: --group 'New Features:^feat' --group 'Fixes:^fix'",
        long,
        value_name = "TITLE:PATTERN"
    )]
    pub group: Vec<String>,
    #[clap(
        help = "Title for commits that match no group (default: \"Other Changes\"). \
                Pass an empty string to suppress unmatched commits entirely.",
        long,
        value_name = "TITLE"
    )]
    pub other: Option<String>,
    #[clap(
        help = "Title template for the changelog heading. Use {from} for the starting version \
                and {to} for the target version. \
                Defaults to \"Changes in {to}\" when --to is given, \
                otherwise \"Changelog since {from}\".",
        long
    )]
    pub title: Option<String>,
    #[clap(
        help = "Upper bound for the changelog (e.g. v1.2.3). When the tag exists it limits \
                which commits are included; when it does not exist yet (e.g. in CI before \
                tagging) it falls back to HEAD and is used as a display label only. \
                Exposed as {to} in the --title template.",
        long
    )]
    pub to: Option<String>,
    #[clap(
        help = "Overwrite the output file instead of prepending to it",
        long,
        action
    )]
    pub overwrite: bool,
    #[clap(
        help = "Write the changelog to a file instead of stdout",
        long,
        short = 'o'
    )]
    pub output: Option<String>,
    #[clap(
        help = "Output format: text (default) or json",
        long,
        short = 'f',
        value_enum,
        default_value = "text"
    )]
    pub format: OutputFormat,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum VersionSourceName {
    Tag,
    Branch,
}

//! One module per CLI subcommand, dispatched from `main.rs`. Helpers here are
//! shared by several commands; command modules don't depend on each other.
mod changelog;
mod last_version;
mod log;
mod next_version;
mod release;

use crate::cli::{OutputFormat, VersionSourceName};
use crate::error::FlophaError;
use crate::gitutils;
use crate::version_source::{BranchVersionSource, TagVersionSource, VersionSource};
use crate::versioning::{self, Increment, Version, Versioner};
pub use changelog::changelog;
pub use last_version::last_version;
pub use log::log_versions;
pub use next_version::next_version;
pub use release::release;

fn pattern_or_default(pattern: &Option<String>) -> String {
    pattern
        .clone()
        .unwrap_or_else(|| versioning::DEFAULT_PATTERN.to_string())
}

/// Prints `null` for JSON output, or `message` for text, when a command has no result.
fn print_none(format: &OutputFormat, message: &str) {
    match format {
        OutputFormat::Json => println!("null"),
        OutputFormat::Text => println!("{}", message),
    }
}

fn version_source_factory(source: &VersionSourceName) -> Box<dyn VersionSource> {
    match source {
        VersionSourceName::Branch => Box::new(BranchVersionSource),
        VersionSourceName::Tag => Box::new(TagVersionSource),
    }
}

fn versioner_factory(
    repo: &git2::Repository,
    pattern: String,
    source: &VersionSourceName,
) -> Versioner {
    let version_source = version_source_factory(source);
    let versions = version_source.fetch_all(repo);
    Versioner::new(versions, pattern)
}

/// Determines the [`Increment`] to apply: auto-detects from commit messages
/// since `last` when `auto` is set, otherwise returns `fallback` unchanged.
/// Takes an already-resolved `last` so callers that also need it (`release`
/// uses it as the changelog start) don't compute it twice.
fn resolve_increment(
    repo: &git2::Repository,
    last: Option<&Version>,
    auto: bool,
    raw_rules: &[String],
    fallback: Increment,
) -> Result<Increment, FlophaError> {
    if !auto {
        return Ok(fallback);
    }
    let rules = versioning::build_rules(raw_rules)?;
    Ok(match last {
        Some(last) => {
            let messages = gitutils::commits_since_tag(repo, &last.tag).unwrap_or_default();
            versioning::detect_increment(&messages, &rules)
        }
        None => {
            ::log::warn!("--auto: no prior tag found, falling back to --increment");
            fallback
        }
    })
}

/// Returns the next available pre-release counter for `base_version` on `channel`
/// (i.e. one greater than the highest existing `{base_version}-{channel}.N` tag).
fn next_pre_release_number(repo: &git2::Repository, base_version: &str, channel: &str) -> u32 {
    let max_pre = repo
        .tag_names(None)
        .map(|names| {
            names
                .iter()
                .flatten()
                .filter_map(|t| versioning::pre_release_number(t, base_version, channel))
                .max()
                .unwrap_or(0)
        })
        .unwrap_or(0);
    max_pre.saturating_add(1)
}

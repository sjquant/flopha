use std::path::Path;

use crate::cli::{ChangelogArgs, LastVersionArgs, LogArgs, NextVersionArgs, OutputFormat, VersionSourceName};
use crate::error::FlophaError;
use crate::gitutils;
use crate::version_source::{BranchVersionSource, TagVersionSource, VersionSource};
use crate::versioning::{self, BumpRule, Increment, Versioner};
use crate::versioning::classify_commit;

pub fn last_version(path: &Path, args: &LastVersionArgs) -> Result<Option<String>, FlophaError> {
    let repo = gitutils::get_repo(path)?;
    try_fetch_from_origin(&repo);
    let pattern = args
        .pattern
        .clone()
        .unwrap_or("v{major}.{minor}.{patch}".to_string());
    let versioner = versioner_factory(&repo, pattern, &args.source);
    if let Some(version) = versioner.last_version() {
        match args.format {
            OutputFormat::Json => println!("{{\"version\":\"{}\"}}", version.tag),
            OutputFormat::Text => println!("{}", version.tag),
        }

        if args.checkout {
            let version_source = version_source_factory(&args.source);
            version_source.checkout(&repo, &version.tag)?;
        }

        Ok(Some(version.tag))
    } else {
        match args.format {
            OutputFormat::Json => println!("null"),
            OutputFormat::Text => println!("No version found"),
        }
        Ok(None)
    }
}

pub fn next_version(path: &Path, args: &NextVersionArgs) -> Result<Option<String>, FlophaError> {
    let repo = gitutils::get_repo(path)?;
    try_fetch_from_origin(&repo);
    let pattern = args
        .pattern
        .clone()
        .unwrap_or("v{major}.{minor}.{patch}".to_string());

    let version_source = version_source_factory(&args.source);
    let versioner = Versioner::new(version_source.fetch_all(&repo), pattern);

    let increment = if args.auto {
        let rules = build_rules(&args.rule)?;
        match versioner.last_version() {
            Some(last) => {
                let messages = gitutils::commits_since_tag(&repo, &last.tag).unwrap_or_default();
                versioning::detect_increment(&messages, &rules)
            }
            None => {
                log::warn!("--auto: no prior tag found, falling back to --increment");
                args.increment.clone()
            }
        }
    } else {
        args.increment.clone()
    };

    let next = match versioner.next_version(increment)? {
        Some(v) => v,
        None => {
            match args.format {
                OutputFormat::Json => println!("null"),
                OutputFormat::Text => println!("No version found"),
            }
            return Ok(None);
        }
    };

    let final_tag = if let Some(channel) = &args.pre {
        pre_release_tag(&next.tag, channel, &repo)
    } else {
        next.tag.clone()
    };

    match args.format {
        OutputFormat::Json => println!("{{\"version\":\"{}\"}}", final_tag),
        OutputFormat::Text => println!("{}", final_tag),
    }

    if args.create {
        version_source.create(&repo, &final_tag)?;
    }

    Ok(Some(final_tag))
}

/// Returns the next pre-release tag for `base_version` on `channel`.
///
/// Always scans the repo's actual git tags (not the version-source list, which
/// can be branch names when --source=branch is used) so the counter is correct
/// regardless of which version source drives the base version.
fn pre_release_tag(base_version: &str, channel: &str, repo: &git2::Repository) -> String {
    let prefix = format!("{}-{}.", base_version, channel);
    let max_pre = repo
        .tag_names(None)
        .map(|names| {
            names
                .iter()
                .flatten()
                .filter_map(|t| t.strip_prefix(&prefix))
                .filter_map(|s| s.parse::<u32>().ok())
                .max()
                .unwrap_or(0)
        })
        .unwrap_or(0);
    format!("{}-{}.{}", base_version, channel, max_pre.saturating_add(1))
}

pub fn log_versions(path: &Path, args: &LogArgs) -> Result<(), FlophaError> {
    let repo = gitutils::get_repo(path)?;
    try_fetch_from_origin(&repo);

    let pattern = args
        .pattern
        .clone()
        .unwrap_or("v{major}.{minor}.{patch}".to_string());
    let versioner = versioner_factory(&repo, pattern, &args.source);

    let mut versions = versioner.all_versions();
    versions.reverse();

    if let Some(limit) = args.limit {
        versions.truncate(limit);
    }

    if versions.is_empty() {
        match args.format {
            OutputFormat::Json => println!("[]"),
            OutputFormat::Text => println!("No versions found"),
        }
        return Ok(());
    }

    // Collect display rows: (tag, date_str, commit_count)
    let mut rows: Vec<(String, String, usize)> = Vec::new();
    for (i, version) in versions.iter().enumerate() {
        let date_str = gitutils::tag_commit_time(&repo, &version.tag)
            .map(format_date)
            .unwrap_or_else(|_| "unknown".to_string());

        let commit_count = if i + 1 < versions.len() {
            let prev = &versions[i + 1];
            let from_oid = gitutils::tag_commit_oid(&repo, &prev.tag).ok();
            let to_oid = gitutils::tag_commit_oid(&repo, &version.tag).ok();
            match (from_oid, to_oid) {
                (Some(from), Some(to)) => {
                    gitutils::count_commits_between(&repo, from, to).unwrap_or(0)
                }
                _ => 0,
            }
        } else {
            0
        };

        rows.push((version.tag.clone(), date_str, commit_count));
    }

    match args.format {
        OutputFormat::Json => {
            let entries: Vec<String> = rows
                .iter()
                .enumerate()
                .map(|(i, (tag, date, count))| {
                    let commits_val = if i + 1 < rows.len() {
                        count.to_string()
                    } else {
                        "null".to_string()
                    };
                    format!(
                        "{{\"version\":\"{}\",\"date\":\"{}\",\"commits\":{}}}",
                        tag, date, commits_val
                    )
                })
                .collect();
            println!("[{}]", entries.join(","));
        }
        OutputFormat::Text => {
            let tag_width = rows.iter().map(|(t, _, _)| t.len()).max().unwrap_or(0);
            let date_width = rows.iter().map(|(_, d, _)| d.len()).max().unwrap_or(0);

            for (i, (tag, date, count)) in rows.iter().enumerate() {
                let commit_info = if i + 1 < rows.len() {
                    format!("{} commit{}", count, if *count == 1 { "" } else { "s" })
                } else {
                    "\u{2014}".to_string()
                };
                let padded_date = format!("{:<date_width$}", date, date_width = date_width);
                println!(
                    "  {:<tag_width$}  {SEP}  {padded_date}  {SEP}  {commit_info}",
                    tag,
                    tag_width = tag_width,
                );
            }
        }
    }

    Ok(())
}

pub fn changelog(path: &Path, args: &ChangelogArgs) -> Result<(), FlophaError> {
    let repo = gitutils::get_repo(path)?;
    try_fetch_from_origin(&repo);

    let pattern = args
        .pattern
        .clone()
        .unwrap_or("v{major}.{minor}.{patch}".to_string());

    let from_tag = if let Some(ref from) = args.from {
        from.clone()
    } else {
        let versioner = versioner_factory(&repo, pattern, &args.source);
        match versioner.last_version() {
            Some(v) => v.tag,
            None => {
                match args.format {
                    OutputFormat::Json => println!("null"),
                    OutputFormat::Text => println!("No version found"),
                }
                return Ok(());
            }
        }
    };

    let rules = build_changelog_rules(&args.rule)?;
    let commits = gitutils::commits_since_tag_with_info(&repo, &from_tag).unwrap_or_default();

    let mut breaking: Vec<ChangelogEntry> = Vec::new();
    let mut features: Vec<ChangelogEntry> = Vec::new();
    let mut fixes: Vec<ChangelogEntry> = Vec::new();
    let mut other: Vec<ChangelogEntry> = Vec::new();

    for commit in &commits {
        let subject = commit.message.lines().next().unwrap_or("").trim().to_string();
        let entry = ChangelogEntry { subject, hash: commit.short_id.clone() };
        match classify_commit(&commit.message, &rules) {
            Some(Increment::Major) => breaking.push(entry),
            Some(Increment::Minor) => features.push(entry),
            Some(Increment::Patch) => fixes.push(entry),
            None => other.push(entry),
        }
    }

    let content = match args.format {
        OutputFormat::Json => format_changelog_json(&from_tag, &breaking, &features, &fixes, &other),
        OutputFormat::Text => format_changelog_text(&from_tag, &breaking, &features, &fixes, &other),
    };

    if let Some(ref output_path) = args.output {
        std::fs::write(output_path, &content)?;
    } else {
        print!("{}", content);
    }

    Ok(())
}

struct ChangelogEntry {
    subject: String,
    hash: String,
}

fn format_changelog_text(
    from: &str,
    breaking: &[ChangelogEntry],
    features: &[ChangelogEntry],
    fixes: &[ChangelogEntry],
    other: &[ChangelogEntry],
) -> String {
    let mut out = format!("## Changelog since {}\n", from);

    if !breaking.is_empty() {
        out.push_str("\n### Breaking Changes\n");
        for e in breaking {
            out.push_str(&format!("- {} ({})\n", e.subject, e.hash));
        }
    }
    if !features.is_empty() {
        out.push_str("\n### Features\n");
        for e in features {
            out.push_str(&format!("- {} ({})\n", e.subject, e.hash));
        }
    }
    if !fixes.is_empty() {
        out.push_str("\n### Bug Fixes\n");
        for e in fixes {
            out.push_str(&format!("- {} ({})\n", e.subject, e.hash));
        }
    }
    if !other.is_empty() {
        out.push_str("\n### Other Changes\n");
        for e in other {
            out.push_str(&format!("- {} ({})\n", e.subject, e.hash));
        }
    }

    out
}

fn format_changelog_json(
    from: &str,
    breaking: &[ChangelogEntry],
    features: &[ChangelogEntry],
    fixes: &[ChangelogEntry],
    other: &[ChangelogEntry],
) -> String {
    fn entries_to_json(entries: &[ChangelogEntry]) -> String {
        let items: Vec<String> = entries
            .iter()
            .map(|e| {
                format!(
                    "{{\"subject\":{},\"hash\":\"{}\"}}",
                    serde_json::Value::String(e.subject.clone()),
                    e.hash
                )
            })
            .collect();
        format!("[{}]", items.join(","))
    }

    format!(
        "{{\"from\":\"{}\",\"breaking_changes\":{},\"features\":{},\"fixes\":{},\"other\":{}}}",
        from,
        entries_to_json(breaking),
        entries_to_json(features),
        entries_to_json(fixes),
        entries_to_json(other),
    )
}

fn try_fetch_from_origin(repo: &git2::Repository) {
    match gitutils::get_remote(repo, "origin") {
        Ok(mut remote) => {
            if let Err(e) = gitutils::fetch_all(&mut remote) {
                log::warn!("Failed to fetch from origin: {}", e);
            }
        }
        Err(_) => log::debug!("No remote 'origin' found, using local data only"),
    }
}

const SEP: &str = "─";

fn format_date(ts: i64) -> String {
    let secs = ts.max(0) as u64;
    let days_since_epoch = secs / 86400;

    let mut remaining = days_since_epoch;
    let mut year = 1970u32;
    loop {
        let days_in_year = if is_leap(year) { 366 } else { 365 };
        if remaining < days_in_year {
            break;
        }
        remaining -= days_in_year;
        year += 1;
    }
    let leap = is_leap(year);
    let month_days: &[u64] = if leap {
        &[31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        &[31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut month = 1u32;
    for &md in month_days {
        if remaining < md {
            break;
        }
        remaining -= md;
        month += 1;
    }
    let day = remaining + 1;
    format!("{:04}-{:02}-{:02}", year, month, day)
}

fn is_leap(year: u32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn build_rules(raw_rules: &[String]) -> Result<Vec<BumpRule>, FlophaError> {
    if raw_rules.is_empty() {
        return Ok(versioning::conventional_bump_rules());
    }
    raw_rules.iter().map(|s| parse_bump_rule(s)).collect()
}

fn build_changelog_rules(raw_rules: &[String]) -> Result<Vec<BumpRule>, FlophaError> {
    if raw_rules.is_empty() {
        return Ok(versioning::conventional_changelog_rules());
    }
    raw_rules.iter().map(|s| parse_bump_rule(s)).collect()
}

fn parse_bump_rule(s: &str) -> Result<BumpRule, FlophaError> {
    let (level, pattern) = s.split_once(':').ok_or_else(|| FlophaError::InvalidRule {
        input: s.to_string(),
        reason: "expected format '<level>:<pattern>'".to_string(),
    })?;
    let increment = match level {
        "major" => Increment::Major,
        "minor" => Increment::Minor,
        "patch" => Increment::Patch,
        other => {
            return Err(FlophaError::InvalidRule {
                input: s.to_string(),
                reason: format!("unknown level '{}', expected major, minor, or patch", other),
            })
        }
    };
    BumpRule::new(pattern, increment).map_err(|e| FlophaError::InvalidRule {
        input: s.to_string(),
        reason: format!("invalid regex: {}", e),
    })
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::LastVersionArgs;
    use crate::versioning::Increment;
    use crate::{gitutils, testutils};

    #[test]
    fn test_last_version_tag_returns_latest_matching_pattern() {
        let (td, repo) = testutils::init_repo();
        let (_remote_td, mut remote) = testutils::init_remote(&repo);

        let tags = vec![
            "flopha@0.1.0",
            "flopha@1.0.0",
            "flopha@1.0.1",
            "flopha@1.1.1",
            "flopha@1.1.9",
            "flopha@2.10.11",
            "flopha@1.1.10",
            "flopha@2.9.9",
            "flopha@2.10.10",
            "v3.9.9",
        ];
        for tag in tags {
            create_new_remote_tag(&repo, &mut remote, tag, true);
        }

        let args = LastVersionArgs {
            pattern: Some("flopha@{major}.{minor}.{patch}".to_string()),
            source: VersionSourceName::Tag,
            checkout: false,
            format: OutputFormat::Text,
        };

        let result = last_version(td.path(), &args).unwrap();

        assert_eq!(result, Some("flopha@2.10.11".to_string()));
    }

    #[test]
    fn test_last_version_tag_returns_none_without_match() {
        let (td, repo) = testutils::init_repo();
        let (_remote_td, mut remote) = testutils::init_remote(&repo);

        let tags = vec!["v0.1.0", "v1.0.0", "v1.0.1"];
        for tag in tags {
            create_new_remote_tag(&repo, &mut remote, tag, true);
        }

        let args = LastVersionArgs {
            pattern: Some("flopha@{major}.{minor}.{patch}".to_string()),
            source: VersionSourceName::Tag,
            checkout: false,
            format: OutputFormat::Text,
        };
        let result = last_version(td.path(), &args).unwrap();

        assert_eq!(result, None);
    }

    #[test]
    fn test_last_version_tag_checkout_works() {
        let (td, repo) = testutils::init_repo();
        let (_remote_td, mut remote) = testutils::init_remote(&repo);

        let tags = vec![
            "flopha@0.1.0",
            "flopha@1.0.0",
            "flopha@1.0.1",
            "flopha@1.1.1",
            "flopha@1.1.2",
            "flopha@0.4.5",
        ];
        for tag in tags {
            create_new_remote_tag(&repo, &mut remote, tag, true);
        }

        let args = LastVersionArgs {
            pattern: Some("flopha@{major}.{minor}.{patch}".to_string()),
            source: VersionSourceName::Tag,
            checkout: true,
            format: OutputFormat::Text,
        };
        last_version(td.path(), &args).unwrap();

        let tag_id = repo.revparse_single("refs/tags/flopha@1.1.2").unwrap().id();
        let head_id = repo.head().unwrap().peel_to_commit().unwrap().id();
        assert_eq!(tag_id, head_id);
    }

    #[test]
    fn test_last_version_tag_returns_none_with_non_matching_pattern() {
        let (td, repo) = testutils::init_repo();
        let (_remote_td, mut remote) = testutils::init_remote(&repo);

        let tags = vec!["v1.0.0", "v1.1.0", "v2.0.0"];
        for tag in tags {
            create_new_remote_tag(&repo, &mut remote, tag, false);
        }

        let args = LastVersionArgs {
            pattern: Some("release-{major}.{minor}.{patch}".to_string()),
            source: VersionSourceName::Tag,
            checkout: false,
            format: OutputFormat::Text,
        };

        let result = last_version(td.path(), &args).unwrap();

        assert_eq!(result, None);
    }

    #[test]
    fn test_last_version_returns_last_version_with_given_pattern_for_branches() {
        let (td, repo) = testutils::init_repo();
        let (_remote_td, mut remote) = testutils::init_remote(&repo);

        let branches = vec![
            "release/0.1.0",
            "release/1.0.0",
            "release/1.0.1",
            "release/1.1.1",
            "release/1.1.9",
            "release/2.10.11",
            "release/1.1.10",
            "release/2.9.9",
            "release/2.10.10",
        ];
        for branch in branches {
            create_new_remote_branch(&repo, &mut remote, branch);
        }

        let args = LastVersionArgs {
            pattern: Some("release/{major}.{minor}.{patch}".to_string()),
            source: VersionSourceName::Branch,
            checkout: false,
            format: OutputFormat::Text,
        };

        let result = last_version(td.path(), &args).unwrap();

        assert_eq!(result, Some("release/2.10.11".to_string()));
    }

    #[test]
    fn test_last_version_branch_returns_latest_matching_pattern() {
        let (td, repo) = testutils::init_repo();
        let (_remote_td, mut remote) = testutils::init_remote(&repo);

        let branches = vec![
            "release/1.0.0",
            "release/1.1.0",
            "release/2.0.0",
            "main",
            "develop",
        ];
        for branch in branches {
            create_new_remote_branch(&repo, &mut remote, branch);
        }

        let args = LastVersionArgs {
            pattern: Some("release/{major}.{minor}.{patch}".to_string()),
            source: VersionSourceName::Branch,
            checkout: false,
            format: OutputFormat::Text,
        };

        let result = last_version(td.path(), &args).unwrap();

        assert_eq!(result, Some("release/2.0.0".to_string()));
    }

    #[test]
    fn test_last_version_branch_checkout_works() {
        let (td, repo) = testutils::init_repo();
        let (_remote_td, mut remote) = testutils::init_remote(&repo);

        let branches = vec![
            "release/1.0.0",
            "release/1.1.0",
            "release/2.0.0",
            "release/2.1.0",
        ];
        for branch in branches {
            create_new_remote_branch(&repo, &mut remote, branch);
        }

        let args = LastVersionArgs {
            pattern: Some("release/{major}.{minor}.{patch}".to_string()),
            source: VersionSourceName::Branch,
            checkout: true,
            format: OutputFormat::Text,
        };
        last_version(td.path(), &args).unwrap();

        let branch_id = repo
            .revparse_single("refs/heads/release/2.1.0")
            .unwrap()
            .id();
        let head_id = repo.head().unwrap().peel_to_commit().unwrap().id();
        assert_eq!(branch_id, head_id);
    }

    #[test]
    fn test_next_version_returns_next_version_with_given_pattern() {
        let (td, repo) = testutils::init_repo();
        let (_remote_td, mut remote) = testutils::init_remote(&repo);
        let tags = vec![
            "flopha@0.1.0",
            "flopha@1.0.0",
            "flopha@1.0.1",
            "flopha@1.1.1",
            "flopha@1.1.9",
            "flopha@2.10.11",
            "flopha@1.1.10",
            "flopha@2.9.9",
            "flopha@2.10.10",
            "v3.9.9",
        ];
        for tag in tags {
            create_new_remote_tag(&repo, &mut remote, tag, false);
        }
        gitutils::checkout_tag(&repo, "flopha@2.10.11").unwrap();
        gitutils::commit(&repo, "New commit").unwrap();

        let args = NextVersionArgs {
            pattern: Some("flopha@{major}.{minor}.{patch}".to_string()),
            increment: Increment::Patch,
            auto: false,
            rule: vec![],
            pre: None,
            source: VersionSourceName::Tag,
            create: false,
            format: OutputFormat::Text,
        };
        let result = next_version(td.path(), &args).unwrap();

        assert_eq!(result, Some("flopha@2.10.12".to_string()))
    }

    #[test]
    fn test_next_version_with_tag_create_action() {
        let (td, repo) = testutils::init_repo();
        let (_remote_td, mut remote) = testutils::init_remote(&repo);
        let tags = vec![
            "flopha@0.1.0",
            "flopha@1.0.0",
            "flopha@1.0.1",
            "flopha@1.1.1",
            "flopha@1.1.2",
            "flopha@0.4.5",
        ];
        for tag in tags {
            create_new_remote_tag(&repo, &mut remote, tag, false);
        }
        gitutils::checkout_tag(&repo, "flopha@1.1.2").unwrap();
        gitutils::commit(&repo, "New commit").unwrap();

        let args = NextVersionArgs {
            pattern: Some("flopha@{major}.{minor}.{patch}".to_string()),
            increment: Increment::Patch,
            auto: false,
            rule: vec![],
            pre: None,
            source: VersionSourceName::Tag,
            create: true,
            format: OutputFormat::Text,
        };
        next_version(td.path(), &args).unwrap();

        let tag_id = repo.revparse_single("refs/tags/flopha@1.1.3").unwrap().id();
        let head_id = repo.head().unwrap().peel_to_commit().unwrap().id();
        assert_eq!(tag_id, head_id);
    }

    #[test]
    fn next_version_branch_returns_next_version_with_pattern() {
        let (td, repo) = testutils::init_repo();
        let (_remote_td, mut remote) = testutils::init_remote(&repo);
        let branches = vec![
            "release/0.1.0",
            "release/1.0.0",
            "release/1.0.1",
            "release/1.1.1",
            "release/1.1.9",
            "release/2.10.11",
            "release/1.1.10",
            "release/2.9.9",
            "release/2.10.10",
        ];
        for branch in branches {
            create_new_remote_branch(&repo, &mut remote, branch);
        }
        gitutils::checkout_branch(&repo, "release/2.10.11", false).unwrap();
        gitutils::commit(&repo, "New commit").unwrap();

        let args = NextVersionArgs {
            pattern: Some("release/{major}.{minor}.{patch}".to_string()),
            increment: Increment::Patch,
            auto: false,
            rule: vec![],
            pre: None,
            source: VersionSourceName::Branch,
            create: false,
            format: OutputFormat::Text,
        };
        let result = next_version(td.path(), &args).unwrap();

        assert_eq!(result, Some("release/2.10.12".to_string()))
    }

    #[test]
    fn test_next_version_branch_returns_none_without_match() {
        let (td, repo) = testutils::init_repo();
        let (_remote_td, mut remote) = testutils::init_remote(&repo);

        let branches = vec!["main", "develop", "feature/new-feature"];
        for branch in branches {
            create_new_remote_branch(&repo, &mut remote, branch);
        }

        let args = NextVersionArgs {
            pattern: Some("release/{major}.{minor}.{patch}".to_string()),
            increment: Increment::Patch,
            auto: false,
            rule: vec![],
            pre: None,
            source: VersionSourceName::Branch,
            create: false,
            format: OutputFormat::Text,
        };

        let result = next_version(td.path(), &args).unwrap();

        assert_eq!(result, None);
    }

    #[test]
    fn test_next_version_branch_with_create_action() {
        let (td, repo) = testutils::init_repo();
        let (_remote_td, mut remote) = testutils::init_remote(&repo);

        let branches = vec!["release/1.0.0", "release/1.1.0", "release/2.0.0"];
        for branch in branches {
            create_new_remote_branch(&repo, &mut remote, branch);
        }
        gitutils::checkout_branch(&repo, "release/2.0.0", false).unwrap();
        gitutils::commit(&repo, "New commit").unwrap();

        let args = NextVersionArgs {
            pattern: Some("release/{major}.{minor}.{patch}".to_string()),
            increment: Increment::Minor,
            auto: false,
            rule: vec![],
            pre: None,
            source: VersionSourceName::Branch,
            create: true,
            format: OutputFormat::Text,
        };
        let result = next_version(td.path(), &args).unwrap();

        assert_eq!(result, Some("release/2.1.0".to_string()));

        let branches = repo.branches(Some(git2::BranchType::Local)).unwrap();
        assert!(branches.into_iter().any(|b| {
            let (branch, _) = b.unwrap();
            branch.name().unwrap() == Some("release/2.1.0")
        }));
    }

    #[test]
    fn test_next_version_auto_detects_feat_as_minor() {
        let (td, repo) = testutils::init_repo();
        let (_remote_td, mut remote) = testutils::init_remote(&repo);

        let tags = vec!["v1.0.0", "v1.1.0"];
        for tag in tags {
            create_new_remote_tag(&repo, &mut remote, tag, false);
        }
        gitutils::checkout_tag(&repo, "v1.1.0").unwrap();
        gitutils::commit(&repo, "feat: add new command").unwrap();

        let args = NextVersionArgs {
            pattern: Some("v{major}.{minor}.{patch}".to_string()),
            increment: Increment::Patch,
            auto: true,
            rule: vec![],
            pre: None,
            source: VersionSourceName::Tag,
            create: false,
            format: OutputFormat::Text,
        };
        let result = next_version(td.path(), &args).unwrap();

        assert_eq!(result, Some("v1.2.0".to_string()));
    }

    #[test]
    fn test_next_version_pre_release_starts_at_1() {
        let (td, repo) = testutils::init_repo();
        let (_remote_td, mut remote) = testutils::init_remote(&repo);

        let tags = vec!["v1.0.0"];
        for tag in tags {
            create_new_remote_tag(&repo, &mut remote, tag, false);
        }
        gitutils::checkout_tag(&repo, "v1.0.0").unwrap();
        gitutils::commit(&repo, "fix: something").unwrap();

        let args = NextVersionArgs {
            pattern: Some("v{major}.{minor}.{patch}".to_string()),
            increment: Increment::Patch,
            auto: false,
            rule: vec![],
            pre: Some("alpha".to_string()),
            source: VersionSourceName::Tag,
            create: false,
            format: OutputFormat::Text,
        };
        let result = next_version(td.path(), &args).unwrap();

        assert_eq!(result, Some("v1.0.1-alpha.1".to_string()));
    }

    #[test]
    fn test_next_version_pre_release_increments() {
        let (td, repo) = testutils::init_repo();
        let (_remote_td, mut remote) = testutils::init_remote(&repo);

        let tags = vec!["v1.0.0", "v1.0.1-alpha.1"];
        for tag in tags {
            create_new_remote_tag(&repo, &mut remote, tag, false);
        }
        gitutils::checkout_tag(&repo, "v1.0.0").unwrap();
        gitutils::commit(&repo, "fix: something").unwrap();

        let args = NextVersionArgs {
            pattern: Some("v{major}.{minor}.{patch}".to_string()),
            increment: Increment::Patch,
            auto: false,
            rule: vec![],
            pre: Some("alpha".to_string()),
            source: VersionSourceName::Tag,
            create: false,
            format: OutputFormat::Text,
        };
        let result = next_version(td.path(), &args).unwrap();

        assert_eq!(result, Some("v1.0.1-alpha.2".to_string()));
    }

    #[test]
    fn test_next_version_auto_with_custom_rules() {
        let (td, repo) = testutils::init_repo();
        let (_remote_td, mut remote) = testutils::init_remote(&repo);

        let tags = vec!["v1.0.0"];
        for tag in tags {
            create_new_remote_tag(&repo, &mut remote, tag, false);
        }
        gitutils::checkout_tag(&repo, "v1.0.0").unwrap();
        gitutils::commit(&repo, "feat: add thing").unwrap();

        let args = NextVersionArgs {
            pattern: Some("v{major}.{minor}.{patch}".to_string()),
            increment: Increment::Patch,
            auto: true,
            rule: vec!["major:BUMP_MAJOR:".to_string()],
            pre: None,
            source: VersionSourceName::Tag,
            create: false,
            format: OutputFormat::Text,
        };
        let result = next_version(td.path(), &args).unwrap();

        assert_eq!(result, Some("v1.0.1".to_string()));
    }

    #[test]
    fn test_changelog_categorizes_conventional_commits() {
        let (td, repo) = testutils::init_repo();
        let (_remote_td, mut remote) = testutils::init_remote(&repo);

        create_new_remote_tag(&repo, &mut remote, "v1.0.0", false);
        gitutils::checkout_tag(&repo, "v1.0.0").unwrap();
        gitutils::commit(&repo, "feat: add search").unwrap();
        gitutils::commit(&repo, "fix: crash on empty input").unwrap();
        gitutils::commit(&repo, "chore: update deps").unwrap();

        let args = ChangelogArgs {
            from: Some("v1.0.0".to_string()),
            pattern: Some("v{major}.{minor}.{patch}".to_string()),
            source: VersionSourceName::Tag,
            rule: vec![],
            output: None,
            format: OutputFormat::Text,
        };

        // Should not error and should produce non-empty output.
        changelog(td.path(), &args).unwrap();
    }

    #[test]
    fn test_changelog_custom_rules() {
        let (td, repo) = testutils::init_repo();
        let (_remote_td, mut remote) = testutils::init_remote(&repo);

        create_new_remote_tag(&repo, &mut remote, "v1.0.0", false);
        gitutils::checkout_tag(&repo, "v1.0.0").unwrap();
        gitutils::commit(&repo, "BUMP_MAJOR: api overhaul").unwrap();
        gitutils::commit(&repo, "ADD: new endpoint").unwrap();

        let args = ChangelogArgs {
            from: Some("v1.0.0".to_string()),
            pattern: Some("v{major}.{minor}.{patch}".to_string()),
            source: VersionSourceName::Tag,
            rule: vec!["major:BUMP_MAJOR:".to_string(), "minor:^ADD:".to_string()],
            output: None,
            format: OutputFormat::Text,
        };

        // Custom rules should categorize without error.
        changelog(td.path(), &args).unwrap();
    }

    fn create_new_remote_tag(
        repo: &git2::Repository,
        remote: &mut git2::Remote,
        tag: &str,
        should_delete: bool,
    ) {
        let commit_id = gitutils::commit(repo, "New commit").unwrap();
        gitutils::tag_oid(repo, commit_id, tag).unwrap();
        remote.push(&[format!("refs/tags/{}", tag)], None).unwrap();

        if should_delete {
            repo.tag_delete(tag).unwrap();
        }
    }

    fn create_new_remote_branch(repo: &git2::Repository, remote: &mut git2::Remote, branch: &str) {
        gitutils::checkout_branch(repo, branch, true).unwrap();
        gitutils::commit(repo, "New commit").unwrap();
        let mut branch = repo.find_branch(branch, git2::BranchType::Local).unwrap();
        gitutils::push_branch(remote, &mut branch).unwrap();
    }
}

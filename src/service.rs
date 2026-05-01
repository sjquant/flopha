use std::path::Path;

use crate::cli::{LastVersionArgs, LogArgs, NextVersionArgs, VersionSourceName};
use crate::error::FlophaError;
use crate::gitutils;
use crate::version_source::{BranchVersionSource, TagVersionSource, VersionSource};
use crate::versioning::{self, BumpRule, Increment, Versioner};

pub fn last_version(path: &Path, args: &LastVersionArgs) -> Result<Option<String>, FlophaError> {
    let repo = gitutils::get_repo(path)?;
    let mut remote = gitutils::get_remote(&repo, "origin")?;
    gitutils::fetch_all(&mut remote)?;
    let pattern = args
        .pattern
        .clone()
        .unwrap_or("v{major}.{minor}.{patch}".to_string());
    let versioner = versioner_factory(&repo, pattern, &args.source);
    if let Some(version) = versioner.last_version() {
        println!("{}", version.tag);

        if args.checkout {
            let version_source = version_source_factory(&args.source);
            version_source.checkout(&repo, &version.tag)?;
        }

        Ok(Some(version.tag))
    } else {
        println!("No version found");
        Ok(None)
    }
}

pub fn next_version(path: &Path, args: &NextVersionArgs) -> Result<Option<String>, FlophaError> {
    let repo = gitutils::get_repo(path)?;
    let mut remote = gitutils::get_remote(&repo, "origin")?;
    gitutils::fetch_all(&mut remote)?;
    let pattern = args
        .pattern
        .clone()
        .unwrap_or("v{major}.{minor}.{patch}".to_string());

    let version_source = version_source_factory(&args.source);
    let versioner = Versioner::new(version_source.fetch_all(&repo), pattern);

    // Determine increment level, honouring --auto if set.
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
            println!("No version found");
            return Ok(None);
        }
    };

    // If a pre-release channel was requested, compute the pre-release tag.
    let final_tag = if let Some(channel) = &args.pre {
        pre_release_tag(&next.tag, channel, &repo)
    } else {
        next.tag.clone()
    };

    println!("{}", final_tag);

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
    let mut remote = gitutils::get_remote(&repo, "origin")?;
    gitutils::fetch_all(&mut remote)?;

    let pattern = args
        .pattern
        .clone()
        .unwrap_or("v{major}.{minor}.{patch}".to_string());
    let versioner = versioner_factory(&repo, pattern, &args.source);

    let mut versions = versioner.all_versions();
    // Show newest first.
    versions.reverse();

    if let Some(limit) = args.limit {
        versions.truncate(limit);
    }

    if versions.is_empty() {
        println!("No versions found");
        return Ok(());
    }

    // Collect display rows: (tag, date_str, commit_count_str)
    let mut rows: Vec<(String, String, String)> = Vec::new();
    for (i, version) in versions.iter().enumerate() {
        let date_str = gitutils::tag_commit_time(&repo, &version.tag)
            .map(format_date)
            .unwrap_or_else(|_| "unknown".to_string());

        // Count commits between this version and the next older one.
        let commit_info = if i + 1 < versions.len() {
            let prev = &versions[i + 1];
            let from_oid = gitutils::tag_commit_oid(&repo, &prev.tag).ok();
            let to_oid = gitutils::tag_commit_oid(&repo, &version.tag).ok();
            let count = match (from_oid, to_oid) {
                (Some(from), Some(to)) => {
                    gitutils::count_commits_between(&repo, from, to).unwrap_or(0)
                }
                _ => 0,
            };
            format!("{} commit{}", count, if count == 1 { "" } else { "s" })
        } else {
            // Oldest release: no prior tag boundary exists, so showing a raw count would
            // include the entire project history and be misleading.
            "\u{2014}".to_string()
        };

        rows.push((version.tag.clone(), date_str, commit_info));
    }

    // Align columns.
    let tag_width = rows.iter().map(|(t, _, _)| t.len()).max().unwrap_or(0);
    let date_width = rows.iter().map(|(_, d, _)| d.len()).max().unwrap_or(0);

    for (tag, date, commits) in &rows {
        let padded_date = format!("{:<date_width$}", date, date_width = date_width);
        println!(
            "  {:<tag_width$}  {SEP}  {padded_date}  {SEP}  {commits}",
            tag,
            tag_width = tag_width,
        );
    }

    Ok(())
}

const SEP: &str = "─";

/// Formats a Unix timestamp as `YYYY-MM-DD`.
fn format_date(ts: i64) -> String {
    // Days since Unix epoch.
    let secs = ts.max(0) as u64;
    let days_since_epoch = secs / 86400;

    // Gregorian calendar calculation (no external dep needed for dates after 1970).
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

/// Returns the rule set to use for `--auto`.
///
/// If `raw_rules` is empty the built-in conventional-commit defaults are used.
/// Otherwise each entry is parsed as `<level>:<regex>` and any parse error is
/// surfaced immediately so the user gets a clear message before any git I/O.
fn build_rules(raw_rules: &[String]) -> Result<Vec<BumpRule>, FlophaError> {
    if raw_rules.is_empty() {
        return Ok(versioning::conventional_bump_rules());
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
        };
        let result = next_version(td.path(), &args).unwrap();

        assert_eq!(result, Some("v1.0.1-alpha.1".to_string()));
    }

    #[test]
    fn test_next_version_pre_release_increments() {
        let (td, repo) = testutils::init_repo();
        let (_remote_td, mut remote) = testutils::init_remote(&repo);

        // v1.0.1-alpha.1 already exists; next should be alpha.2
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
        // This commit would be "minor" under conventional commits, but with a
        // custom rule only "BUMP_MAJOR:" triggers major and nothing else matches minor.
        gitutils::commit(&repo, "feat: add thing").unwrap();

        let args = NextVersionArgs {
            pattern: Some("v{major}.{minor}.{patch}".to_string()),
            increment: Increment::Patch,
            auto: true,
            rule: vec!["major:BUMP_MAJOR:".to_string()],
            pre: None,
            source: VersionSourceName::Tag,
            create: false,
        };
        let result = next_version(td.path(), &args).unwrap();

        // "feat:" doesn't match any custom rule → falls through to patch
        assert_eq!(result, Some("v1.0.1".to_string()));
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

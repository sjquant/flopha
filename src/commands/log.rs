use std::path::Path;

use crate::cli::{LogArgs, OutputFormat};
use crate::error::FlophaError;
use crate::gitutils;
use crate::versioning::Version;

pub fn log_versions(path: &Path, args: &LogArgs) -> Result<(), FlophaError> {
    let repo = gitutils::get_repo(path)?;
    gitutils::try_fetch_origin(&repo);

    let pattern = super::pattern_or_default(&args.pattern);
    let versioner = super::versioner_factory(&repo, pattern, &args.source);

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

    print_log(&log_rows(&repo, &versions), &args.format);
    Ok(())
}

/// Builds one row per version, newest first, as `versions` is ordered.
fn log_rows(repo: &git2::Repository, versions: &[Version]) -> Vec<LogRow> {
    versions
        .iter()
        .enumerate()
        .map(|(i, version)| {
            let date = gitutils::tag_commit_time(repo, &version.tag)
                .map(format_date)
                .unwrap_or_else(|_| "unknown".to_string());
            let commits = versions.get(i + 1).map(|prev| {
                let from_oid = gitutils::tag_commit_oid(repo, &prev.tag).ok();
                let to_oid = gitutils::tag_commit_oid(repo, &version.tag).ok();
                match (from_oid, to_oid) {
                    (Some(from), Some(to)) => {
                        gitutils::count_commits_between(repo, from, to).unwrap_or(0)
                    }
                    _ => 0,
                }
            });
            LogRow {
                tag: version.tag.clone(),
                date,
                commits,
            }
        })
        .collect()
}

struct LogRow {
    tag: String,
    date: String,
    /// Commits since the previous (older) version; `None` for the oldest one listed.
    commits: Option<usize>,
}

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

fn print_log(rows: &[LogRow], format: &OutputFormat) {
    match format {
        OutputFormat::Json => {
            let entries: Vec<serde_json::Value> = rows
                .iter()
                .map(|row| {
                    serde_json::json!({"version": row.tag, "date": row.date, "commits": row.commits})
                })
                .collect();
            println!("{}", serde_json::Value::Array(entries));
        }
        OutputFormat::Text => {
            let tag_width = rows.iter().map(|row| row.tag.len()).max().unwrap_or(0);
            let date_width = rows.iter().map(|row| row.date.len()).max().unwrap_or(0);

            for row in rows {
                let commit_info = match row.commits {
                    Some(count) => format!("{} commit{}", count, if count == 1 { "" } else { "s" }),
                    None => "\u{2014}".to_string(),
                };
                let padded_date = format!("{:<date_width$}", row.date, date_width = date_width);
                println!(
                    "  {:<tag_width$}  {SEP}  {padded_date}  {SEP}  {commit_info}",
                    row.tag,
                    tag_width = tag_width,
                );
            }
        }
    }
}

const SEP: &str = "─";

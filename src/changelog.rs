use std::collections::HashMap;

use crate::cli::OutputFormat;

use crate::error::FlophaError;

use crate::gitutils::{self, CommitInfo};

/// Parameters for [`build_changelog`], bundled into a struct (rather than passed
/// positionally) so call sites read as named fields instead of relying on
/// argument order to convey meaning.
pub(crate) struct ChangelogRequest<'a> {
    pub from_tag: Option<&'a str>,
    /// Upper bound for the commit range (inclusive); HEAD when `None`. Must be
    /// a resolvable ref/tag when set.
    pub to: Option<&'a str>,
    pub raw_groups: &'a [String],
    pub other: Option<&'a str>,
    pub title_template: Option<&'a str>,
    pub format: &'a OutputFormat,
}

/// Builds changelog content (grouped, formatted) for commits between `from_tag`
/// (exclusive) and `to` (inclusive; HEAD when `None`). Shared by the `changelog`
/// and `release` commands.
pub(crate) fn build_changelog(
    repo: &git2::Repository,
    req: &ChangelogRequest,
) -> Result<String, FlophaError> {
    let group_rules = build_group_rules(req.raw_groups)?;
    let commits = match req.from_tag {
        Some(tag) => gitutils::commits_since_tag_with_info(repo, tag, req.to)?,
        None => gitutils::all_commits_with_info(repo, req.to)?,
    };
    let groups = group_commits(&commits, &group_rules, req.other);
    let title = changelog_title(req.title_template, req.from_tag, req.to);

    Ok(match req.format {
        OutputFormat::Json => format_changelog_json(&title, req.from_tag, req.to, &groups),
        OutputFormat::Text => format_changelog_text(&title, &groups),
    })
}

fn build_group_rules(raw: &[String]) -> Result<Vec<GroupRule>, FlophaError> {
    if raw.is_empty() {
        return Ok(default_changelog_groups());
    }
    raw.iter().map(|s| parse_group_rule(s)).collect()
}

fn default_changelog_groups() -> Vec<GroupRule> {
    vec![
        GroupRule::new(
            "Breaking Changes",
            r"BREAKING[- ]CHANGE|(?m)^[a-z]+(\([^)]+\))?!:",
        )
        .unwrap(),
        GroupRule::new("Features", r"(?m)^feat(\([^)]+\))?:").unwrap(),
        GroupRule::new("Bug Fixes", r"(?m)^fix(\([^)]+\))?:").unwrap(),
    ]
}

fn parse_group_rule(s: &str) -> Result<GroupRule, FlophaError> {
    let (title, pattern) = s.split_once(':').ok_or_else(|| FlophaError::InvalidRule {
        input: s.to_string(),
        reason: "expected format 'TITLE:PATTERN'".to_string(),
    })?;
    GroupRule::new(title, pattern).map_err(|e| FlophaError::InvalidRule {
        input: s.to_string(),
        reason: format!("invalid regex: {}", e),
    })
}

struct GroupRule {
    title: String,
    pattern: regex::Regex,
}

impl GroupRule {
    fn new(title: &str, pattern: &str) -> Result<Self, regex::Error> {
        Ok(Self {
            title: title.to_string(),
            pattern: regex::Regex::new(pattern)?,
        })
    }
}

/// Sorts commits into the first matching rule's section, in rule order, skipping
/// empty sections. Unmatched commits go under `other` ("Other Changes" when
/// unset, dropped when empty).
fn group_commits(
    commits: &[CommitInfo],
    group_rules: &[GroupRule],
    other: Option<&str>,
) -> Vec<(String, Vec<ChangelogEntry>)> {
    let mut group_order: Vec<String> = Vec::new();
    for rule in group_rules {
        if !group_order.contains(&rule.title) {
            group_order.push(rule.title.clone());
        }
    }
    let mut group_entries: HashMap<String, Vec<ChangelogEntry>> = HashMap::new();
    let mut other_entries: Vec<ChangelogEntry> = Vec::new();

    for commit in commits {
        let subject = commit
            .message
            .lines()
            .next()
            .unwrap_or("")
            .trim()
            .to_string();
        let entry = ChangelogEntry {
            subject,
            hash: commit.short_id.clone(),
        };
        match group_rules
            .iter()
            .find(|r| r.pattern.is_match(&commit.message))
        {
            Some(rule) => group_entries
                .entry(rule.title.clone())
                .or_default()
                .push(entry),
            None => other_entries.push(entry),
        }
    }

    let mut groups: Vec<(String, Vec<ChangelogEntry>)> = group_order
        .into_iter()
        .filter_map(|title| {
            group_entries
                .remove(&title)
                .filter(|e| !e.is_empty())
                .map(|e| (title, e))
        })
        .collect();
    if !other_entries.is_empty() {
        match other {
            Some("") => {}
            Some(title) => groups.push((title.to_string(), other_entries)),
            None => groups.push(("Other Changes".to_string(), other_entries)),
        }
    }
    groups
}

struct ChangelogEntry {
    subject: String,
    hash: String,
}

fn changelog_title(
    title_template: Option<&str>,
    from_tag: Option<&str>,
    to: Option<&str>,
) -> String {
    match title_template {
        Some(t) => {
            if t.contains("{to}") && to.is_none() {
                log::warn!(
                    "--title contains {{to}} but --to was not supplied; placeholder will be empty"
                );
            }
            t.replace("{from}", from_tag.unwrap_or(""))
                .replace("{to}", to.unwrap_or(""))
        }
        None => match to {
            Some(to) => format!("Changes in {}", to),
            None => match from_tag {
                Some(from) => format!("Changelog since {}", from),
                None => "Initial Changelog".to_string(),
            },
        },
    }
}

fn format_changelog_json(
    title: &str,
    from: Option<&str>,
    to: Option<&str>,
    groups: &[(String, Vec<ChangelogEntry>)],
) -> String {
    let groups_val: Vec<serde_json::Value> = groups
        .iter()
        .map(|(group_title, entries)| {
            let entries_val: Vec<serde_json::Value> = entries
                .iter()
                .map(|e| serde_json::json!({"subject": e.subject, "hash": e.hash}))
                .collect();
            serde_json::json!({"title": group_title, "entries": entries_val})
        })
        .collect();
    let mut obj = serde_json::json!({"title": title, "from": from, "groups": groups_val});
    if let Some(to) = to {
        obj["to"] = serde_json::json!(to);
    }
    obj.to_string() + "\n"
}

fn format_changelog_text(title: &str, groups: &[(String, Vec<ChangelogEntry>)]) -> String {
    let mut out = format!("## {}\n", title);
    for (group_title, entries) in groups {
        out.push_str(&format!("\n### {}\n", group_title));
        for e in entries {
            out.push_str(&format!("- {} ({})\n", e.subject, e.hash));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_changelog_json_structure() {
        let groups = vec![
            (
                "Features".to_string(),
                vec![ChangelogEntry {
                    subject: "add search".to_string(),
                    hash: "abc1234".to_string(),
                }],
            ),
            (
                "Bug Fixes".to_string(),
                vec![ChangelogEntry {
                    subject: "fix crash".to_string(),
                    hash: "def5678".to_string(),
                }],
            ),
        ];
        let json = format_changelog_json("Changelog since v1.0.0", Some("v1.0.0"), None, &groups);
        let v: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");

        assert_eq!(v["title"], "Changelog since v1.0.0");
        assert_eq!(v["from"], "v1.0.0");
        assert!(v.get("to").is_none());
        assert_eq!(v["groups"].as_array().unwrap().len(), 2);
        assert_eq!(v["groups"][0]["title"], "Features");
        assert_eq!(v["groups"][0]["entries"][0]["subject"], "add search");
        assert_eq!(v["groups"][0]["entries"][0]["hash"], "abc1234");
        assert_eq!(v["groups"][1]["title"], "Bug Fixes");
    }

    #[test]
    fn test_changelog_json_escapes_special_chars() {
        let groups = vec![(
            r#"Group "A""#.to_string(),
            vec![ChangelogEntry {
                subject: r#"feat: support "quoted" args and backslash \"#.to_string(),
                hash: "abc1234".to_string(),
            }],
        )];
        let json = format_changelog_json(
            r#"Release v1.0.0"edge""#,
            Some(r#"v1.0.0"edge"#),
            Some(r#"v1.0.0"edge""#),
            &groups,
        );

        // Must parse without error despite embedded quotes and backslashes.
        let v: serde_json::Value =
            serde_json::from_str(&json).expect("valid JSON with special chars");
        assert_eq!(v["title"], r#"Release v1.0.0"edge""#);
        assert_eq!(v["from"], r#"v1.0.0"edge"#);
        assert_eq!(v["to"], r#"v1.0.0"edge""#);
        assert_eq!(v["groups"][0]["title"], r#"Group "A""#);
        assert_eq!(
            v["groups"][0]["entries"][0]["subject"],
            r#"feat: support "quoted" args and backslash \"#
        );
    }

    #[test]
    fn test_changelog_json_empty_groups() {
        let json = format_changelog_json("Changelog since v1.0.0", Some("v1.0.0"), None, &[]);
        let v: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
        assert_eq!(v["from"], "v1.0.0");
        assert!(v.get("to").is_none());
        assert!(v["groups"].as_array().unwrap().is_empty());
    }

    #[test]
    fn test_changelog_json_with_to_field() {
        let groups = vec![(
            "Features".to_string(),
            vec![ChangelogEntry {
                subject: "add search".to_string(),
                hash: "abc1234".to_string(),
            }],
        )];
        let json =
            format_changelog_json("Changes in v1.1.0", Some("v1.0.0"), Some("v1.1.0"), &groups);
        let v: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
        assert_eq!(v["title"], "Changes in v1.1.0");
        assert_eq!(v["from"], "v1.0.0");
        assert_eq!(v["to"], "v1.1.0");
        assert_eq!(v["groups"][0]["title"], "Features");
    }
}

use clap::ValueEnum;
use regex::Regex;

use crate::error::FlophaError;

/// A single rule that maps a regex pattern (matched against a commit message) to an
/// [`Increment`] level.  Rules are evaluated with major > minor > patch precedence.
pub struct BumpRule {
    pub pattern: Regex,
    pub increment: Increment,
}

impl BumpRule {
    pub fn new(pattern: &str, increment: Increment) -> Result<Self, regex::Error> {
        Ok(Self {
            pattern: Regex::new(pattern)?,
            increment,
        })
    }
}

/// The built-in conventional-commit rules used when no `--rule` flags are supplied.
///
/// | Pattern | Bump |
/// |---------|------|
/// | `BREAKING CHANGE` / `BREAKING-CHANGE` anywhere in message | major |
/// | `!` after type (e.g. `feat!:`, `feat(api)!:`) | major |
/// | `feat:` / `feat(<scope>):` at line start | minor |
pub fn conventional_bump_rules() -> Vec<BumpRule> {
    vec![
        BumpRule::new(r"BREAKING[- ]CHANGE", Increment::Major).unwrap(),
        BumpRule::new(r"(?m)^[a-z]+(\([^)]+\))?!:", Increment::Major).unwrap(),
        BumpRule::new(r"(?m)^feat(\([^)]+\))?:", Increment::Minor).unwrap(),
    ]
}

/// Infers the highest-priority [`Increment`] from `messages` using `rules`.
///
/// Every rule is tested against every message independently; the highest-priority
/// match across the whole set wins (major > minor > patch).  Returns `Patch` when
/// nothing matches.
pub fn detect_increment(messages: &[String], rules: &[BumpRule]) -> Increment {
    let mut result = Increment::Patch;
    for message in messages {
        for rule in rules {
            if rule.pattern.is_match(message) {
                match rule.increment {
                    Increment::Major => return Increment::Major,
                    Increment::Minor => result = Increment::Minor,
                    Increment::Patch => {}
                }
            }
        }
    }
    result
}

pub struct Versioner {
    tags: Vec<String>,
    pattern: String,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Version {
    pub tag: String,
    pub major: Option<u32>,
    pub minor: Option<u32>,
    pub patch: Option<u32>,
}

impl Version {
    pub fn new(tag: String, major: Option<u32>, minor: Option<u32>, patch: Option<u32>) -> Self {
        Self {
            tag,
            major,
            minor,
            patch,
        }
    }
}

#[derive(Debug, Clone, ValueEnum)]
pub enum Increment {
    Major,
    Minor,
    Patch,
}

impl Versioner {
    pub fn new(tags: Vec<String>, pattern: String) -> Self {
        Self { tags, pattern }
    }

    pub fn last_version(&self) -> Option<Version> {
        self.sorted_versions().into_iter().next_back()
    }

    /// Returns all versions matching the pattern, sorted ascending (oldest first).
    pub fn all_versions(&self) -> Vec<Version> {
        self.sorted_versions()
    }

    fn sorted_versions(&self) -> Vec<Version> {
        let regex = self.get_regex();
        let mut versions: Vec<Version> = self
            .tags
            .iter()
            .filter_map(|tag| {
                let caps = regex.captures(tag)?;
                let major = parse_version(&caps, "major");
                let minor = parse_version(&caps, "minor");
                let patch = parse_version(&caps, "patch");
                Some(Version::new(tag.to_string(), major, minor, patch))
            })
            .collect();
        versions.sort_by(|a, b| {
            a.major
                .cmp(&b.major)
                .then(a.minor.cmp(&b.minor))
                .then(a.patch.cmp(&b.patch))
        });
        versions
    }

    pub fn next_version(&self, increment: Increment) -> Result<Option<Version>, FlophaError> {
        let last_version = match self.last_version() {
            Some(v) => v,
            None => return Ok(None),
        };

        let (major, minor, patch) = match increment {
            Increment::Major => {
                let major = last_version
                    .major
                    .ok_or(FlophaError::MissingVersionComponent("major".into()))?
                    .saturating_add(1);
                (major, 0, 0)
            }
            Increment::Minor => {
                let major = last_version
                    .major
                    .ok_or(FlophaError::MissingVersionComponent("major".into()))?;
                let minor = last_version
                    .minor
                    .ok_or(FlophaError::MissingVersionComponent("minor".into()))?
                    .saturating_add(1);
                (major, minor, 0)
            }
            Increment::Patch => {
                let major = last_version
                    .major
                    .ok_or(FlophaError::MissingVersionComponent("major".into()))?;
                let minor = last_version
                    .minor
                    .ok_or(FlophaError::MissingVersionComponent("minor".into()))?;
                let patch = last_version
                    .patch
                    .ok_or(FlophaError::MissingVersionComponent("patch".into()))?
                    .saturating_add(1);
                (major, minor, patch)
            }
        };

        let tag = self
            .pattern
            .replace("{major}", &major.to_string())
            .replace("{minor}", &minor.to_string())
            .replace("{patch}", &patch.to_string());

        Ok(Some(Version::new(
            tag,
            Some(major),
            Some(minor),
            Some(patch),
        )))
    }

    fn get_regex(&self) -> Regex {
        // Replace placeholders with unique sentinels BEFORE escaping, so
        // regex::escape never touches the placeholder text.  The sentinels
        // use \x01 delimiters which are not regex metacharacters and will
        // survive escape unchanged.
        const SENTINELS: &[(&str, &str, &str)] = &[
            ("{major}", "\x01MAJOR\x01", "(?P<major>\\d+)"),
            ("{minor}", "\x01MINOR\x01", "(?P<minor>\\d+)"),
            ("{patch}", "\x01PATCH\x01", "(?P<patch>\\d+)"),
        ];

        let mut marked = self.pattern.clone();
        for (placeholder, sentinel, _) in SENTINELS {
            marked = marked.replace(placeholder, sentinel);
        }

        let mut expr = regex::escape(&marked);
        for (_, sentinel, group) in SENTINELS {
            expr = expr.replace(sentinel, group);
        }

        Regex::new(&format!("^{}$", expr))
            .unwrap_or_else(|e| panic!("invalid pattern {:?}: {}", self.pattern, e))
    }
}

fn parse_version(caps: &regex::Captures, name: &str) -> Option<u32> {
    caps.name(name).and_then(|v| v.as_str().parse::<u32>().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_last_version() {
        let tags = vec![
            "v1.0.0".to_string(),
            "v1.0.1".to_string(),
            "v1.0.2".to_string(),
            "v2.2.1".to_string(),
            "v1.1.0".to_string(),
            "v2.0.0".to_string(),
            "v2.1.0".to_string(),
            "v2.1.1".to_string(),
            "v2.1.2".to_string(),
            "v2.2.0".to_string(),
            "z4.0.0".to_string(),
        ];
        let versioner = Versioner::new(tags.clone(), "v{major}.{minor}.{patch}".to_string());
        let last_version = versioner.last_version();
        assert_eq!(
            last_version,
            Some(Version::new(
                "v2.2.1".to_string(),
                Some(2),
                Some(2),
                Some(1)
            ))
        );

        let versioner = Versioner::new(tags, "no-{major}.{minor}.{patch}".to_string());
        let last_version = versioner.last_version();
        assert_eq!(last_version, None);
    }

    #[test]
    fn test_scoped_last_version() {
        let tags = vec![
            "v1.0.0".to_string(),
            "v1.0.1".to_string(),
            "v1.0.2".to_string(),
            "v2.2.1".to_string(),
            "v1.1.0".to_string(),
            "v2.0.0".to_string(),
            "v2.1.0".to_string(),
            "v2.1.1".to_string(),
            "v2.1.2".to_string(),
            "v2.2.0".to_string(),
            "z4.0.0".to_string(),
        ];
        let versioner = Versioner::new(tags.clone(), "v1.{minor}.{patch}".to_string());
        let last_version = versioner.last_version();
        assert_eq!(
            last_version,
            Some(Version::new("v1.1.0".to_string(), None, Some(1), Some(0)))
        );

        let versioner = Versioner::new(tags, "v{major}.0.{patch}".to_string());
        let last_version = versioner.last_version();
        assert_eq!(
            last_version,
            Some(Version::new("v2.0.0".to_string(), Some(2), None, Some(0)))
        );
    }

    #[test]
    fn test_last_version_with_mixed_semantic_versioning_pattern() {
        let versioner = Versioner::new(
            vec![
                "v1.0.1".to_string(),
                "v2.0.1".to_string(),
                "v3.0.1".to_string(),
                "v1.2.2".to_string(),
                "v1.0.3".to_string(),
                "v2.2.1".to_string(),
                "v1.1.1".to_string(),
                "v2.1.1".to_string(),
                "v2.1.2".to_string(),
                "v2.2.0".to_string(),
                "z4.0.0".to_string(),
            ],
            "v{patch}.{minor}.{major}".to_string(),
        );
        let last_version = versioner.last_version();
        assert_eq!(
            last_version,
            Some(Version::new(
                "v1.0.3".to_string(),
                Some(3),
                Some(0),
                Some(1)
            ))
        );
    }

    #[test]
    fn test_next_version() {
        let tags = vec![
            "v1.0.0".to_string(),
            "v1.0.1".to_string(),
            "v1.0.2".to_string(),
            "v2.2.1".to_string(),
            "v1.1.0".to_string(),
            "v2.0.0".to_string(),
            "v2.1.0".to_string(),
            "v2.1.1".to_string(),
            "v2.1.2".to_string(),
            "v2.2.0".to_string(),
            "z4.0.0".to_string(),
        ];
        let versioner = Versioner::new(tags.clone(), "v{major}.{minor}.{patch}".to_string());
        let next_version = versioner.next_version(Increment::Major).unwrap();
        assert_eq!(
            next_version,
            Some(Version::new(
                "v3.0.0".to_string(),
                Some(3),
                Some(0),
                Some(0)
            ))
        );

        let next_version = versioner.next_version(Increment::Minor).unwrap();
        assert_eq!(
            next_version,
            Some(Version::new(
                "v2.3.0".to_string(),
                Some(2),
                Some(3),
                Some(0)
            ))
        );

        let next_version = versioner.next_version(Increment::Patch).unwrap();
        assert_eq!(
            next_version,
            Some(Version::new(
                "v2.2.2".to_string(),
                Some(2),
                Some(2),
                Some(2)
            ))
        );
    }

    #[test]
    fn test_next_version_returns_none_when_no_last_version() {
        let versioner = Versioner::new(
            vec!["v1.0.0".to_string(), "v1.0.1".to_string()],
            "no-{major}.{minor}.{patch}".to_string(),
        );
        let next_version = versioner.next_version(Increment::Major).unwrap();
        assert_eq!(next_version, None);
    }

    #[test]
    fn test_next_version_error_when_no_increment_in_pattern() {
        let versioner = Versioner::new(
            vec!["v1.0.0".to_string(), "v1.0.1".to_string()],
            "v1.{minor}.{patch}".to_string(),
        );
        assert!(versioner.next_version(Increment::Major).is_err());
    }

    // ── bump-rule / auto-detection tests ─────────────────────────────────────

    fn cc_rules() -> Vec<BumpRule> {
        conventional_bump_rules()
    }

    #[test]
    fn test_breaking_change_footer_is_major() {
        let msgs = vec!["fix: something\n\nBREAKING CHANGE: old API removed".to_string()];
        assert!(matches!(
            detect_increment(&msgs, &cc_rules()),
            Increment::Major
        ));
    }

    #[test]
    fn test_breaking_change_dash_is_major() {
        let msgs = vec!["fix: something\n\nBREAKING-CHANGE: old API removed".to_string()];
        assert!(matches!(
            detect_increment(&msgs, &cc_rules()),
            Increment::Major
        ));
    }

    #[test]
    fn test_bang_after_type_is_major() {
        let msgs = vec!["feat!: redesign everything".to_string()];
        assert!(matches!(
            detect_increment(&msgs, &cc_rules()),
            Increment::Major
        ));
    }

    #[test]
    fn test_bang_with_scope_is_major() {
        let msgs = vec!["feat(api)!: remove endpoint".to_string()];
        assert!(matches!(
            detect_increment(&msgs, &cc_rules()),
            Increment::Major
        ));
    }

    #[test]
    fn test_bang_without_colon_is_not_major() {
        // `feat!` with no trailing colon is not a valid CC breaking change
        let msgs = vec!["feat! redesign everything".to_string()];
        assert!(!matches!(
            detect_increment(&msgs, &cc_rules()),
            Increment::Major
        ));
    }

    #[test]
    fn test_feat_is_minor() {
        let msgs = vec![
            "fix: small bug".to_string(),
            "feat: add new command".to_string(),
        ];
        assert!(matches!(
            detect_increment(&msgs, &cc_rules()),
            Increment::Minor
        ));
    }

    #[test]
    fn test_feat_with_scope_is_minor() {
        let msgs = vec!["feat(cli): add --auto flag".to_string()];
        assert!(matches!(
            detect_increment(&msgs, &cc_rules()),
            Increment::Minor
        ));
    }

    #[test]
    fn test_fix_only_is_patch() {
        let msgs = vec!["fix: typo".to_string(), "chore: update deps".to_string()];
        assert!(matches!(
            detect_increment(&msgs, &cc_rules()),
            Increment::Patch
        ));
    }

    #[test]
    fn test_empty_messages_is_patch() {
        assert!(matches!(
            detect_increment(&[], &cc_rules()),
            Increment::Patch
        ));
    }

    #[test]
    fn test_custom_rules_override_defaults() {
        let rules = vec![
            BumpRule::new(r"^MAJOR:", Increment::Major).unwrap(),
            BumpRule::new(r"^MINOR:", Increment::Minor).unwrap(),
        ];
        // "feat:" would be minor under defaults but there's no matching rule here → patch
        let msgs = vec!["feat: something".to_string()];
        assert!(matches!(detect_increment(&msgs, &rules), Increment::Patch));

        let msgs = vec!["MINOR: add thing".to_string()];
        assert!(matches!(detect_increment(&msgs, &rules), Increment::Minor));

        let msgs = vec!["MAJOR: big change".to_string()];
        assert!(matches!(detect_increment(&msgs, &rules), Increment::Major));
    }

    #[test]
    fn test_custom_rules_major_short_circuits() {
        let rules = vec![
            BumpRule::new(r"breaking", Increment::Major).unwrap(),
            BumpRule::new(r"feature", Increment::Minor).unwrap(),
        ];
        // Both match; major should win and return immediately
        let msgs = vec!["breaking feature change".to_string()];
        assert!(matches!(detect_increment(&msgs, &rules), Increment::Major));
    }
}

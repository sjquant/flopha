use clap::ValueEnum;
use regex::Regex;

use crate::error::FlophaError;

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
        let regex = self.get_regex();
        let mut versions: Vec<Version> = Vec::new();
        for tag in self.tags.iter() {
            if let Some(caps) = regex.captures(tag) {
                let major = parse_version(&caps, "major");
                let minor = parse_version(&caps, "minor");
                let patch = parse_version(&caps, "patch");
                versions.push(Version::new(tag.to_string(), major, minor, patch));
            }
        }
        versions.sort_by(|a, b| {
            a.major
                .cmp(&b.major)
                .then(a.minor.cmp(&b.minor))
                .then(a.patch.cmp(&b.patch))
        });

        if !versions.is_empty() {
            versions.last().cloned()
        } else {
            None
        }
    }

    /// Returns all versions matching the pattern, sorted ascending (oldest first).
    pub fn all_versions(&self) -> Vec<Version> {
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
                    + 1;
                (major, 0, 0)
            }
            Increment::Minor => {
                let major = last_version
                    .major
                    .ok_or(FlophaError::MissingVersionComponent("major".into()))?;
                let minor = last_version
                    .minor
                    .ok_or(FlophaError::MissingVersionComponent("minor".into()))?
                    + 1;
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
                    + 1;
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
        let mut expr = regex::escape(&self.pattern)
            .replace("\\{major\\}", "{major}")
            .replace("\\{minor\\}", "{minor}")
            .replace("\\{patch\\}", "{patch}")
            .replace("{major}", "(?P<major>\\d+)")
            .replace("{minor}", "(?P<minor>\\d+)")
            .replace("{patch}", "(?P<patch>\\d+)");
        // Add ^ and $ to match the whole string
        expr = format!("^{}$", expr);
        Regex::new(&expr).unwrap()
    }
}

fn parse_version(caps: &regex::Captures, name: &str) -> Option<u32> {
    caps.name(name)
        .map(|version| version.as_str().parse::<u32>().unwrap())
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
}

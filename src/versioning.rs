use regex::Regex;

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
            if a.major > b.major {
                return std::cmp::Ordering::Greater;
            } else if a.major < b.major {
                return std::cmp::Ordering::Less;
            }
            if a.minor > b.minor {
                return std::cmp::Ordering::Greater;
            } else if a.minor < b.minor {
                return std::cmp::Ordering::Less;
            }
            if a.patch > b.patch {
                return std::cmp::Ordering::Greater;
            } else if a.patch < b.patch {
                return std::cmp::Ordering::Less;
            }
            std::cmp::Ordering::Equal
        });

        if versions.len() > 0 {
            let last_version = versions.last().unwrap().clone();
            Some(last_version)
        } else {
            None
        }
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
        let re = Regex::new(&expr).unwrap();
        re
    }
}

fn parse_version(caps: &regex::Captures, name: &str) -> Option<u32> {
    if let Some(version) = caps.name(name) {
        Some(version.as_str().parse::<u32>().unwrap())
    } else {
        None
    }
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
}

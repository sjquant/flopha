use regex::Regex;

pub struct Versioner {
    tags: Vec<String>,
    pattern: String,
}

impl Versioner {
    pub fn new(tags: Vec<String>, pattern: String) -> Self {
        Self { tags, pattern }
    }

    pub fn last_version(&self) -> Option<String> {
        let regex = self.get_regex();
        let mut versions: Vec<(String, u32, u32, u32)> = Vec::new();
        for tag in self.tags.iter() {
            if let Some(caps) = regex.captures(tag) {
                let major = parse_version(&caps, "major");
                let minor = parse_version(&caps, "minor");
                let patch = parse_version(&caps, "patch");
                versions.push((tag.to_string(), major, minor, patch));
            }
        }
        versions.sort_by(|a, b| {
            if a.1 > b.1 {
                return std::cmp::Ordering::Greater;
            } else if a.1 < b.1 {
                return std::cmp::Ordering::Less;
            }
            if a.2 > b.2 {
                return std::cmp::Ordering::Greater;
            } else if a.2 < b.2 {
                return std::cmp::Ordering::Less;
            }
            if a.3 > b.3 {
                return std::cmp::Ordering::Greater;
            } else if a.3 < b.3 {
                return std::cmp::Ordering::Less;
            }
            std::cmp::Ordering::Equal
        });

        if versions.len() > 0 {
            Some(versions.last().unwrap().0.to_string())
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

fn parse_version(caps: &regex::Captures, name: &str) -> u32 {
    let major = match caps.name(name) {
        Some(major) => major.as_str(),
        None => "0",
    }
    .parse::<u32>()
    .unwrap();
    major
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
        assert_eq!(last_version, Some("v2.2.1".to_string()));

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
        assert_eq!(last_version, Some("v1.1.0".to_string()));

        let versioner = Versioner::new(tags, "v{major}.0.{patch}".to_string());
        let last_version = versioner.last_version();
        assert_eq!(last_version, Some("v2.0.0".to_string()));
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
        assert_eq!(last_version, Some("v1.0.3".to_string()));
    }
}

use regex::Regex;

struct Versioner {
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
                let major = caps.name("major").unwrap().as_str().parse::<u32>().unwrap();
                let minor = caps.name("minor").unwrap().as_str().parse::<u32>().unwrap();
                let patch = caps.name("patch").unwrap().as_str().parse::<u32>().unwrap();
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
        let expr = regex::escape(&self.pattern)
            .replace("\\{major\\}", "{major}")
            .replace("\\{minor\\}", "{minor}")
            .replace("\\{patch\\}", "{patch}")
            .replace("{major}", "(?P<major>\\d+)")
            .replace("{minor}", "(?P<minor>\\d+)")
            .replace("{patch}", "(?P<patch>\\d+)");
        let re = Regex::new(&expr).unwrap();
        re
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_last_version() {
        let versioner = Versioner::new(
            vec![
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
            ],
            "v{major}.{minor}.{patch}".to_string(),
        );
        let last_version = versioner.last_version();
        assert_eq!(last_version, Some("v2.2.1".to_string()));
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

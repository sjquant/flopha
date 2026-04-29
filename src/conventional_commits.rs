use crate::versioning::Increment;

/// Detect the appropriate semver bump level by scanning conventional commit messages.
///
/// Rules (highest precedence first):
///   BREAKING CHANGE in footer, or `!` after type  → major
///   `feat` type                                    → minor
///   anything else                                  → patch
pub fn detect_increment(messages: &[String]) -> Increment {
    let mut increment = Increment::Patch;

    for message in messages {
        let first_line = message.lines().next().unwrap_or("").trim();

        // Breaking change in the commit body/footer
        if message.contains("BREAKING CHANGE:") || message.contains("BREAKING-CHANGE:") {
            return Increment::Major;
        }

        // Parse the conventional commit type from the first line
        if let Some(colon_pos) = first_line.find(':') {
            let type_scope = &first_line[..colon_pos];
            // Strip optional scope: `feat(api)!` → `feat!`
            let type_part = type_scope
                .find('(')
                .map(|i| {
                    let after_scope = type_scope.rfind(')').map(|j| &type_scope[j + 1..]).unwrap_or("");
                    format!("{}{}", &type_scope[..i], after_scope)
                })
                .unwrap_or_else(|| type_scope.to_string());

            if type_part.ends_with('!') {
                return Increment::Major;
            }

            let type_name = type_part.trim();
            if type_name == "feat" {
                increment = Increment::Minor;
            }
        }
    }

    increment
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_breaking_change_footer_is_major() {
        let msgs = vec!["fix: something\n\nBREAKING CHANGE: old API removed".to_string()];
        assert!(matches!(detect_increment(&msgs), Increment::Major));
    }

    #[test]
    fn test_bang_after_type_is_major() {
        let msgs = vec!["feat!: redesign everything".to_string()];
        assert!(matches!(detect_increment(&msgs), Increment::Major));
    }

    #[test]
    fn test_bang_with_scope_is_major() {
        let msgs = vec!["feat(api)!: remove endpoint".to_string()];
        assert!(matches!(detect_increment(&msgs), Increment::Major));
    }

    #[test]
    fn test_feat_is_minor() {
        let msgs = vec![
            "fix: small bug".to_string(),
            "feat: add new command".to_string(),
        ];
        assert!(matches!(detect_increment(&msgs), Increment::Minor));
    }

    #[test]
    fn test_fix_only_is_patch() {
        let msgs = vec!["fix: typo".to_string(), "chore: update deps".to_string()];
        assert!(matches!(detect_increment(&msgs), Increment::Patch));
    }

    #[test]
    fn test_empty_is_patch() {
        assert!(matches!(detect_increment(&[]), Increment::Patch));
    }
}

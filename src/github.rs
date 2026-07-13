use std::process::Command;

use git2::Repository;

use crate::error::FlophaError;
use crate::gitutils;

/// Resolves the `owner/repo` slug from a remote's URL, supporting the common
/// GitHub URL shapes (`https://github.com/owner/repo(.git)`, `git@github.com:owner/repo.git`,
/// `ssh://git@github.com/owner/repo.git`).
pub fn repo_slug_from_remote(repo: &Repository, remote_name: &str) -> Result<String, FlophaError> {
    let remote = gitutils::get_remote(repo, remote_name)?;
    let url = remote
        .url()
        .ok_or_else(|| FlophaError::Config(format!("remote '{}' has no URL", remote_name)))?;
    parse_github_slug(url).ok_or_else(|| {
        FlophaError::Config(format!(
            "could not determine a GitHub owner/repo from remote URL '{}'",
            url
        ))
    })
}

fn parse_github_slug(url: &str) -> Option<String> {
    let trimmed = url.trim_end_matches('/').trim_end_matches(".git");
    if let Some(rest) = trimmed.strip_prefix("git@github.com:") {
        return Some(rest.to_string());
    }
    let idx = trimmed.find("github.com/")?;
    let slug = &trimmed[idx + "github.com/".len()..];
    if slug.is_empty() {
        None
    } else {
        Some(slug.to_string())
    }
}

pub struct ReleaseRequest<'a> {
    pub repo_slug: &'a str,
    pub tag: &'a str,
    pub title: &'a str,
    pub body: Option<&'a str>,
    pub draft: bool,
    pub prerelease: bool,
    pub generate_notes: bool,
}

/// Creates a GitHub Release for an already-pushed tag by shelling out to the `gh`
/// CLI — the same mechanism `action/run.sh` already relies on, so auth (`GH_TOKEN`
/// / `GITHUB_TOKEN`) and permission requirements are unchanged for CI users.
pub fn create_release(req: &ReleaseRequest) -> Result<String, FlophaError> {
    let mut cmd = Command::new("gh");
    cmd.args([
        "release",
        "create",
        req.tag,
        "--repo",
        req.repo_slug,
        "--title",
        req.title,
    ]);
    if req.draft {
        cmd.arg("--draft");
    }
    if req.prerelease {
        cmd.arg("--prerelease");
    }
    match req.body {
        Some(body) => {
            cmd.args(["--notes", body]);
        }
        None if req.generate_notes => {
            cmd.arg("--generate-notes");
        }
        None => {
            cmd.args(["--notes", ""]);
        }
    }

    let output = cmd
        .output()
        .map_err(|e| FlophaError::CommandFailed(format!("failed to run 'gh': {}", e)))?;

    if !output.status.success() {
        return Err(FlophaError::CommandFailed(format!(
            "gh release create failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parses_https_url() {
        assert_eq!(
            parse_github_slug("https://github.com/sjquant/flopha.git"),
            Some("sjquant/flopha".to_string())
        );
    }

    #[test]
    fn test_parses_https_url_without_git_suffix() {
        assert_eq!(
            parse_github_slug("https://github.com/sjquant/flopha"),
            Some("sjquant/flopha".to_string())
        );
    }

    #[test]
    fn test_parses_ssh_shorthand_url() {
        assert_eq!(
            parse_github_slug("git@github.com:sjquant/flopha.git"),
            Some("sjquant/flopha".to_string())
        );
    }

    #[test]
    fn test_parses_ssh_url() {
        assert_eq!(
            parse_github_slug("ssh://git@github.com/sjquant/flopha.git"),
            Some("sjquant/flopha".to_string())
        );
    }

    #[test]
    fn test_non_github_remote_returns_none() {
        assert_eq!(
            parse_github_slug("https://gitlab.com/sjquant/flopha.git"),
            None
        );
    }
}

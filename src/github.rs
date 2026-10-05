use std::process::Command;
use std::time::Duration;

use git2::Repository;
use ureq::tls::{RootCerts, TlsConfig, TlsProvider};

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

const DEFAULT_API_URL: &str = "https://api.github.com";

/// Creates GitHub Releases through the REST API, so `release` works without
/// the GitHub CLI installed.
pub struct GitHubClient {
    agent: ureq::Agent,
    api_url: String,
    token: String,
}

impl GitHubClient {
    /// Uses `GITHUB_API_URL` when set (GitHub Actions sets it, including on
    /// GitHub Enterprise Server), otherwise api.github.com. Errors when no token
    /// is available, so callers can fail before pushing anything.
    pub fn from_env() -> Result<Self, FlophaError> {
        let api_url = std::env::var("GITHUB_API_URL")
            .ok()
            .filter(|url| !url.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_API_URL.to_string());
        let token = resolve_token().ok_or_else(|| {
            FlophaError::Config(
                "release.create = true needs a GitHub token: set GH_TOKEN or GITHUB_TOKEN, \
                 or log in with `gh auth login`"
                    .to_string(),
            )
        })?;
        Ok(Self::new(&api_url, token))
    }

    pub fn new(api_url: &str, token: String) -> Self {
        let config = ureq::Agent::config_builder()
            .tls_config(
                TlsConfig::builder()
                    .provider(TlsProvider::NativeTls)
                    .root_certs(RootCerts::PlatformVerifier)
                    .build(),
            )
            .http_status_as_error(false)
            .timeout_global(Some(Duration::from_secs(30)))
            .user_agent(concat!("flopha/", env!("CARGO_PKG_VERSION")))
            .build();
        Self {
            agent: config.into(),
            api_url: api_url.trim_end_matches('/').to_string(),
            token,
        }
    }

    /// Creates a Release for an already-pushed tag and returns its HTML URL.
    pub fn create_release(&self, req: &ReleaseRequest) -> Result<String, FlophaError> {
        let mut payload = serde_json::json!({
            "tag_name": req.tag,
            "name": req.title,
            "draft": req.draft,
            "prerelease": req.prerelease,
        });
        match req.body {
            Some(body) => payload["body"] = body.into(),
            None if req.generate_notes => payload["generate_release_notes"] = true.into(),
            None => {}
        }

        let url = format!("{}/repos/{}/releases", self.api_url, req.repo_slug);
        let mut response = self
            .agent
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .content_type("application/json")
            .send(payload.to_string())
            .map_err(|e| FlophaError::GitHub(format!("request to {} failed: {}", url, e)))?;

        let status = response.status();
        let text = response
            .body_mut()
            .read_to_string()
            .map_err(|e| FlophaError::GitHub(format!("failed to read response: {}", e)))?;
        let json: serde_json::Value = serde_json::from_str(&text).unwrap_or_default();

        if !status.is_success() {
            let mut message = json["message"].as_str().unwrap_or(text.trim()).to_string();
            // "Validation Failed" alone is unhelpful; the details (e.g. the tag's
            // release already exists) are in `errors`.
            if let Some(errors) = json.get("errors") {
                message = format!("{} {}", message, errors);
            }
            return Err(FlophaError::GitHub(format!(
                "creating release '{}' in {} failed ({}): {}",
                req.tag,
                req.repo_slug,
                status.as_u16(),
                message
            )));
        }

        json["html_url"]
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| FlophaError::GitHub("response did not include html_url".to_string()))
    }
}

fn resolve_token() -> Option<String> {
    ["GH_TOKEN", "GITHUB_TOKEN"]
        .iter()
        .filter_map(|var| std::env::var(var).ok())
        .map(|token| token.trim().to_string())
        .find(|token| !token.is_empty())
        .or_else(gh_auth_token)
}

/// Reuses the GitHub CLI's stored login for local runs, when `gh` is installed.
fn gh_auth_token() -> Option<String> {
    let output = Command::new("gh").args(["auth", "token"]).output().ok()?;
    let token = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (output.status.success() && !token.is_empty()).then_some(token)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;
    use std::thread;

    /// Serves one canned HTTP response on a local port and returns the base URL
    /// plus a handle yielding the raw request it received.
    fn serve_once(
        status_line: &'static str,
        body: &'static str,
    ) -> (String, thread::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut head = String::new();
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                if line == "\r\n" || line.is_empty() {
                    break;
                }
                head.push_str(&line);
            }
            let len = head
                .lines()
                .find_map(|l| {
                    l.to_ascii_lowercase()
                        .strip_prefix("content-length:")
                        .map(|v| v.trim().parse::<usize>().unwrap())
                })
                .unwrap_or(0);
            let mut request_body = vec![0; len];
            reader.read_exact(&mut request_body).unwrap();
            write!(
                stream,
                "HTTP/1.1 {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                status_line,
                body.len(),
                body
            )
            .unwrap();
            format!("{}\r\n{}", head, String::from_utf8(request_body).unwrap())
        });
        (url, handle)
    }

    fn release_request(body: Option<&str>) -> ReleaseRequest<'_> {
        ReleaseRequest {
            repo_slug: "sjquant/flopha",
            tag: "v1.1.0",
            title: "v1.1.0",
            body,
            draft: false,
            prerelease: true,
            generate_notes: true,
        }
    }

    /// It posts the release to the repo's releases endpoint with the token and returns its URL.
    #[test]
    fn test_create_release_posts_to_api_and_returns_html_url() {
        // Given an API that accepts the release
        let (url, request) = serve_once(
            "201 Created",
            r#"{"html_url":"https://github.com/sjquant/flopha/releases/tag/v1.1.0"}"#,
        );
        let client = GitHubClient::new(&url, "secret-token".to_string());

        // When creating a release with a changelog body
        let result = client.create_release(&release_request(Some("## Features")));

        // Then it returns the release URL and sent an authenticated POST with the release fields
        assert_eq!(
            result.unwrap(),
            "https://github.com/sjquant/flopha/releases/tag/v1.1.0"
        );
        let request = request.join().unwrap();
        assert!(request.starts_with("POST /repos/sjquant/flopha/releases HTTP/1.1"));
        assert!(request
            .to_ascii_lowercase()
            .contains("authorization: bearer secret-token"));
        let payload: serde_json::Value =
            serde_json::from_str(request.split("\r\n\r\n").last().unwrap()).unwrap();
        assert_eq!(payload["tag_name"], "v1.1.0");
        assert_eq!(payload["body"], "## Features");
        assert_eq!(payload["prerelease"], true);
        // An explicit body wins over GitHub's generated notes
        assert!(payload.get("generate_release_notes").is_none());
    }

    /// It surfaces GitHub's error message and details when the API rejects the release.
    #[test]
    fn test_create_release_reports_api_error_details() {
        // Given an API that rejects the release because one already exists for the tag
        let (url, _request) = serve_once(
            "422 Unprocessable Entity",
            r#"{"message":"Validation Failed","errors":[{"resource":"Release","code":"already_exists","field":"tag_name"}]}"#,
        );
        let client = GitHubClient::new(&url, "secret-token".to_string());

        // When creating the release
        let err = client.create_release(&release_request(None)).unwrap_err();

        // Then the error names the status, GitHub's message, and the validation details
        let message = err.to_string();
        assert!(message.contains("422"), "{message}");
        assert!(message.contains("Validation Failed"), "{message}");
        assert!(message.contains("already_exists"), "{message}");
    }

    /// It extracts owner/repo from an HTTPS URL with a `.git` suffix.
    #[test]
    fn test_parses_https_url() {
        // Given a standard HTTPS clone URL
        // When parsing it
        // Then the owner/repo slug is extracted
        assert_eq!(
            parse_github_slug("https://github.com/sjquant/flopha.git"),
            Some("sjquant/flopha".to_string())
        );
    }

    /// It extracts owner/repo from an HTTPS URL without a `.git` suffix.
    #[test]
    fn test_parses_https_url_without_git_suffix() {
        // Given an HTTPS URL with no .git suffix
        // When parsing it
        // Then the owner/repo slug is extracted
        assert_eq!(
            parse_github_slug("https://github.com/sjquant/flopha"),
            Some("sjquant/flopha".to_string())
        );
    }

    /// It extracts owner/repo from the `git@github.com:owner/repo.git` shorthand.
    #[test]
    fn test_parses_ssh_shorthand_url() {
        // Given an SSH shorthand URL
        // When parsing it
        // Then the owner/repo slug is extracted
        assert_eq!(
            parse_github_slug("git@github.com:sjquant/flopha.git"),
            Some("sjquant/flopha".to_string())
        );
    }

    /// It extracts owner/repo from a full `ssh://` URL.
    #[test]
    fn test_parses_ssh_url() {
        // Given a full ssh:// URL
        // When parsing it
        // Then the owner/repo slug is extracted
        assert_eq!(
            parse_github_slug("ssh://git@github.com/sjquant/flopha.git"),
            Some("sjquant/flopha".to_string())
        );
    }

    /// It returns `None` for remotes that aren't hosted on github.com.
    #[test]
    fn test_non_github_remote_returns_none() {
        // Given a non-GitHub remote URL
        // When parsing it
        // Then no slug is extracted
        assert_eq!(
            parse_github_slug("https://gitlab.com/sjquant/flopha.git"),
            None
        );
    }
}

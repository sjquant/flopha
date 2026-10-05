use std::process::Command;
use std::time::Duration;

use git2::Repository;
use ureq::http::StatusCode;
use ureq::tls::{RootCerts, TlsConfig, TlsProvider};

use crate::error::FlophaError;
use crate::gitutils;

/// A repository on GitHub or GitHub Enterprise Server, as identified by a remote URL.
pub struct RemoteRepo {
    pub host: String,
    pub slug: String,
}

/// Resolves the host and `owner/repo` slug from a remote's URL, supporting HTTPS
/// (`https://host/owner/repo(.git)`), SCP-style SSH (`git@host:owner/repo.git`),
/// and `ssh://` URLs.
pub fn remote_repo(repo: &Repository, remote_name: &str) -> Result<RemoteRepo, FlophaError> {
    let remote = gitutils::get_remote(repo, remote_name)?;
    let url = remote
        .url()
        .ok_or_else(|| FlophaError::Config(format!("remote '{}' has no URL", remote_name)))?;
    parse_remote_url(url).ok_or_else(|| {
        FlophaError::Config(format!(
            "could not determine a GitHub owner/repo from remote URL '{}'",
            url
        ))
    })
}

fn parse_remote_url(url: &str) -> Option<RemoteRepo> {
    let (authority, path) = match url.split_once("://") {
        Some((_scheme, rest)) => rest.split_once('/')?,
        None => url.split_once(':')?,
    };
    let host = authority.rsplit('@').next()?;
    let host = host.split(':').next()?;
    let slug = path.trim_end_matches('/').trim_end_matches(".git");
    let mut segments = slug.split('/');
    let valid = matches!(
        (segments.next(), segments.next(), segments.next()),
        (Some(owner), Some(name), None) if !owner.is_empty() && !name.is_empty()
    );
    (valid && !host.is_empty()).then(|| RemoteRepo {
        host: host.to_string(),
        slug: slug.to_string(),
    })
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

/// Creates GitHub Releases through the REST API, so `release` works without
/// the GitHub CLI installed.
pub struct GitHubClient {
    agent: ureq::Agent,
    api_url: String,
    token: String,
}

impl GitHubClient {
    /// Uses `GITHUB_API_URL` when set (GitHub Actions sets it, including on
    /// GitHub Enterprise Server), otherwise the API of `host`. Errors when no
    /// token is available, so callers can fail before pushing anything.
    pub fn from_env(host: &str) -> Result<Self, FlophaError> {
        let api_url = std::env::var("GITHUB_API_URL")
            .ok()
            .filter(|url| !url.trim().is_empty())
            .unwrap_or_else(|| match host {
                "github.com" => "https://api.github.com".to_string(),
                _ => format!("https://{}/api/v3", host),
            });
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

    /// Confirms the token can reach `repo_slug`, so a wrong host, repo, or token
    /// is caught before anything is pushed.
    pub fn check_access(&self, repo_slug: &str) -> Result<(), FlophaError> {
        let (status, json) = self.get(&format!("/repos/{}", repo_slug))?;
        if !status.is_success() {
            return Err(api_error(
                &format!("accessing {}", repo_slug),
                status,
                &json,
            ));
        }
        Ok(())
    }

    /// Returns the HTML URL of the Release (draft or published) for `tag`, if any.
    pub fn find_release(&self, repo_slug: &str, tag: &str) -> Result<Option<String>, FlophaError> {
        let path = format!("/repos/{}/releases?per_page=100", repo_slug);
        let (status, json) = self.get(&path)?;
        if !status.is_success() {
            return Err(api_error(
                &format!("listing releases of {}", repo_slug),
                status,
                &json,
            ));
        }
        Ok(json
            .as_array()
            .into_iter()
            .flatten()
            .find(|release| release["tag_name"] == tag)
            .and_then(|release| release["html_url"].as_str())
            .map(str::to_string))
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

        let path = format!("/repos/{}/releases", req.repo_slug);
        let (status, json) = self.post(&path, payload.to_string())?;
        if !status.is_success() {
            let action = format!("creating release '{}' in {}", req.tag, req.repo_slug);
            return Err(api_error(&action, status, &json));
        }
        json["html_url"]
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| FlophaError::GitHub("response did not include html_url".to_string()))
    }

    fn get(&self, path: &str) -> Result<(StatusCode, serde_json::Value), FlophaError> {
        let url = format!("{}{}", self.api_url, path);
        let result = self
            .agent
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .call();
        read_response(result, &url)
    }

    fn post(
        &self,
        path: &str,
        body: String,
    ) -> Result<(StatusCode, serde_json::Value), FlophaError> {
        let url = format!("{}{}", self.api_url, path);
        let result = self
            .agent
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .content_type("application/json")
            .send(body);
        read_response(result, &url)
    }
}

fn read_response(
    result: Result<ureq::http::Response<ureq::Body>, ureq::Error>,
    url: &str,
) -> Result<(StatusCode, serde_json::Value), FlophaError> {
    let mut response =
        result.map_err(|e| FlophaError::GitHub(format!("request to {} failed: {}", url, e)))?;
    let status = response.status();
    let text = response
        .body_mut()
        .read_to_string()
        .map_err(|e| FlophaError::GitHub(format!("failed to read response: {}", e)))?;
    Ok((status, serde_json::from_str(&text).unwrap_or_default()))
}

fn api_error(action: &str, status: StatusCode, json: &serde_json::Value) -> FlophaError {
    let mut message = json["message"].as_str().unwrap_or("").to_string();
    // "Validation Failed" alone is unhelpful; the details (e.g. the tag's release
    // already exists) are in `errors`.
    if let Some(errors) = json.get("errors") {
        message = format!("{} {}", message, errors);
    }
    FlophaError::GitHub(format!(
        "{} failed ({}): {}",
        action,
        status.as_u16(),
        message.trim()
    ))
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

    /// It finds an existing Release (including drafts) by its tag.
    #[test]
    fn test_find_release_matches_tag_name() {
        // Given an API listing a draft release for v1.1.0
        let (url, _request) = serve_once(
            "200 OK",
            r#"[{"tag_name":"v1.0.0","html_url":"https://example.com/v1.0.0"},{"tag_name":"v1.1.0","draft":true,"html_url":"https://example.com/v1.1.0"}]"#,
        );
        let client = GitHubClient::new(&url, "secret-token".to_string());

        // When looking up the release for v1.1.0
        let found = client.find_release("sjquant/flopha", "v1.1.0").unwrap();

        // Then the matching release's URL is returned
        assert_eq!(found.as_deref(), Some("https://example.com/v1.1.0"));
    }

    fn parsed(url: &str) -> Option<(String, String)> {
        parse_remote_url(url).map(|r| (r.host, r.slug))
    }

    fn pair(host: &str, slug: &str) -> Option<(String, String)> {
        Some((host.to_string(), slug.to_string()))
    }

    /// It extracts host and owner/repo from HTTPS URLs, with or without `.git`.
    #[test]
    fn test_parses_https_urls() {
        // Given HTTPS clone URLs
        // When parsing them
        // Then the host and owner/repo slug are extracted
        assert_eq!(
            parsed("https://github.com/sjquant/flopha.git"),
            pair("github.com", "sjquant/flopha")
        );
        assert_eq!(
            parsed("https://x-access-token:abc@github.com/sjquant/flopha"),
            pair("github.com", "sjquant/flopha")
        );
    }

    /// It extracts host and owner/repo from SCP-style and `ssh://` URLs.
    #[test]
    fn test_parses_ssh_urls() {
        // Given SSH remote URLs
        // When parsing them
        // Then the host and owner/repo slug are extracted
        assert_eq!(
            parsed("git@github.com:sjquant/flopha.git"),
            pair("github.com", "sjquant/flopha")
        );
        assert_eq!(
            parsed("ssh://git@github.com:22/sjquant/flopha.git"),
            pair("github.com", "sjquant/flopha")
        );
    }

    /// It keeps the host of GitHub Enterprise Server remotes so the right API is used.
    #[test]
    fn test_parses_enterprise_host() {
        // Given a GitHub Enterprise Server remote
        // When parsing it
        // Then the enterprise host is kept
        assert_eq!(
            parsed("https://github.example.com/team/app.git"),
            pair("github.example.com", "team/app")
        );
    }

    /// It rejects URLs that don't name exactly one owner and repo on a host.
    #[test]
    fn test_rejects_urls_without_owner_and_repo() {
        // Given a local path remote and a nested-group remote
        // When parsing them
        // Then no repository is extracted
        assert_eq!(parsed("file:///tmp/remote"), None);
        assert_eq!(parsed("https://gitlab.com/group/sub/app.git"), None);
    }
}

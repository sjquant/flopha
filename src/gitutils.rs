use std::io::Write;
use std::path::Path;

use git2::{Branch, DescribeFormatOptions, DescribeOptions, Repository};

use crate::error::FlophaError;

pub fn get_repo(path: &Path) -> Result<Repository, FlophaError> {
    Repository::open(path).map_err(|e| FlophaError::RepoNotFound {
        path: path.display().to_string(),
        source: e,
    })
}

pub fn get_remote<'a>(repo: &'a Repository, name: &str) -> Result<git2::Remote<'a>, FlophaError> {
    repo.find_remote(name).map_err(|e| FlophaError::RemoteNotFound {
        name: name.to_string(),
        source: e,
    })
}

pub fn tag_oid(repo: &Repository, id: git2::Oid, tagname: &str) -> Result<git2::Oid, git2::Error> {
    let obj = repo.find_object(id, None)?;
    repo.tag_lightweight(tagname, &obj, true)
}

pub fn commit(repo: &Repository, message: &str) -> Result<git2::Oid, git2::Error> {
    let mut index = repo.index()?;
    let id = index.write_tree()?;
    let tree = repo.find_tree(id)?;
    let sig = repo.signature()?;
    let head = repo.head();
    let parents = if let Ok(head) = head {
        vec![head.peel_to_commit()?]
    } else {
        vec![]
    };
    repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        message,
        &tree,
        &parents.iter().collect::<Vec<_>>(),
    )
}

pub fn checkout_branch(
    repo: &Repository,
    name: &str,
    force: bool,
) -> Result<(), git2::Error> {
    let branch = repo.find_branch(name, git2::BranchType::Local);
    if force && branch.is_err() {
        let commit = repo.head()?.peel_to_commit()?;
        repo.branch(name, &commit, true)?;
    } else {
        branch?;
    }
    let (object, reference) = repo.revparse_ext(name)?;
    repo.checkout_tree(&object, None)?;
    let reference =
        reference.ok_or_else(|| git2::Error::from_str("symbolic reference not found"))?;
    let ref_name = reference
        .name()
        .ok_or_else(|| git2::Error::from_str("branch name is not valid UTF-8"))?;
    repo.set_head(ref_name)?;
    log::debug!("Switched to branch '{}'", name);
    Ok(())
}

pub fn checkout_tag(repo: &Repository, tag: &str) -> Result<(), git2::Error> {
    let (object, reference) = repo.revparse_ext(tag)?;
    repo.checkout_tree(&object, None)?;
    let reference =
        reference.ok_or_else(|| git2::Error::from_str("symbolic reference not found"))?;
    let ref_name = reference
        .name()
        .ok_or_else(|| git2::Error::from_str("tag name is not valid UTF-8"))?;
    repo.set_head(ref_name)?;
    log::debug!("Switched to tag '{}'", tag);
    Ok(())
}

pub fn get_head_branch(repo: &Repository) -> Result<Branch<'_>, git2::Error> {
    Ok(Branch::wrap(repo.head()?))
}

pub fn get_last_tag_name(repo: &Repository) -> Result<String, git2::Error> {
    let describe = repo.describe(DescribeOptions::new().describe_tags())?;
    describe.format(Some(DescribeFormatOptions::new().abbreviated_size(0)))
}

pub fn fetch_all(remote: &mut git2::Remote) -> Result<(), git2::Error> {
    log::debug!("Fetching all branches and tags from remote...");
    let mut fo = fetch_options();
    remote.fetch(&["refs/heads/*:refs/heads/*"], Some(&mut fo), None)?;
    log::debug!("Successfully fetched all branches and tags from remote.");
    Ok(())
}

fn fetch_options() -> git2::FetchOptions<'static> {
    let mut fo = git2::FetchOptions::new();
    fo.download_tags(git2::AutotagOption::All);
    let cb = git_callbacks();
    fo.remote_callbacks(cb);
    fo
}

pub fn push_tag(remote: &mut git2::Remote, tag: &str) -> Result<(), git2::Error> {
    log::debug!("Pushing tag '{}' to remote...", tag);
    let mut po = push_options();
    let ref_spec = format!("refs/tags/{}:refs/tags/{}", tag, tag);
    remote.push(&[&ref_spec], Some(&mut po))?;
    log::debug!("Successfully pushed tag '{}' to remote.", tag);
    Ok(())
}

pub fn push_branch(
    remote: &mut git2::Remote,
    branch: &mut Branch,
) -> Result<(), git2::Error> {
    let branch_name = branch
        .name()?
        .ok_or_else(|| git2::Error::from_str("branch name is not valid UTF-8"))?
        .to_string();
    let remote_name = remote
        .name()
        .ok_or_else(|| git2::Error::from_str("remote name is not valid UTF-8"))?;
    let upstream_name = format!("{}/{}", remote_name, branch_name.as_str());
    log::debug!("Pushing branch '{}' to remote...", branch_name);
    let mut po = push_options();
    let refspec = format!("refs/heads/{}:refs/heads/{}", branch_name, branch_name);
    remote.push(&[&refspec], Some(&mut po))?;
    branch.set_upstream(Some(&upstream_name))?;
    log::debug!("Successfully pushed branch '{}' to remote.", branch_name);
    Ok(())
}

fn push_options() -> git2::PushOptions<'static> {
    let mut po = git2::PushOptions::new();
    let cb = git_callbacks();
    po.remote_callbacks(cb);
    po
}

// Shelling out to `git credential fill` rather than using git2::Cred::credential_helper()
// because libgit2 cannot reliably locate system credential helpers (e.g. osxkeychain on
// macOS is installed outside PATH by Homebrew/Xcode). The git CLI knows exactly where to
// find them. flopha already requires git to be installed, so this adds no new dependency.
fn git_credential_fill(url: &str) -> Option<(String, String)> {
    let (protocol, rest) = url.split_once("://")?;
    let host = rest.split('/').next()?;
    let host = host.split('@').next_back()?;
    let input = format!("protocol={}\nhost={}\n\n", protocol, host);

    let mut child = std::process::Command::new("git")
        .args(["credential", "fill"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .ok()?;

    child.stdin.take()?.write_all(input.as_bytes()).ok()?;

    let output = child.wait_with_output().ok()?;
    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8(output.stdout).ok()?;
    let mut username = None;
    let mut password = None;
    for line in stdout.lines() {
        if let Some(val) = line.strip_prefix("username=") {
            username = Some(val.to_string());
        } else if let Some(val) = line.strip_prefix("password=") {
            password = Some(val.to_string());
        }
    }
    Some((username?, password?))
}

fn git_callbacks() -> git2::RemoteCallbacks<'static> {
    let git_config = git2::Config::open_default().ok();
    let mut cb = git2::RemoteCallbacks::new();
    cb.credentials(move |url, username, allowed| {
        let mut cred_helper = git2::CredentialHelper::new(url);
        if let Some(ref cfg) = git_config {
            cred_helper.config(cfg);
        }
        if allowed.is_ssh_key() {
            let user = username
                .map(std::string::ToString::to_string)
                .or_else(|| cred_helper.username.clone())
                .unwrap_or_else(|| "git".to_string());
            git2::Cred::ssh_key_from_agent(&user)
        } else if allowed.is_user_pass_plaintext() {
            if let (Ok(user), Ok(pass)) =
                (std::env::var("GIT_USERNAME"), std::env::var("GIT_PASSWORD"))
            {
                return git2::Cred::userpass_plaintext(&user, &pass);
            }
            if let Ok(token) = std::env::var("GITHUB_TOKEN") {
                return git2::Cred::userpass_plaintext("x-access-token", &token);
            }
            // libgit2's credential_helper doesn't invoke osxkeychain (and other
            // system helpers) correctly. Shell out to `git credential fill` instead.
            if let Some((user, pass)) = git_credential_fill(url) {
                return git2::Cred::userpass_plaintext(&user, &pass);
            }
            if let Some(ref cfg) = git_config {
                git2::Cred::credential_helper(cfg, url, username)
            } else {
                Err(git2::Error::from_str("no git config available for credential helper"))
            }
        } else if allowed.is_default() {
            git2::Cred::default()
        } else {
            Err(git2::Error::from_str(
                "Remote authentication is required but none available",
            ))
        }
    });
    cb
}

pub fn create_branch(repo: &Repository, name: &str, force: bool) -> Result<(), git2::Error> {
    let commit = repo.head()?.peel_to_commit()?;
    repo.branch(name, &commit, force)?;
    log::debug!("Created branch '{}'", name);
    Ok(())
}

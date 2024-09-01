use std::path::Path;

use git2::{Branch, DescribeFormatOptions, DescribeOptions, Repository};

#[derive(Default, Clone)]
pub struct CommandOptions {
    pub verbose: bool,
}

fn print_verbose(message: &str, verbose: bool) {
    if verbose {
        println!("{}", message);
    }
}

pub fn get_repo(path: &Path) -> Repository {
    let repo = Repository::open(path).expect("Repository not found");
    repo
}

pub fn get_remote<'a>(repo: &'a Repository, name: &str) -> git2::Remote<'a> {
    let remote = repo.find_remote(name).expect("Remote 'origin' not found");
    remote
}

pub fn tag_oid(repo: &Repository, id: git2::Oid, tagname: &str) -> Result<git2::Oid, git2::Error> {
    let obj = repo.find_object(id, None).unwrap();
    repo.tag_lightweight(tagname, &obj, true)
}

pub fn commit(repo: &Repository, message: &str) -> Result<git2::Oid, git2::Error> {
    let mut index = repo.index().unwrap();
    let id = index.write_tree().unwrap();
    let tree = repo.find_tree(id).unwrap();
    let sig = repo.signature().unwrap();
    let head = repo.head();
    let parents = if let Ok(head) = head {
        vec![head.peel_to_commit().unwrap()]
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
    options: Option<&CommandOptions>,
) -> Result<(), git2::Error> {
    let default = CommandOptions::default();
    let opts = options.unwrap_or(&default);
    let branch = repo.find_branch(name, git2::BranchType::Local);
    if force && branch.is_err() {
        let commit = repo.head().unwrap().peel_to_commit().unwrap();
        repo.branch(name, &commit, true).unwrap();
    } else {
        branch?;
    }
    let (object, reference) = repo.revparse_ext(name).expect("Branch not found");
    repo.checkout_tree(&object, None)
        .expect("Failed to checkout");
    repo.set_head(reference.unwrap().name().unwrap())?;
    print_verbose(&format!("Switched to branch '{}'", name), opts.verbose);
    Result::Ok(())
}

pub fn checkout_tag(
    repo: &Repository,
    tag: &str,
    options: Option<&CommandOptions>,
) -> Result<(), git2::Error> {
    let default = CommandOptions::default();
    let opts = options.unwrap_or(&default);
    let (object, reference) = repo.revparse_ext(tag).expect("Tag not found");
    repo.checkout_tree(&object, None)
        .expect("Failed to checkout");
    repo.set_head(reference.unwrap().name().unwrap())?;
    print_verbose(&format!("Switched to tag '{}'", tag), opts.verbose);
    Result::Ok(())
}

pub fn get_head_branch(repo: &Repository) -> Result<Branch, git2::Error> {
    Ok(Branch::wrap(repo.head()?))
}

pub fn get_last_tag_name(repo: &Repository) -> Result<String, git2::Error> {
    let describe = repo.describe(DescribeOptions::new().describe_tags())?;
    Ok(describe.format(Some(DescribeFormatOptions::new().abbreviated_size(0)))?)
}

pub fn fetch_all(
    remote: &mut git2::Remote,
    options: Option<&CommandOptions>,
) -> Result<(), git2::Error> {
    let default = CommandOptions::default();
    let opts = options.unwrap_or(&default);
    print_verbose(
        &format!("Fetching all branches and tags from remote..."),
        opts.verbose,
    );
    let mut fo = fetch_options();
    remote.fetch(&["refs/heads/*:refs/heads/*"], Some(&mut fo), None)?;
    print_verbose(
        &format!("Successfully fetched all branches and tags from remote."),
        opts.verbose,
    );
    Result::Ok(())
}

fn fetch_options() -> git2::FetchOptions<'static> {
    let mut fo = git2::FetchOptions::new();
    fo.download_tags(git2::AutotagOption::All);
    let cb = git_callbacks();
    fo.remote_callbacks(cb);
    fo
}

// Update the push_tag function to accept options
pub fn push_tag(
    remote: &mut git2::Remote,
    tag: &str,
    options: Option<&CommandOptions>,
) -> Result<(), git2::Error> {
    let default = CommandOptions::default();
    let opts = options.unwrap_or(&default);
    print_verbose(&format!("Pushing tag '{}' to remote...", tag), opts.verbose);
    let mut po = push_options();
    let ref_spec = format!("refs/tags/{}:refs/tags/{}", tag, tag);
    remote.push(&[&ref_spec], Some(&mut po))?;
    print_verbose(
        &format!("Successfully pushed tag '{}' to remote.", tag),
        opts.verbose,
    );
    Result::Ok(())
}

pub fn push_branch(
    remote: &mut git2::Remote,
    branch: &mut Branch,
    options: Option<&CommandOptions>,
) -> Result<(), git2::Error> {
    let default = CommandOptions::default();
    let opts = options.unwrap_or(&default);
    let branch_name = branch
        .name()
        .unwrap()
        .expect("Failed to get branch name")
        .to_string();
    let remote_name = remote.name().unwrap();
    let upstream_name = format!("{}/{}", remote_name, branch_name.as_str());
    print_verbose(
        &format!("Pushing branch '{}' to remote...", branch_name),
        opts.verbose,
    );
    let mut po = push_options();
    let refspec = format!("refs/heads/{}:refs/heads/{}", branch_name, branch_name);
    remote.push(&[&refspec], Some(&mut po))?;
    branch.set_upstream(Some(&upstream_name))?;
    print_verbose(
        &format!("Successfully pushed branch '{}' to remote.", branch_name),
        opts.verbose,
    );
    Result::Ok(())
}

fn push_options() -> git2::PushOptions<'static> {
    let mut po = git2::PushOptions::new();
    let cb = git_callbacks();
    po.remote_callbacks(cb);
    po
}

fn git_callbacks() -> git2::RemoteCallbacks<'static> {
    let git_config = git2::Config::open_default().unwrap();
    let mut cb = git2::RemoteCallbacks::new();
    cb.credentials(move |url, username, allowed| {
        let mut cred_helper = git2::CredentialHelper::new(url);
        cred_helper.config(&git_config);
        if allowed.is_ssh_key() {
            let user = username
                .map(std::string::ToString::to_string)
                .or_else(|| cred_helper.username.clone())
                .unwrap_or_else(|| "git".to_string());
            git2::Cred::ssh_key_from_agent(&user)
        } else if allowed.is_user_pass_plaintext() {
            git2::Cred::credential_helper(&git_config, url, username)
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

use git2::{Repository, Branch, DescribeOptions, DescribeFormatOptions};


pub fn tag_oid(repo: &Repository, id: git2::Oid, tagname: &str, force: bool) -> Result<git2::Oid, git2::Error> {
        let obj = repo.find_object(id, None).unwrap();
        repo.tag_lightweight(tagname, &obj, force)
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
    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents.iter().collect::<Vec<_>>())
}

pub fn checkout_branch(repo: &Repository, name: &str, force: bool) -> Result<(), git2::Error> {
    let branch = repo.find_branch(name, git2::BranchType::Local);
    if force && branch.is_err() {
        let commit = repo.head().unwrap().peel_to_commit().unwrap();
        repo.branch(name, &commit, true).unwrap();
    } else {
        branch?;
    }
    let (object, reference) = repo.revparse_ext(name).expect("Branch not found");
    repo.checkout_tree(&object, None).expect("Failed to checkout");
    repo.set_head(reference.unwrap().name().unwrap())?;
    Result::Ok(())
}

pub fn checkout_tag(repo: &Repository, tag: &str) -> Result<(), git2::Error> {
    let (object, reference) = repo.revparse_ext(tag).expect("Tag not found");
    repo.checkout_tree(&object, None).expect("Failed to checkout");
    repo.set_head(reference.unwrap().name().unwrap())?;
    Result::Ok(())
}


pub fn get_head_branch(repo: &Repository) -> Result<Branch, git2::Error> {
    Ok(Branch::wrap(repo.head()?))
}

pub fn get_last_tag_name(repo: &Repository) -> Result<String, git2::Error> {
    let describe = repo.describe(DescribeOptions::new().describe_tags())?;
    Ok(describe.format(Some(DescribeFormatOptions::new().abbreviated_size(0)))?)
}

pub fn fetch_all(remote: &mut git2::Remote) -> Result<(), git2::Error>{
    let mut fo = git2::FetchOptions::new();
    fo.download_tags(git2::AutotagOption::All);
    let cb = git_callbacks();
    fo.remote_callbacks(cb);
    remote.fetch(&["refs/heads/*:refs/heads/*"],  Some(&mut fo), None)?;
    Result::Ok(())
}

pub fn push_tag(remote: &mut git2::Remote, tag: &str) -> Result<(), git2::Error>{
    let mut po = git2::PushOptions::new();
    let cb = git_callbacks();
    po.remote_callbacks(cb);
    let ref_spec = format!("refs/tags/{}:refs/tags/{}", tag, tag);
    remote.push(&[&ref_spec], Some(&mut po))?;
    Result::Ok(())
}


pub fn push_branch(remote: &mut git2::Remote, branch_name: &str) -> Result<(), git2::Error>{
    let mut po = git2::PushOptions::new();
    let cb = git_callbacks();
    po.remote_callbacks(cb);
    let refspec = format!("refs/heads/{}:refs/heads/{}", branch_name, branch_name);
    remote.push(&[&refspec], Some(&mut po))?;
    Result::Ok(())
}


// I almost brought this from https://github.com/AchetaGames/Epic-Asset-Manager/blob/c8377966d984d34786f5baeddb0c6ca33e38c5bd/src/models/engine_data.rs#L427
fn git_callbacks() -> git2::RemoteCallbacks<'static> {
    let git_config = git2::Config::open_default().unwrap();
    let mut cb = git2::RemoteCallbacks::new();
    cb.credentials(move |url, username, allowed| {
        let mut cred_helper = git2::CredentialHelper::new(url);
        cred_helper.config(&git_config);
        if allowed.is_user_pass_plaintext() {
            git2::Cred::credential_helper(&git_config, url, username)
        } else if allowed.is_default() {
            git2::Cred::default()
        } else {
            Err(git2::Error::from_str("no authentication available"))
        }
    });
    cb
}
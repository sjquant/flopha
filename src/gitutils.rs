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
    remote.fetch(&["refs/heads/*:refs/heads/*"],  Some(&mut fo), None)?;
    Result::Ok(())
}
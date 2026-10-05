use std::path::{Path, PathBuf};

use crate::changelog::{build_changelog, ChangelogRequest};
use crate::cli::{OutputFormat, ReleaseArgs, VersionSourceName};
use crate::config::FlophaConfig;
use crate::error::FlophaError;
use crate::github::{self, GitHubClient, ReleaseRequest};
use crate::gitutils;
use crate::manifest;
use crate::version_source::{TagVersionSource, VersionSource};
use crate::versioning::{self, Version, Versioner};

/// Runs the full config-driven release pipeline described by `flopha.toml`:
/// compute bump -> sync manifests -> commit -> annotated tag -> push ->
/// changelog -> GitHub Release. Returns the tag that was created (or would be,
/// on `--dry-run`).
pub fn release(path: &Path, args: &ReleaseArgs) -> Result<Option<String>, FlophaError> {
    let config = FlophaConfig::load(&path.join(&args.config))?;

    let repo = gitutils::get_repo(path)?;
    gitutils::try_fetch_origin_tracking(&repo);

    let versioner = super::versioner_factory(
        &repo,
        config.version.pattern.clone(),
        &VersionSourceName::Tag,
    );
    let last = versioner.last_version();

    let increment = super::resolve_increment(
        &repo,
        last.as_ref(),
        config.version.auto,
        &config.version.rules,
        config.version.increment.clone(),
    )?;

    let next = versioner.next_version(increment)?.ok_or_else(|| {
        FlophaError::Config(
            "no version tag matches version.pattern; nothing to bump from".to_string(),
        )
    })?;
    let version_core = next.core()?;

    if let Some(tagged) = release_at_head(&repo, &config, &versioner, &next, &version_core)? {
        return finish_release(&repo, &config, args, tagged);
    }
    if let Some(last) = &last {
        if !gitutils::has_commits_since_tag(&repo, &last.tag)? {
            print_nothing_to_release(&args.format, &format!("no commits since {}", last.tag));
            return Ok(None);
        }
    }

    let from_tag = last.map(|v| v.tag);
    let (tag, version) = release_tag_and_version(&repo, &config, &next, version_core);

    if !config.manifests.is_empty() && !args.dry_run {
        check_can_commit(&repo)?;
    }

    let changelog = build_release_changelog(&repo, &config, from_tag.as_deref(), &tag)?;

    if args.dry_run {
        print_plan(&args.format, &from_tag, &tag, &config, &changelog);
        return Ok(Some(tag));
    }

    let github = connect_github(&repo, &config)?;
    let updates = manifest_updates(path, &config, &version)?;

    let original_head = repo.head()?.peel_to_commit()?;
    commit_and_tag(&repo, path, &config, &updates, &tag, &version)?;
    push_release(&repo, &tag, !updates.is_empty(), &original_head)?;

    println!("Released {}", tag);
    if let Some((client, repo_slug)) = &github {
        let url = create_github_release(client, repo_slug, &config, &tag, &version, &changelog)
            .map_err(|e| release_failed(&tag, e))?;
        println!("GitHub Release: {}", url);
    }

    Ok(Some(tag))
}

/// Finds the release tag at HEAD: the latest stable tag, or with `version.pre`
/// set, a `-{channel}.{n}` tag of the upcoming version (which the stable
/// pattern alone doesn't match).
fn release_at_head(
    repo: &git2::Repository,
    config: &FlophaConfig,
    versioner: &Versioner,
    next: &Version,
    version_core: &str,
) -> Result<Option<TaggedRelease>, FlophaError> {
    let head = repo.head()?.peel_to_commit()?.id();
    let at_head = |tag: &str| gitutils::tag_commit_oid(repo, tag).ok() == Some(head);
    let versions = versioner.all_versions();

    if let Some(channel) = &config.version.pre {
        let tags = repo.tag_names(None)?;
        return Ok(tags.iter().flatten().find_map(|tag| {
            let n = versioning::pre_release_number(tag, &next.tag, channel)?;
            if !at_head(tag) {
                return None;
            }
            Some(TaggedRelease {
                tag: tag.to_string(),
                version: versioning::pre_release(version_core, channel, n),
                from_tag: versions.last().map(|v| v.tag.clone()),
            })
        }));
    }

    match versions.split_last() {
        Some((last, earlier)) if at_head(&last.tag) => Ok(Some(TaggedRelease {
            tag: last.tag.clone(),
            version: last.core()?,
            from_tag: earlier.last().map(|v| v.tag.clone()),
        })),
        _ => Ok(None),
    }
}

/// A release whose tag already points at HEAD, possibly left unfinished by an
/// interrupted run (tag not pushed, or GitHub Release not created).
struct TaggedRelease {
    tag: String,
    version: String,
    from_tag: Option<String>,
}

/// Completes a release that is already tagged at HEAD: pushes the tag if origin
/// doesn't have it and creates the GitHub Release if it's missing. Lets a rerun
/// recover from a failed push or Release instead of reporting nothing to do.
fn finish_release(
    repo: &git2::Repository,
    config: &FlophaConfig,
    args: &ReleaseArgs,
    tagged: TaggedRelease,
) -> Result<Option<String>, FlophaError> {
    if args.dry_run {
        print_nothing_to_release(
            &args.format,
            &format!("{} is already tagged at HEAD", tagged.tag),
        );
        return Ok(None);
    }
    let github = connect_github(repo, config)?;

    let mut remote = gitutils::get_remote(repo, "origin")?;
    let pushed_tag = !gitutils::remote_has_ref(&mut remote, &format!("refs/tags/{}", tagged.tag))?;
    if pushed_tag {
        gitutils::push_tag(&mut remote, &tagged.tag)?;
        println!("Pushed tag {}", tagged.tag);
    }

    let mut created_release = false;
    if let Some((client, repo_slug)) = &github {
        if client.find_release(repo_slug, &tagged.tag)?.is_none() {
            let changelog =
                build_release_changelog(repo, config, tagged.from_tag.as_deref(), &tagged.tag)?;
            let url = create_github_release(
                client,
                repo_slug,
                config,
                &tagged.tag,
                &tagged.version,
                &changelog,
            )
            .map_err(|e| release_failed(&tagged.tag, e))?;
            println!("GitHub Release: {}", url);
            created_release = true;
        }
    }

    if !pushed_tag && !created_release {
        print_nothing_to_release(&args.format, &format!("{} is already released", tagged.tag));
        return Ok(None);
    }
    Ok(Some(tagged.tag))
}

fn print_nothing_to_release(format: &OutputFormat, reason: &str) {
    super::print_none(format, &format!("Nothing to release: {}", reason));
}

/// The tag and the bare manifest version, both with the same `-{channel}.{n}`
/// suffix when `version.pre` is set. The counter is computed once from the tag
/// prefix and applied to both, so the tag and manifests can't disagree.
fn release_tag_and_version(
    repo: &git2::Repository,
    config: &FlophaConfig,
    next: &Version,
    version_core: String,
) -> (String, String) {
    match &config.version.pre {
        Some(channel) => {
            let n = super::next_pre_release_number(repo, &next.tag, channel);
            (
                versioning::pre_release(&next.tag, channel, n),
                versioning::pre_release(&version_core, channel, n),
            )
        }
        None => (next.tag.clone(), version_core),
    }
}

/// Refuses to build a release commit that couldn't be pushed cleanly or would
/// carry unrelated work: it must land on a branch, contain only the version
/// bump, and sit on top of everything already on origin.
fn check_can_commit(repo: &git2::Repository) -> Result<(), FlophaError> {
    if !repo.head()?.is_branch() {
        return Err(FlophaError::Config(
            "flopha.toml declares [[manifest]] targets, so release needs to commit the \
             bump, but HEAD is not on a branch (detached HEAD). Check out a branch first \
             (e.g. `git checkout -B <branch>`)."
                .to_string(),
        ));
    }
    if gitutils::has_uncommitted_changes(repo)? {
        return Err(FlophaError::Config(
            "the working tree has uncommitted changes to tracked files; commit or stash them \
             first so they don't end up in the release commit"
                .to_string(),
        ));
    }
    let branch = gitutils::get_head_branch(repo)?;
    let name = branch.name()?.unwrap_or_default();
    let behind = gitutils::commits_behind_remote(repo, "origin", name)?;
    if behind > 0 {
        return Err(FlophaError::Config(format!(
            "'{}' is {} commit(s) behind origin/{}; pull first so the release commit \
             doesn't revert upstream changes",
            name, behind, name
        )));
    }
    Ok(())
}

/// The changelog for `tag` when `changelog.enabled`. Commits are gathered up
/// to HEAD, since the tag may not exist yet; `tag` only labels the title.
fn build_release_changelog(
    repo: &git2::Repository,
    config: &FlophaConfig,
    from_tag: Option<&str>,
    tag: &str,
) -> Result<Option<String>, FlophaError> {
    if !config.changelog.enabled {
        return Ok(None);
    }
    build_changelog(
        repo,
        &ChangelogRequest {
            from_tag,
            to: None,
            to_label: Some(tag),
            raw_groups: &config.changelog.groups,
            other: config.changelog.other.as_deref(),
            title_template: config.changelog.title.as_deref(),
            format: &OutputFormat::Text,
        },
    )
    .map(Some)
}

fn print_plan(
    format: &OutputFormat,
    from_tag: &Option<String>,
    tag: &str,
    config: &FlophaConfig,
    changelog: &Option<String>,
) {
    let manifest_paths: Vec<&str> = config.manifests.iter().map(|m| m.path()).collect();

    match format {
        OutputFormat::Json => {
            let plan = serde_json::json!({
                "from": from_tag,
                "to": tag,
                "manifests": manifest_paths,
                "commit": !manifest_paths.is_empty(),
                "push": true,
                "release": config.release.create,
                "draft": config.release.draft,
                "changelog": changelog,
            });
            println!("{}", plan);
        }
        OutputFormat::Text => {
            println!("Release plan:");
            println!(
                "  bump:      {} -> {}",
                from_tag.as_deref().unwrap_or("(none)"),
                tag
            );
            if manifest_paths.is_empty() {
                println!("  manifests: (none configured)");
            } else {
                println!("  manifests:");
                for p in &manifest_paths {
                    println!("    - {}", p);
                }
            }
            println!("  tag:       {} (annotated)", tag);
            println!("  push:      origin");
            if config.release.create {
                println!("  release:   yes (draft: {})", config.release.draft);
            } else {
                println!("  release:   no");
            }
            if let Some(cl) = changelog {
                println!("\nChangelog preview:\n{}", cl);
            }
        }
    }
}

/// Resolves the GitHub repo and token and confirms access, before anything is
/// written or pushed, so a missing token or wrong repo can't leave a pushed tag
/// without its Release.
fn connect_github(
    repo: &git2::Repository,
    config: &FlophaConfig,
) -> Result<Option<(GitHubClient, String)>, FlophaError> {
    if !config.release.create {
        return Ok(None);
    }
    let remote = github::remote_repo(repo, "origin");
    let (host, repo_slug) = match &config.release.repo {
        Some(slug) => (
            remote
                .map(|r| r.host)
                .unwrap_or_else(|_| "github.com".to_string()),
            slug.clone(),
        ),
        None => {
            let remote = remote?;
            (remote.host, remote.slug)
        }
    };
    let client = GitHubClient::from_env(&host)?;
    client.check_access(&repo_slug)?;
    Ok(Some((client, repo_slug)))
}

/// Computes every manifest edit before any file is written, so a failing
/// target leaves no partial edits.
fn manifest_updates(
    path: &Path,
    config: &FlophaConfig,
    version: &str,
) -> Result<Vec<(PathBuf, String)>, FlophaError> {
    let mut edits = manifest::Edits::new(path);
    for target in &config.manifests {
        manifest::apply(&mut edits, target, version)?;
    }
    Ok(edits.changes())
}

/// Writes and commits the manifest updates (if any), then creates the annotated tag.
fn commit_and_tag(
    repo: &git2::Repository,
    path: &Path,
    config: &FlophaConfig,
    updates: &[(PathBuf, String)],
    tag: &str,
    version: &str,
) -> Result<(), FlophaError> {
    if !updates.is_empty() {
        for (rel, content) in updates {
            std::fs::write(path.join(rel), content)?;
            gitutils::stage_path(repo, rel)?;
        }
        gitutils::commit(repo, &format!("chore(release): {}", tag))?;
    }

    let tag_message = render(
        config
            .version
            .tag_message
            .as_deref()
            .unwrap_or("Release {tag}"),
        tag,
        version,
    );
    TagVersionSource.create(repo, tag, Some(&tag_message))?;
    Ok(())
}

/// Pushes the release commit (when there is one) and then the tag. If the
/// commit can't be pushed, nothing has reached origin yet, so the local commit
/// and tag are undone and a rerun starts over.
fn push_release(
    repo: &git2::Repository,
    tag: &str,
    has_commit: bool,
    original_head: &git2::Commit,
) -> Result<(), FlophaError> {
    let mut remote = gitutils::get_remote(repo, "origin")?;
    if has_commit {
        let mut branch = gitutils::get_head_branch(repo)?;
        if let Err(e) = gitutils::push_branch(&mut remote, &mut branch) {
            // The hard reset only discards the release commit's own edits:
            // `check_can_commit` required a clean tree.
            repo.tag_delete(tag)?;
            repo.reset(original_head.as_object(), git2::ResetType::Hard, None)?;
            return Err(FlophaError::Config(format!(
                "pushing the release commit failed, so the local commit and tag '{}' were \
                 undone: {}",
                tag, e
            )));
        }
    }
    gitutils::push_tag(&mut remote, tag).map_err(|e| {
        FlophaError::Config(format!(
            "tag '{}' was created but pushing it failed: {}. Re-run `flopha release` to retry.",
            tag, e
        ))
    })
}

/// Fills the `{tag}` and `{version}` placeholders of a config template.
fn render(template: &str, tag: &str, version: &str) -> String {
    template.replace("{tag}", tag).replace("{version}", version)
}

fn create_github_release(
    client: &GitHubClient,
    repo_slug: &str,
    config: &FlophaConfig,
    tag: &str,
    version_core: &str,
    changelog: &Option<String>,
) -> Result<String, FlophaError> {
    let title = render(
        config.release.title.as_deref().unwrap_or("{tag}"),
        tag,
        version_core,
    );
    let body = config.release.body.clone().or_else(|| changelog.clone());

    client.create_release(&ReleaseRequest {
        repo_slug,
        tag,
        title: &title,
        body: body.as_deref(),
        draft: config.release.draft,
        prerelease: config.is_prerelease(),
        generate_notes: config.release.generate_notes,
    })
}

fn release_failed(tag: &str, e: FlophaError) -> FlophaError {
    FlophaError::Config(format!(
        "tag '{}' was pushed, but creating the GitHub Release failed: {}. Re-run \
         `flopha release` to retry.",
        tag, e
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{gitutils, testutils};
    use tempfile::TempDir;

    fn write_config(dir: &Path, content: &str) {
        std::fs::write(dir.join("flopha.toml"), content).unwrap();
    }

    fn release_args() -> ReleaseArgs {
        ReleaseArgs {
            config: "flopha.toml".to_string(),
            dry_run: false,
            format: OutputFormat::Text,
        }
    }

    /// It syncs the configured manifest, commits, tags, and pushes both to origin.
    #[test]
    fn test_release_syncs_manifest_commits_tags_and_pushes() {
        // Given a repo with a Cargo.toml manifest target configured and a tagged v1.0.0
        let (td, repo, remote_td) = repo_with_manifest("");

        // When running the release pipeline
        let result = release(td.path(), &release_args()).unwrap();

        // Then the manifest is updated, the tag is annotated, and it reaches the remote
        assert_eq!(result, Some("v1.0.1".to_string()));

        let content = std::fs::read_to_string(td.path().join("Cargo.toml")).unwrap();
        assert!(content.contains("version = \"1.0.1\""));

        let tag_obj = repo.revparse_single("refs/tags/v1.0.1").unwrap();
        assert_eq!(tag_obj.kind(), Some(git2::ObjectType::Tag));

        let remote_repo = git2::Repository::open(remote_td.path()).unwrap();
        assert!(remote_repo
            .tag_names(None)
            .unwrap()
            .iter()
            .any(|t| t == Some("v1.0.1")));
    }

    /// It computes and prints the plan without creating a tag or touching files.
    #[test]
    fn test_release_dry_run_makes_no_changes() {
        // Given a repo tagged v1.0.0 with one commit since
        let (td, repo) = testutils::init_repo();
        let (_remote_td, _remote) = testutils::init_remote(&repo);

        testutils::tag_head(&repo, "v1.0.0");
        gitutils::commit(&repo, "fix: something").unwrap();

        write_config(td.path(), "");

        // When running with --dry-run
        let args = ReleaseArgs {
            dry_run: true,
            ..release_args()
        };
        let result = release(td.path(), &args).unwrap();

        // Then the next tag is reported but never created
        assert_eq!(result, Some("v1.0.1".to_string()));
        assert!(
            repo.revparse_single("refs/tags/v1.0.1").is_err(),
            "dry-run must not create a tag"
        );
    }

    /// It still tags and pushes when no manifest targets are configured.
    #[test]
    fn test_release_without_manifests_only_tags_and_pushes() {
        // Given a repo tagged v1.0.0 and an empty flopha.toml (no manifest targets)
        let (td, repo) = testutils::init_repo();
        let (remote_td, _remote) = testutils::init_remote(&repo);

        testutils::tag_head(&repo, "v1.0.0");
        gitutils::commit(&repo, "fix: something").unwrap();

        write_config(td.path(), "");

        // When running the release pipeline
        let result = release(td.path(), &release_args()).unwrap();

        // Then the tag alone is created and pushed
        assert_eq!(result, Some("v1.0.1".to_string()));
        let remote_repo = git2::Repository::open(remote_td.path()).unwrap();
        assert!(remote_repo
            .tag_names(None)
            .unwrap()
            .iter()
            .any(|t| t == Some("v1.0.1")));
    }

    /// It generates and prints a changelog covering commits up to HEAD, even though
    /// the new tag doesn't exist yet when the changelog is built.
    #[test]
    fn test_release_with_changelog_enabled_succeeds() {
        // Given changelog.enabled = true and a feature commit since the last tag
        let (td, repo) = testutils::init_repo();
        let (_remote_td, _remote) = testutils::init_remote(&repo);

        testutils::tag_head(&repo, "v1.0.0");
        gitutils::commit(&repo, "feat: add search").unwrap();

        write_config(
            td.path(),
            r#"
                [changelog]
                enabled = true
            "#,
        );

        // When running the release pipeline
        let result = release(td.path(), &release_args());

        // Then it succeeds and creates the bumped tag (a prior bug asked for commits
        // up to the not-yet-existing tag and always failed with "not found")
        assert_eq!(result.unwrap(), Some("v1.1.0".to_string()));
        assert!(repo.revparse_single("refs/tags/v1.1.0").is_ok());
    }

    /// It uses the same pre-release counter for the git tag and the manifest
    /// version on the *second* pre-release of a given base version — a prior bug
    /// derived the counter from two different base strings (the full tag vs. the
    /// bare version), which only diverged once a `-{channel}.2` tag already existed.
    #[test]
    fn test_release_pre_release_tag_and_manifest_version_share_the_same_counter() {
        // Given a repo already on v1.0.1-beta.1 and a Cargo.toml manifest target
        let (td, repo) = testutils::init_repo();
        let (_remote_td, mut remote) = testutils::init_remote(&repo);

        std::fs::write(
            td.path().join("Cargo.toml"),
            "[package]\nname = \"app\"\nversion = \"1.0.1-beta.1\"\n",
        )
        .unwrap();
        gitutils::stage_path(&repo, Path::new("Cargo.toml")).unwrap();
        gitutils::commit(&repo, "chore: add manifest").unwrap();
        let commit_id = repo.head().unwrap().peel_to_commit().unwrap().id();
        gitutils::tag_oid(&repo, commit_id, "v1.0.0").unwrap();
        gitutils::tag_oid(&repo, commit_id, "v1.0.1-beta.1").unwrap();
        remote
            .push(&["refs/tags/v1.0.0", "refs/tags/v1.0.1-beta.1"], None)
            .unwrap();
        gitutils::commit(&repo, "fix: something").unwrap();

        write_config(
            td.path(),
            r#"
                [version]
                pre = "beta"

                [[manifest]]
                path = "Cargo.toml"
                type = "cargo"
            "#,
        );

        // When releasing a second beta pre-release
        let result = release(td.path(), &release_args()).unwrap();

        // Then the tag and the manifest version both carry counter 2, not one
        // counter 2 and the other silently stuck at 1
        assert_eq!(result, Some("v1.0.1-beta.2".to_string()));
        let content = std::fs::read_to_string(td.path().join("Cargo.toml")).unwrap();
        assert!(
            content.contains("version = \"1.0.1-beta.2\""),
            "manifest version should match the pushed tag's counter, got: {content}"
        );
    }

    /// It exits successfully without tagging when HEAD has no commits since the last release.
    #[test]
    fn test_release_with_no_new_commits_is_a_no_op() {
        // Given a repo whose HEAD is already tagged v1.0.0 on origin
        let (td, repo) = testutils::init_repo();
        let (_remote_td, mut remote) = testutils::init_remote(&repo);

        testutils::tag_head(&repo, "v1.0.0");
        remote.push(&["refs/tags/v1.0.0"], None).unwrap();

        write_config(td.path(), "");

        // When running the release pipeline
        let result = release(td.path(), &release_args());

        // Then it succeeds with nothing released and no new tag is created
        assert_eq!(result.unwrap(), None);
        assert!(repo.revparse_single("refs/tags/v1.0.1").is_err());
    }

    /// It pushes a tag left local by an interrupted run instead of reporting nothing to do.
    #[test]
    fn test_release_rerun_pushes_unpushed_tag() {
        // Given HEAD tagged v1.0.0 locally, but the tag never reached origin
        let (td, repo) = testutils::init_repo();
        let (remote_td, _remote) = testutils::init_remote(&repo);

        testutils::tag_head(&repo, "v1.0.0");

        write_config(td.path(), "");

        // When re-running the release pipeline
        let result = release(td.path(), &release_args());

        // Then it finishes v1.0.0 by pushing its tag, without cutting a new version
        assert_eq!(result.unwrap(), Some("v1.0.0".to_string()));
        let remote_repo = git2::Repository::open(remote_td.path()).unwrap();
        assert!(remote_repo.revparse_single("refs/tags/v1.0.0").is_ok());
        assert!(repo.revparse_single("refs/tags/v1.0.1").is_err());
    }

    /// It doesn't cut another pre-release when HEAD already carries one.
    #[test]
    fn test_release_pre_release_rerun_on_released_head_is_a_no_op() {
        // Given v1.0.0 on an earlier commit and HEAD already released as v1.0.1-beta.1
        let (td, repo) = testutils::init_repo();
        let (_remote_td, mut remote) = testutils::init_remote(&repo);

        let base = repo.head().unwrap().peel_to_commit().unwrap().id();
        gitutils::tag_oid(&repo, base, "v1.0.0").unwrap();
        let head = gitutils::commit(&repo, "fix: something").unwrap();
        gitutils::tag_oid(&repo, head, "v1.0.1-beta.1").unwrap();
        remote
            .push(&["refs/tags/v1.0.0", "refs/tags/v1.0.1-beta.1"], None)
            .unwrap();

        write_config(
            td.path(),
            r#"
                [version]
                pre = "beta"
            "#,
        );

        // When re-running the release pipeline
        let result = release(td.path(), &release_args());

        // Then nothing new is released
        assert_eq!(result.unwrap(), None);
        assert!(repo.revparse_single("refs/tags/v1.0.1-beta.2").is_err());
    }

    /// Sets up a repo with a committed Cargo.toml at 1.0.0 tagged v1.0.0, one fix
    /// commit since, and a config syncing that manifest.
    fn repo_with_manifest(extra_config: &str) -> (TempDir, git2::Repository, TempDir) {
        let (td, repo) = testutils::init_repo();
        let remote_td = {
            let (remote_td, mut remote) = testutils::init_remote(&repo);
            std::fs::write(
                td.path().join("Cargo.toml"),
                "[package]\nname = \"app\"\nversion = \"1.0.0\"\n",
            )
            .unwrap();
            gitutils::stage_path(&repo, Path::new("Cargo.toml")).unwrap();
            let tagged = gitutils::commit(&repo, "chore: add manifest").unwrap();
            gitutils::tag_oid(&repo, tagged, "v1.0.0").unwrap();
            gitutils::commit(&repo, "fix: something").unwrap();
            remote
                .push(&["refs/heads/main", "refs/tags/v1.0.0"], None)
                .unwrap();
            remote_td
        };
        write_config(
            td.path(),
            &format!(
                "{}\n[[manifest]]\npath = \"Cargo.toml\"\ntype = \"cargo\"\n",
                extra_config
            ),
        );
        (td, repo, remote_td)
    }

    /// It undoes the local release commit and tag when origin rejects the push.
    #[test]
    fn test_release_rolls_back_when_branch_push_fails() {
        // Given a repo whose origin can be fetched from but not pushed to
        let (td, repo, _remote_td) = repo_with_manifest("");
        let head_before = repo.head().unwrap().peel_to_commit().unwrap().id();
        repo.remote_set_pushurl("origin", Some("file:///nonexistent/flopha-remote"))
            .unwrap();

        // When running the release pipeline
        let result = release(td.path(), &release_args());

        // Then it errors after undoing the local commit, tag, and manifest edit
        let err = result.unwrap_err();
        assert!(err.to_string().contains("were undone"), "{err}");
        assert!(repo.revparse_single("refs/tags/v1.0.1").is_err());
        assert_eq!(
            repo.head().unwrap().peel_to_commit().unwrap().id(),
            head_before
        );
        let content = std::fs::read_to_string(td.path().join("Cargo.toml")).unwrap();
        assert!(content.contains("version = \"1.0.0\""));
    }

    /// It refuses to commit when tracked files have uncommitted changes.
    #[test]
    fn test_release_with_dirty_tree_is_rejected() {
        // Given an uncommitted edit to the manifest
        let (td, repo, _remote_td) = repo_with_manifest("");
        let dirty = "[package]\nname = \"app\"\nversion = \"1.0.0\"\nedition = \"2021\"\n";
        std::fs::write(td.path().join("Cargo.toml"), dirty).unwrap();

        // When running the release pipeline
        let err = release(td.path(), &release_args()).unwrap_err();

        // Then it errors without tagging or touching the edit
        assert!(err.to_string().contains("uncommitted changes"), "{err}");
        assert!(repo.revparse_single("refs/tags/v1.0.1").is_err());
        assert_eq!(
            std::fs::read_to_string(td.path().join("Cargo.toml")).unwrap(),
            dirty
        );
    }

    /// It refuses to commit on top of a branch that is behind origin.
    #[test]
    fn test_release_behind_origin_is_rejected() {
        // Given origin/main has a commit the local branch doesn't
        let (td, repo, remote_td) = repo_with_manifest("");
        let remote_repo = git2::Repository::open(remote_td.path()).unwrap();
        {
            let parent = remote_repo.head().unwrap().peel_to_commit().unwrap();
            let sig = git2::Signature::now("name", "email").unwrap();
            remote_repo
                .commit(
                    Some("refs/heads/main"),
                    &sig,
                    &sig,
                    "feat: upstream change",
                    &parent.tree().unwrap(),
                    &[&parent],
                )
                .unwrap();
        }

        // When running the release pipeline
        let err = release(td.path(), &release_args()).unwrap_err();

        // Then it asks to pull first and creates nothing
        assert!(err.to_string().contains("behind origin/main"), "{err}");
        assert!(repo.revparse_single("refs/tags/v1.0.1").is_err());
    }

    /// It fills `{tag}` and `{version}` into the configured annotated tag message.
    #[test]
    fn test_release_tag_message_substitutes_placeholders() {
        // Given a tag_message template
        let (td, repo, _remote_td) =
            repo_with_manifest("[version]\ntag_message = \"Release {tag} ({version})\"\n");

        // When running the release pipeline
        release(td.path(), &release_args()).unwrap();

        // Then the annotated tag carries the substituted message
        let tag = repo
            .revparse_single("refs/tags/v1.0.1")
            .unwrap()
            .peel_to_tag()
            .unwrap();
        assert_eq!(tag.message().unwrap().trim(), "Release v1.0.1 (1.0.1)");
    }

    /// It fails before tagging or touching manifests when the GitHub Release can't be created.
    #[test]
    fn test_release_create_fails_before_any_side_effect_when_github_is_unresolvable() {
        // Given release.create = true, a manifest target, and an origin that isn't on GitHub
        let (td, repo, _remote_td) = repo_with_manifest("[release]\ncreate = true\n");
        let manifest = std::fs::read_to_string(td.path().join("Cargo.toml")).unwrap();

        // When running the release pipeline
        let err = release(td.path(), &release_args()).unwrap_err();

        // Then it errors without creating the tag or rewriting the manifest
        assert!(err.to_string().contains("GitHub owner/repo"), "{err}");
        assert!(repo.revparse_single("refs/tags/v1.0.1").is_err());
        assert_eq!(
            std::fs::read_to_string(td.path().join("Cargo.toml")).unwrap(),
            manifest
        );
    }

    /// It rejects `version.source = "branch"` up front.
    #[test]
    fn test_release_branch_source_is_rejected() {
        // Given a config that sets version.source to "branch"
        let (td, repo) = testutils::init_repo();
        let (_remote_td, _remote) = testutils::init_remote(&repo);

        write_config(
            td.path(),
            r#"
                [version]
                source = "branch"
            "#,
        );

        // When running the release pipeline
        let result = release(td.path(), &release_args());

        // Then it errors before touching the repository
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("version.source"));
    }

    /// It errors clearly when the config file doesn't exist.
    #[test]
    fn test_release_missing_config_file_errors() {
        // Given a repo with no flopha.toml
        let (td, repo) = testutils::init_repo();
        let (_remote_td, _remote) = testutils::init_remote(&repo);

        // When running the release pipeline
        let result = release(td.path(), &release_args());

        // Then it errors
        assert!(result.is_err());
    }
}

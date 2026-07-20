use std::path::{Path, PathBuf};

use crate::cli::{OutputFormat, ReleaseArgs};
use crate::config::FlophaConfig;
use crate::error::FlophaError;
use crate::github::{self, ReleaseRequest};
use crate::gitutils;
use crate::manifest;
use crate::service::{self, ChangelogRequest};
use crate::version_source::{TagVersionSource, VersionSource};
use crate::versioning::{Version, Versioner};

/// Runs the full config-driven release pipeline described by `flopha.toml`:
/// compute bump -> sync manifests -> commit -> annotated tag -> push ->
/// changelog -> GitHub Release. Returns the tag that was created (or would be,
/// on `--dry-run`).
pub fn release(path: &Path, args: &ReleaseArgs) -> Result<Option<String>, FlophaError> {
    let config = FlophaConfig::load(&path.join(&args.config))?;

    let repo = gitutils::get_repo(path)?;
    service::try_fetch_from_origin(&repo);

    let versioner = Versioner::new(
        TagVersionSource.fetch_all(&repo),
        config.version.pattern.clone(),
    );
    let last = versioner.last_version();
    let from_tag = last.as_ref().map(|v| v.tag.clone());

    let increment = service::resolve_increment(
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
    let version_core = bare_version(&next)?;

    // Both the tag and the bare manifest version get the same `-{channel}.{n}`
    // suffix, so the counter is computed once and applied to each — computing
    // it twice via two `pre_release_tag` calls (one keyed on `next.tag`, one on
    // `version_core`) would look up the counter against two different prefixes
    // and could silently return different numbers for the tag vs. the manifest.
    let (tag, version_core) = match &config.version.pre {
        Some(channel) => {
            let n = service::next_pre_release_number(&repo, &next.tag, channel);
            (
                format!("{}-{}.{}", next.tag, channel, n),
                format!("{}-{}.{}", version_core, channel, n),
            )
        }
        None => (next.tag.clone(), version_core),
    };

    // A `flopha.toml` with `[[manifest]]` targets means we're about to commit; that
    // commit must land on a branch we can push, not a detached HEAD (the common state
    // after CI checkouts of a specific ref/SHA). Fail before touching any file rather
    // than after the commit is already made locally with nowhere to push it.
    if !config.manifests.is_empty() && !args.dry_run && !repo.head()?.is_branch() {
        return Err(FlophaError::Config(
            "flopha.toml declares [[manifest]] targets, so release needs to commit the \
             bump, but HEAD is not on a branch (detached HEAD). Check out a branch first \
             (e.g. `git checkout -B <branch>`)."
                .to_string(),
        ));
    }

    let changelog = if config.changelog.enabled {
        Some(build_release_changelog(
            &repo,
            &config,
            from_tag.as_deref(),
            &tag,
        )?)
    } else {
        None
    };

    if args.dry_run {
        print_plan(&args.format, &from_tag, &tag, &config, &changelog);
        return Ok(Some(tag));
    }

    let mut updates: Vec<(PathBuf, String)> = Vec::new();
    for target in &config.manifests {
        if let Some(update) = manifest::compute(path, target, &version_core)? {
            updates.push(update);
        }
    }
    for (rel, content) in &updates {
        std::fs::write(path.join(rel), content)?;
    }

    if !updates.is_empty() {
        for (rel, _) in &updates {
            gitutils::stage_path(&repo, rel)?;
        }
        gitutils::commit(&repo, &format!("chore(release): {}", tag))?;
    }

    let tag_message = config
        .version
        .tag_message
        .clone()
        .unwrap_or_else(|| format!("Release {}", tag));
    TagVersionSource.create(&repo, &tag, Some(&tag_message))?;

    let mut remote = gitutils::get_remote(&repo, "origin")?;
    if !updates.is_empty() {
        let mut branch = gitutils::get_head_branch(&repo)?;
        gitutils::push_branch(&mut remote, &mut branch)?;
    }
    gitutils::push_tag(&mut remote, &tag)?;

    let release_url = if config.release.create {
        let url = create_github_release(&repo, &config, &tag, &version_core, &changelog).map_err(
            |e| {
                FlophaError::Config(format!(
                    "tag '{}' was created and pushed, but creating the GitHub Release failed: {}",
                    tag, e
                ))
            },
        )?;
        Some(url)
    } else {
        None
    };

    println!("Released {}", tag);
    if let Some(url) = &release_url {
        println!("GitHub Release: {}", url);
    }

    Ok(Some(tag))
}

/// Extracts the bare `major.minor.patch` string manifest files are synced with.
/// Errors (rather than silently defaulting to `0`) when `version.pattern` is
/// scoped and doesn't capture every component, e.g. `v1.{minor}.{patch}`.
fn bare_version(version: &Version) -> Result<String, FlophaError> {
    let major = version
        .major
        .ok_or_else(|| FlophaError::MissingVersionComponent("major".to_string()))?;
    let minor = version
        .minor
        .ok_or_else(|| FlophaError::MissingVersionComponent("minor".to_string()))?;
    let patch = version
        .patch
        .ok_or_else(|| FlophaError::MissingVersionComponent("patch".to_string()))?;
    Ok(format!("{}.{}.{}", major, minor, patch))
}

/// Builds the changelog for the upcoming release. Commits are gathered up to
/// HEAD (`to: None`) rather than the new `tag`, since the tag doesn't exist yet
/// at this point in the pipeline — `build_changelog` would otherwise fail
/// trying to resolve it. The `{to}` placeholder in the title is filled in with
/// `tag` before delegating, so config-level title templates still work.
fn build_release_changelog(
    repo: &git2::Repository,
    config: &FlophaConfig,
    from_tag: Option<&str>,
    tag: &str,
) -> Result<String, FlophaError> {
    let title_template = config
        .changelog
        .title
        .clone()
        .unwrap_or_else(|| "Changes in {to}".to_string())
        .replace("{to}", tag);

    service::build_changelog(
        repo,
        &ChangelogRequest {
            from_tag,
            to: None,
            raw_groups: &config.changelog.groups,
            other: config.changelog.other.as_deref(),
            title_template: Some(&title_template),
            format: &OutputFormat::Text,
        },
    )
}

fn create_github_release(
    repo: &git2::Repository,
    config: &FlophaConfig,
    tag: &str,
    version_core: &str,
    changelog: &Option<String>,
) -> Result<String, FlophaError> {
    let repo_slug = match &config.release.repo {
        Some(slug) => slug.clone(),
        None => github::repo_slug_from_remote(repo, "origin")?,
    };
    let title = config
        .release
        .title
        .clone()
        .unwrap_or_else(|| tag.to_string())
        .replace("{tag}", tag)
        .replace("{version}", version_core);
    let body = config.release.body.clone().or_else(|| changelog.clone());

    github::create_release(&ReleaseRequest {
        repo_slug: &repo_slug,
        tag,
        title: &title,
        body: body.as_deref(),
        draft: config.release.draft,
        prerelease: config.is_prerelease(),
        generate_notes: config.release.generate_notes,
    })
}

fn print_plan(
    format: &OutputFormat,
    from_tag: &Option<String>,
    tag: &str,
    config: &FlophaConfig,
    changelog: &Option<String>,
) {
    let manifest_paths: Vec<&str> = config.manifests.iter().map(|m| m.path.as_str()).collect();

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{gitutils, testutils};

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
        let (td, repo) = testutils::init_repo();
        let (remote_td, _remote) = testutils::init_remote(&repo);

        std::fs::write(
            td.path().join("Cargo.toml"),
            "[package]\nname = \"app\"\nversion = \"1.0.0\"\n",
        )
        .unwrap();
        gitutils::stage_path(&repo, Path::new("Cargo.toml")).unwrap();
        gitutils::commit(&repo, "chore: add manifest").unwrap();
        gitutils::tag_oid(
            &repo,
            repo.head().unwrap().peel_to_commit().unwrap().id(),
            "v1.0.0",
        )
        .unwrap();
        gitutils::commit(&repo, "fix: something").unwrap();

        write_config(
            td.path(),
            r#"
                [[manifest]]
                path = "Cargo.toml"
                type = "cargo"
            "#,
        );

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

        gitutils::tag_oid(
            &repo,
            repo.head().unwrap().peel_to_commit().unwrap().id(),
            "v1.0.0",
        )
        .unwrap();
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

        gitutils::tag_oid(
            &repo,
            repo.head().unwrap().peel_to_commit().unwrap().id(),
            "v1.0.0",
        )
        .unwrap();
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

        gitutils::tag_oid(
            &repo,
            repo.head().unwrap().peel_to_commit().unwrap().id(),
            "v1.0.0",
        )
        .unwrap();
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

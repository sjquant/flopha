use std::path::{Path, PathBuf};

use crate::cli::{OutputFormat, ReleaseArgs};
use crate::config::FlophaConfig;
use crate::error::FlophaError;
use crate::github::{self, ReleaseRequest};
use crate::gitutils;
use crate::manifest;
use crate::service;
use crate::version_source::{TagVersionSource, VersionSource};
use crate::versioning::Versioner;

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
    let from_tag = versioner.last_version().map(|v| v.tag);

    let increment = service::resolve_increment(
        &repo,
        &versioner,
        config.version.auto,
        &config.version.rules,
        config.version.increment.clone(),
    )?;

    let next = versioner.next_version(increment)?.ok_or_else(|| {
        FlophaError::Config(
            "no version tag matches version.pattern; nothing to bump from".to_string(),
        )
    })?;

    let bare_version = format!(
        "{}.{}.{}",
        next.major.unwrap_or(0),
        next.minor.unwrap_or(0),
        next.patch.unwrap_or(0)
    );

    let (tag, bare_version) = match &config.version.pre {
        Some(channel) => {
            let n = service::next_pre_release_number(&repo, &next.tag, channel);
            (
                format!("{}-{}.{}", next.tag, channel, n),
                format!("{}-{}.{}", bare_version, channel, n),
            )
        }
        None => (next.tag.clone(), bare_version),
    };

    let changelog = if config.changelog.enabled {
        Some(service::build_changelog(
            &repo,
            from_tag.as_deref(),
            Some(&tag),
            &config.changelog.groups,
            config.changelog.other.as_deref(),
            config.changelog.title.as_deref(),
            &OutputFormat::Text,
        )?)
    } else {
        None
    };

    if args.dry_run {
        print_plan(&args.format, &from_tag, &tag, &config, &changelog);
        return Ok(Some(tag));
    }

    let mut touched: Vec<PathBuf> = Vec::new();
    for target in &config.manifests {
        if let Some(rel) = manifest::sync(path, target, &bare_version)? {
            touched.push(rel);
        }
    }

    if !touched.is_empty() {
        for rel in &touched {
            gitutils::stage_path(&repo, rel)?;
        }
        gitutils::commit(&repo, &format!("chore(release): {}", tag))?;
    }

    let tag_message = config
        .version
        .tag_message
        .clone()
        .unwrap_or_else(|| format!("Release {}", tag));
    let head_commit = repo.head()?.peel_to_commit()?;
    gitutils::annotated_tag_oid(&repo, head_commit.id(), &tag, &tag_message)?;

    let mut remote = gitutils::get_remote(&repo, "origin")?;
    if !touched.is_empty() {
        let mut branch = gitutils::get_head_branch(&repo)?;
        gitutils::push_branch(&mut remote, &mut branch)?;
    }
    gitutils::push_tag(&mut remote, &tag)?;

    let release_url = if config.release.create {
        Some(create_github_release(
            &repo,
            &config,
            &tag,
            &bare_version,
            &changelog,
        )?)
    } else {
        None
    };

    println!("Released {}", tag);
    if let Some(url) = &release_url {
        println!("GitHub Release: {}", url);
    }

    Ok(Some(tag))
}

fn create_github_release(
    repo: &git2::Repository,
    config: &FlophaConfig,
    tag: &str,
    bare_version: &str,
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
        .replace("{version}", bare_version);
    let body = config.release.body.clone().or_else(|| changelog.clone());
    let prerelease = config
        .release
        .prerelease
        .unwrap_or(config.version.pre.is_some());

    github::create_release(&ReleaseRequest {
        repo_slug: &repo_slug,
        tag,
        title: &title,
        body: body.as_deref(),
        draft: config.release.draft,
        prerelease,
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

    #[test]
    fn test_release_syncs_manifest_commits_tags_and_pushes() {
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

        let result = release(td.path(), &release_args()).unwrap();

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

    #[test]
    fn test_release_dry_run_makes_no_changes() {
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

        let args = ReleaseArgs {
            dry_run: true,
            ..release_args()
        };
        let result = release(td.path(), &args).unwrap();

        assert_eq!(result, Some("v1.0.1".to_string()));
        assert!(
            repo.revparse_single("refs/tags/v1.0.1").is_err(),
            "dry-run must not create a tag"
        );
    }

    #[test]
    fn test_release_without_manifests_only_tags_and_pushes() {
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

        let result = release(td.path(), &release_args()).unwrap();

        assert_eq!(result, Some("v1.0.1".to_string()));
        let remote_repo = git2::Repository::open(remote_td.path()).unwrap();
        assert!(remote_repo
            .tag_names(None)
            .unwrap()
            .iter()
            .any(|t| t == Some("v1.0.1")));
    }

    #[test]
    fn test_release_branch_source_is_rejected() {
        let (td, repo) = testutils::init_repo();
        let (_remote_td, _remote) = testutils::init_remote(&repo);

        write_config(
            td.path(),
            r#"
                [version]
                source = "branch"
            "#,
        );

        let result = release(td.path(), &release_args());

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("version.source"));
    }

    #[test]
    fn test_release_missing_config_file_errors() {
        let (td, repo) = testutils::init_repo();
        let (_remote_td, _remote) = testutils::init_remote(&repo);

        let result = release(td.path(), &release_args());

        assert!(result.is_err());
    }
}

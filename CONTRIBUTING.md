# Contributing

## Setup

Install dev tools and activate git hooks after cloning:

```sh
cargo install prek
prek install
prek install --hook-type commit-msg
```

Hooks enforce formatting, linting, secret detection, and [gitmoji](https://gitmoji.dev) commit messages.

## Docs site

The site at [flopha.solaqua.dev](https://flopha.solaqua.dev) lives in `website/`.

```sh
pnpm docs:start   # local preview
pnpm docs:build   # production build
```

## Releasing flopha

Prepare a version PR from a branch containing the changes to release:

```sh
cargo run --locked -- release --prepare
git add Cargo.toml Cargo.lock
git commit -m "🔖 Prepare release"
```

Open a PR for those changes. `flopha.toml` derives major bumps from `💥`, minor bumps from `✨`, and patch bumps otherwise. CI verifies version PRs with `release --require-version-match --dry-run`. On merge to `main`, an unpublished Cargo version starts the release workflow: it verifies the prepared files, creates a tag and draft Release, builds the tagged source, uploads binaries, then publishes the Release. Published versions are skipped on ordinary pushes.

If commits added since preparation change the calculated version, verification stops before tagging or publishing. Update the version branch from `main`, rerun `--prepare`, and commit the new versions. If the PR has already merged, open a follow-up version PR. Keep version PR titles neutral (`🔖`), since their commit messages also participate in bump detection. If a build or upload fails after creating the draft, rerun the original failed workflow to finish that same release.

The workflow builds flopha from source so it can use these options before they appear in a published binary. It does not push version commits to `main`.

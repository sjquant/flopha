<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/brand/flopha-icon-dark.svg">
  <img src="assets/brand/flopha-icon.svg" alt="" width="64">
</picture>

# flopha

**Turn merged commits into your next tagged release.**

flopha reads your Git tags and commit messages, picks the next semantic version, and creates the tag, changelog, and GitHub Release. One binary for any language: run it from your terminal, any CI, or as a GitHub Action.

```console
$ git log --oneline v1.2.3..
3f2c1a9 fix: handle empty input
a1b2c3d feat: add release notes export

$ flopha release    # with [release] create = true in flopha.toml
Released v1.3.0
GitHub Release: https://github.com/acme/app/releases/tag/v1.3.0
```

## Install

macOS (Intel, Apple silicon) and Linux x86_64:

```sh
curl -fsSL https://raw.githubusercontent.com/sjquant/flopha/main/scripts/install.sh | sh
```

On other platforms, build from source with `cargo install --git https://github.com/sjquant/flopha --locked`.

## Quick start

Run these inside a Git repository. flopha looks for tags like `v1.2.3`; if you have none yet, add `--initial-version 0.1.0` to choose the first one.

```sh
flopha next-version --auto                   # feat: -> minor, breaking -> major, otherwise patch
flopha next-version --auto --create --push   # create and push that tag
```

## Release with one command

Check in a `flopha.toml` and `flopha release` bumps the version, writes it into your manifests, commits, tags, pushes, and publishes a GitHub Release with a generated changelog.

```toml
[release]
create = true

[[manifest]]
path = "package.json"   # also: Cargo.toml, pyproject.toml, or any file via regex
type = "npm"
```

```sh
flopha release --dry-run   # preview the plan
flopha release
```

Re-running is safe: with no new commits it does nothing, and on an already-tagged commit it only finishes what was left undone. Creating a GitHub Release needs `GH_TOKEN` or `GITHUB_TOKEN` (or `gh auth login` locally).

## GitHub Action

Tag the next version and publish a release from a workflow, with no config file:

```yaml
permissions:
  contents: write

steps:
  - uses: actions/checkout@v4
    with:
      fetch-depth: 0
  - uses: sjquant/flopha@v1
    with:
      create-release: true
      changelog: true
```

See the [action reference](docs/github-action.md) for inputs and outputs.

## Fits the release flow you already have

- **Custom tag patterns** for apps and packages in one repo: `--pattern "mobile@{semver}"`
- **Release branches** instead of tags: `--source branch --pattern "release/{semver}"`
- **Pre-releases** from the next stable version: `--pre rc` gives `v1.3.0-rc.1`
- **Your own commit conventions**: `--auto --rule 'minor:^feature'`
- **Version PRs** when releases need review: `flopha release --prepare`

## Documentation

[flopha.solaqua.dev](https://flopha.solaqua.dev) covers [version patterns](https://flopha.solaqua.dev/docs/version-patterns), [release workflows](https://flopha.solaqua.dev/docs/release-workflows), and the full [command and `flopha.toml` reference](https://flopha.solaqua.dev/docs/command-reference).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

[MIT](LICENSE)

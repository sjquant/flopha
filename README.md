<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/brand/flopha-icon-dark.svg">
  <img src="assets/brand/flopha-icon.svg" alt="" width="64">
</picture>

# flopha

[English](README.md) · [한국어](README.ko.md) · [日本語](README.ja.md) · [简体中文](README.zh-CN.md)

**Turn merged commits into your next tagged release.**

For maintainers who release Git-based projects, flopha turns tags and commit messages into the next semantic version, then can sync manifests, push a tag, and publish a GitHub Release with a changelog. One binary across languages, from your terminal or CI—no Node.js or Python runtime required for the CLI.

```console
$ git log --oneline v1.2.3..
3f2c1a9 fix: handle empty input
a1b2c3d feat: add release notes export

$ flopha next-version --auto
v1.3.0
```

## See your next version

Prebuilt binaries support macOS (Intel and Apple silicon) and Linux x86_64. The installer downloads and installs a binary from GitHub Releases:

```sh
curl -fsSL https://raw.githubusercontent.com/sjquant/flopha/main/scripts/install.sh | sh
```

It installs to `~/.flopha/bin` by default. If `flopha` is not found, follow the [PATH setup](https://flopha.solaqua.dev/docs/installation).

Verify the install, then calculate inside a Git repository with a matching version tag, such as `v1.2.3` with the default pattern. If no tag matches yet, replace the calculation below with `flopha next-version --initial-version 0.1.0` to preview `v0.1.0`:

```sh
flopha --version
flopha next-version --auto
```

With `v1.2.3` and the commits above, the result is `v1.3.0`. `feat:` bumps minor, breaking changes (`feat!:` or `BREAKING CHANGE`) bump major, and other commits bump patch.

Calculation does not create or push a tag; flopha attempts to fetch `origin` first, using local history if the fetch fails. In CI, fetch the full history and tags.

For other platforms, [build from source](https://flopha.solaqua.dev/docs/installation#build-from-source) with Rust 1.85+.

For a tag-only release, create the tag and push it to `origin` (requires write access):

```sh
flopha next-version --auto --create --push
```

For the first tag, use `--initial-version 0.1.0` in place of `--auto`.

## Keep manifests and the release together

If your version also lives in a manifest, check in a `flopha.toml`. This example targets an existing npm `package.json`; Cargo, Python and regex-based targets are also supported.

```toml
[release]
create = true

[[manifest]]
path = "package.json"
type = "npm"
```

```sh
flopha release --dry-run
flopha release
```

`--dry-run` previews the plan without editing manifests, committing, tagging, pushing, or publishing. The second command performs those steps. Run it from a clean working tree on a branch with an `origin` remote you can push to. Creating the GitHub Release needs `GH_TOKEN`, `GITHUB_TOKEN`, or local `gh auth login`. With no version tags, add `--initial-version 0.1.0` to both commands.

When HEAD is already tagged, `release` attempts to finish outstanding pushes or GitHub Release creation rather than bumping again. With no new commits, it does not create another version. For review before publishing, use `flopha release --prepare` to update manifests only, then follow the [version PR workflow](https://flopha.solaqua.dev/docs/release-workflows).

## Tag and publish from GitHub Actions

For projects that use tags as their version source, add these permissions and steps to a workflow job. This pushes a tag and creates a GitHub Release with a changelog; it does **not** update manifest files. The repository needs a matching version tag to start from.

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

See the [action reference (English)](docs/github-action.md) for inputs and outputs, or [release workflows](https://flopha.solaqua.dev/docs/release-workflows) for manifest syncing in CI.

## Fit your release conventions

Keep tags such as `mobile@1.2.3`, use release branches, or cut `rc` pre-releases with [version patterns](https://flopha.solaqua.dev/docs/version-patterns). Custom commit rules let you keep your own commit conventions. See the [command and `flopha.toml` reference](https://flopha.solaqua.dev/docs/command-reference) for the details.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

[MIT](LICENSE)

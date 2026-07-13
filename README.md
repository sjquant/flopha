# flopha

flopha is a CLI for semantic versioning and Git release workflows. It helps teams calculate the next version, manage Git tags and release branches, generate pre-release versions, and automate version bumps from conventional commits.

Use flopha when you want a lightweight release management tool for Git repositories without wiring up a larger release pipeline.

## GitHub Action

Use flopha as a GitHub Action to auto-tag the next semantic version and optionally create a GitHub Release. No setup required.

```yaml
- uses: sjquant/flopha@v1
```

See [Full action docs: inputs, outputs, and examples](docs/github-action.md).

---

## Install

Shell (Mac, Linux):

```
curl -fsSL https://raw.githubusercontent.com/sjquant/flopha/main/scripts/install.sh | sh
```

## Docs

- Official docs: https://sjquant.github.io/flopha
- Website source: `website/`
- Local preview: `pnpm docs:start`
- Production build: `pnpm docs:build`

## Features

- Semantic versioning CLI for Git tags and release branches
- Conventional commit auto-detection for major, minor, and patch bumps
- Pre-release channel support for `alpha`, `beta`, and `rc` style versions
- Custom version patterns for app releases, desktop builds, and monorepo naming
- Version history output for release auditing and changelog workflows

## Quickstart

Run commands inside a Git repository with release tags such as `v1.2.3`.

Check the latest version:

```sh
flopha last-version
```

Calculate the next patch version:

```sh
flopha next-version
```

Let commit messages decide the bump:

```sh
flopha next-version --auto
```

Create the next version in Git:

```sh
flopha next-version --create
```

Show recent release history:

```sh
flopha log --limit 10
```

For custom tag names like `mobile@1.2.3`, use a pattern such as `mobile@{semver}`. See the command reference below or the [official docs](https://sjquant.github.io/flopha) for release branches, changelogs, pre-releases, and CI examples.

## CLI Commands

### NextVersion

Calculates and displays the next version based on the latest matching tag or branch.
Aliases: `nv`

#### Options

- `-i`, `--increment <major|minor|patch>`: Explicit bump level. Default: `patch`.

- `--auto`: Auto-detect the bump level from commit messages since the last version. Built-in conventional commit behavior is:

  - `BREAKING CHANGE` or `BREAKING-CHANGE` -> `major`
  - `<type>!:` or `<type>(<scope>)!:` -> `major`
  - `feat:` or `feat(scope):` -> `minor`
  - anything else -> `patch`

- `--rule <LEVEL:PATTERN>`: Define custom bump rules used with `--auto`. Repeatable. When any `--rule` flags are provided, they replace the built-in conventional commit rules entirely.

- `-p`, `--pattern <PATTERN>`: Specify a custom pattern for version matching and generation. Use placeholders `{major}`, `{minor}`, and `{patch}`, or `{semver}` as shorthand for `{major}.{minor}.{patch}`. Example patterns:

  - `v{major}.{minor}.{patch}`
  - `mobile@{semver}`
  - `release-{major}.{minor}.{patch}`

- `--pre <CHANNEL>`: Format the next version as a pre-release on the given channel. Example: `--pre alpha` produces `v1.2.3-alpha.1`.

- `-s`, `--source <tag|branch>`: Read versions from tags or branches. Default: `tag`.

- `-c`, `--create`: Create the next tag or branch in Git.

- `--tag-message <MESSAGE>`: Create an annotated tag with the given message. Requires `--create` and tag source.

- `--push`: Push the created tag or branch to `origin`. Requires `--create`.

- `-f`, `--format <text|json>`: Output format. Default: `text`.

### LastVersion

Retrieves and displays the most recent version tag or branch in the repository that matches a specified pattern.
Aliases: `lv`

#### Options

- `-p`, `--pattern <PATTERN>`: Get the last version based on a given pattern (e.g., `v{major}.{minor}.{patch}` or `v{semver}`).

- `-s`, `--source <tag|branch>`: Read versions from tags or branches. Default: `tag`.

- `-c`, `--checkout`: Check out the last matching version.

- `-f`, `--format <text|json>`: Output format. Default: `text`.

### Log

Shows matching versions. In tag mode, it also includes tag dates and the number of commits between releases.
Aliases: `lg`

#### Options

- `-p`, `--pattern <PATTERN>`: Filter versions by a pattern such as `v{major}.{minor}.{patch}` or `v{semver}`.

- `-s`, `--source <tag|branch>`: Read versions from tags or branches. Default: `tag`.

  Tag mode provides full timeline metadata. Branch mode still lists matching versions, but tag dates and commit counts are not available.

- `-n`, `--limit <LIMIT>`: Limit the number of versions shown.

- `-f`, `--format <text|json>`: Output format. Default: `text`.

### Changelog

Generates a changelog from commits since the last matching version or a specific starting tag.
Aliases: `cl`

#### Options

- `--from <TAG>`: Tag to start from. Defaults to the latest version matching `--pattern`.

- `--to <VERSION>`: Upper bound for the changelog. The tag must exist when limiting commit history. Also available as `{to}` in `--title`.

- `-p`, `--pattern <PATTERN>`: Match a custom version format. `{semver}` expands to `{major}.{minor}.{patch}`.

- `-s`, `--source <tag|branch>`: Read versions from tags or branches when resolving the default `--from`. Default: `tag`.

- `--group <TITLE:PATTERN>`: Custom changelog group rule. Repeatable. When any group is provided, it replaces the built-in defaults.

- `--other <TITLE>`: Title for commits that match no group. Pass an empty string to suppress unmatched commits. Default: `Other Changes`.

- `--title <TEMPLATE>`: Heading template. Use `{from}` and `{to}` placeholders.

- `-o`, `--output <FILE>`: Write to a file instead of stdout. Existing content is prepended by default.

- `--overwrite`: Replace the output file instead of prepending.

- `-f`, `--format <text|json>`: Output format. Default: `text`.

Default changelog groups:

| Section | Matched commits |
|---|---|
| Breaking Changes | `BREAKING CHANGE`, `BREAKING-CHANGE`, or `type!:` |
| Features | `feat:` or `feat(scope):` |
| Bug Fixes | `fix:` or `fix(scope):` |
| Other Changes | Everything else |

### Release

Runs the full release pipeline in one command, driven by a checked-in `flopha.toml`: compute the version bump, sync it into manifest files (`Cargo.toml`, `package.json`, `pyproject.toml`, or arbitrary regex targets), commit, create an annotated tag, push, generate a changelog, and create a GitHub Release.
Aliases: `rel`

#### Options

- `-c`, `--config <FILE>`: Path to the config file. Default: `flopha.toml`.

- `--dry-run`: Print the release plan (bump, manifest files that would be touched, changelog preview) without making any changes.

- `-f`, `--format <text|json>`: Output format for `--dry-run`. Default: `text`.

#### flopha.toml

```toml
[version]
pattern = "v{major}.{minor}.{patch}"   # default
auto = true                            # detect bump level from conventional commits (default)
# increment = "patch"                  # used instead of auto-detection when auto = false
# rules = ["major:BREAKING CHANGE", "minor:^feat"]
# pre = "beta"                         # pre-release channel, e.g. v1.2.3-beta.1
# tag_message = "Release {tag}"        # annotated tag message; defaults to "Release <tag>"

[changelog]
enabled = true
# groups = ["Breaking Changes:BREAKING CHANGE", "Features:^feat"]
# other = "Other Changes"
# title = "Changes in {to}"

[release]
create = true
draft = false
# title = "{tag}"                     # supports {tag} and {version}
# body = "..."                        # falls back to the generated changelog, then generate_notes
# generate_notes = false
# repo = "owner/repo"                 # defaults to parsing the `origin` remote

[[manifest]]
path = "Cargo.toml"
type = "cargo"                        # sets [package].version

[[manifest]]
path = "package.json"
type = "npm"                          # sets the top-level "version" field

[[manifest]]
path = "pyproject.toml"
type = "pyproject"                    # sets [project].version, falling back to [tool.poetry].version

[[manifest]]
path = "docs/VERSION"
type = "regex"
pattern = "(?m)^version=.*$"
replacement = "version={version}"     # {version} is substituted with the new version
```

`version.source = "branch"` is not supported by `release` — manifest sync, commits, and GitHub Releases all assume tag-based versioning.

`type = "npm"` re-serializes `package.json` as pretty-printed JSON (2-space indent), which normalizes whitespace if the file used a different style. `type = "cargo"` and `type = "pyproject"` edit the TOML document in place instead, preserving comments and formatting exactly.

Manifest sync requires an attached branch (not a detached HEAD) whenever `[[manifest]]` targets are configured, since the version-bump commit needs somewhere to push to. This is the default in most CI checkouts of a branch, but tools that check out a bare SHA (a detached HEAD) will need to check out a branch first.

### Global Options

- `-v`, `--verbose`: Enable verbose output for detailed information.

## Default Behavior

- The CLI fetches from the `origin` remote before resolving versions.
- The default version pattern is `v{major}.{minor}.{patch}`. Use `v{semver}` as a shorter equivalent.

## Contributing

Install dev tools and activate git hooks after cloning:

```sh
cargo install prek
prek install
prek install --hook-type commit-msg
```

Hooks enforce formatting, linting, secret detection, and [gitmoji](https://gitmoji.dev) commit messages.

## License

flopha is released under the [MIT License](LICENSE).

# flopha

flopha is a CLI for semantic versioning and Git release workflows. It helps teams calculate the next version, manage Git tags and release branches, generate pre-release versions, and automate version bumps from conventional commits.

Use flopha when you want a lightweight release management tool for Git repositories without wiring up a larger release pipeline.

## GitHub Action

Auto-tag the next semantic version and optionally create a GitHub Release — one line of YAML.

```yaml
- uses: sjquant/flopha@v1
  with:
    create-release: true
```

Requires `permissions: contents: write` in the calling workflow.

### Inputs

| Input | Default | Description |
|---|---|---|
| `auto` | `true` | Detect bump level from conventional commits: `feat`→minor, `feat!`/`BREAKING CHANGE`→major, anything else→patch. |
| `increment` | `patch` | Bump level when `auto: false`: `major`, `minor`, or `patch`. |
| `pattern` | `v{major}.{minor}.{patch}` | Tag pattern. Use `{major}`, `{minor}`, `{patch}` as placeholders. |
| `pre` | | Pre-release channel: `alpha`, `beta`, `rc`, etc. Produces tags like `v1.2.3-rc.1`. |
| `major-pattern` | | Regex that marks a commit as a major bump. Replaces built-in defaults when set (see note below). |
| `minor-pattern` | | Regex that marks a commit as a minor bump. Replaces built-in defaults when set (see note below). |
| `rule` | | Custom bump rules, one per line, as `level:regex`. Overrides `major-pattern`, `minor-pattern`, and built-in defaults. |
| `create-release` | `false` | Create a GitHub Release for the new tag. |
| `draft` | `false` | Create the release as a draft. |
| `release-title` | tag name | Title for the GitHub Release. |
| `release-body` | | Body text for the release. Takes precedence over `generate-release-notes`. |
| `generate-release-notes` | `false` | Auto-generate release notes from commits (GitHub API). |
| `dry-run` | `false` | Compute and print the next tag without creating or pushing anything. |
| `flopha-version` | `latest` | Pin the flopha binary version, e.g. `v0.3.0`. |
| `github-token` | `github.token` | Token used to push the tag and create the release. |

> **Note on `major-pattern` / `minor-pattern`:** supplying either one (or both) replaces the built-in conventional-commit defaults entirely. Levels not covered by a pattern fall through to patch. Use `rule` for full control over all levels at once.

### Outputs

| Output | Description |
|---|---|
| `tag` | The version tag created (or would-be, on dry-run), e.g. `v1.3.0`. |
| `version` | Bare version number without prefix, e.g. `1.3.0`. |
| `release-url` | URL of the GitHub Release. Empty when `create-release: false` or `dry-run: true`. |

### Examples

**Minimal — tag only:**

```yaml
permissions:
  contents: write

steps:
  - uses: actions/checkout@v4
    with:
      fetch-depth: 0
  - uses: sjquant/flopha@v1
```

**Tag + GitHub Release:**

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
      generate-release-notes: true
```

**Pre-release on non-main branches:**

```yaml
- uses: sjquant/flopha@v1
  with:
    pre: ${{ github.ref_name != 'main' && 'rc' || '' }}
    create-release: true
```

**Custom bump patterns (non-conventional-commit style):**

```yaml
- uses: sjquant/flopha@v1
  with:
    major-pattern: '\[major\]'
    minor-pattern: '\[minor\]'
```

**Full custom rules:**

```yaml
- uses: sjquant/flopha@v1
  with:
    rule: |
      major:BREAKING CHANGE
      minor:^feat
    create-release: true
    draft: true
```

**Dry-run (safe for PRs):**

```yaml
- uses: sjquant/flopha@v1
  id: next
  with:
    dry-run: true
- run: echo "Next tag will be ${{ steps.next.outputs.tag }}"
```

**Monorepo / custom tag pattern:**

```yaml
- uses: sjquant/flopha@v1
  with:
    pattern: 'app@{major}.{minor}.{patch}'
```

---

## Install

Shell (Mac, Linux):

```
curl -fsSL https://raw.githubusercontent.com/sjquant/flopha/main/scripts/install.sh | sh
```

## Features

- Semantic versioning CLI for Git tags and release branches
- Conventional commit auto-detection for major, minor, and patch bumps
- Pre-release channel support for `alpha`, `beta`, and `rc` style versions
- Custom version patterns for app releases, desktop builds, and monorepo naming
- Version history output for release auditing and changelog workflows

## Quickstart

1.  Get the last version:

    ```
    flopha last-version
    ```

2.  Check out to the last version:

    ```
    flopha last-version --checkout
    ```

3.  Calculate the next version:

    ```
    flopha next-version
    ```

4.  Use a custom version pattern:

    ```
    flopha next-version --pattern "desktop@{major}.{minor}.{patch}"
    ```

5.  Auto-detect the next bump from conventional commits:

    ```
    flopha next-version --auto
    ```

6.  Override auto-detection with custom rules:

    ```
    flopha next-version --auto --rule 'major:BREAKING CHANGE' --rule 'minor:^feat'
    ```

7.  Preview a pre-release version:

    ```
    flopha next-version --pre rc
    ```

8.  Create a new version tag:

    ```
    flopha next-version --pattern "desktop@{major}.{minor}.{patch}" --create
    ```

9.  Increment major version:

    ```
    flopha next-version --increment major
    ```

10.  Use branch-based versioning:

    ```
    flopha next-version --source branch
    ```

11.  Create a new version branch:

    ```
    flopha next-version --pattern "release/{major}.{minor}.{patch}" --source branch --create
    ```

12.  Show version history:

    ```
    flopha log --limit 10
    ```

## CLI Commands

### NextVersion

Calculates and displays the next version based on the current version in the repository.
Aliases: `nv`

#### Options

- `-i`, `--increment <INCREMENT>`: Specify the version part to increment. Options are:

  - `major`
  - `minor`
  - `patch`

  Default: `patch`

- `--auto`: Auto-detect the bump level from commit messages since the last tag. This currently works with tag-based versioning. Built-in conventional commit behavior is:

  - `BREAKING CHANGE` or `BREAKING-CHANGE` -> `major`
  - `<type>!:` or `<type>(<scope>)!:` -> `major`
  - `feat:` or `feat(scope):` -> `minor`
  - anything else -> `patch`

- `--rule <LEVEL:PATTERN>`: Define custom bump rules used with `--auto`. Repeatable. When any `--rule` flags are provided, they replace the built-in conventional commit rules entirely.

- `-p`, `--pattern <PATTERN>`: Specify a custom pattern for version matching and generation. Use placeholders `{major}`, `{minor}`, and `{patch}`. Example patterns:

  - `v{major}.{minor}.{patch}`
  - `release-{major}.{minor}.{patch}`

- `--pre <CHANNEL>`: Format the next version as a pre-release on the given channel. Example: `--pre alpha` produces `v1.2.3-alpha.1`.

- `-s`, `--source <SOURCE>`: Specify the source for versioning. Options are:

  - `tag` (default)
  - `branch`

- `-c`, `--create`: Create the next tag or branch in Git.

### LastVersion

Retrieves and displays the most recent version tag or branch in the repository that matches a specified pattern.
Aliases: `lv`

#### Options

- `-p`, `--pattern <PATTERN>`: Get the last version based on a given pattern (e.g., `v{major}.{minor}.{patch}`).

- `-s`, `--source <SOURCE>`: Specify the source for versioning. Options are:

  - `tag` (default)
  - `branch`

- `-c`, `--checkout`: Check out the last matching version.

### Log

Shows matching versions. In tag mode, it also includes tag dates and the number of commits between releases.
Aliases: `lg`

#### Options

- `-p`, `--pattern <PATTERN>`: Filter versions by a pattern such as `v{major}.{minor}.{patch}`.

- `-s`, `--source <SOURCE>`: Specify the source for versioning. Options are:

  - `tag` (default)
  - `branch`

  Tag mode provides full timeline metadata. Branch mode still lists matching versions, but tag dates and commit counts are not available.

- `-n`, `--limit <LIMIT>`: Limit the number of versions shown.

### Global Options

- `-v`, `--verbose`: Enable verbose output for detailed information.

## License

flopha is released under the [MIT License](LICENSE).

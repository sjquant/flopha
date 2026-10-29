# flopha GitHub Action

Auto-tag the next semantic version and optionally create a GitHub Release.

```yaml
- uses: sjquant/flopha@v1
  with:
    create-release: true
    changelog: true
```

Requires `permissions: contents: write` and a checkout with `fetch-depth: 0` so flopha can see tags and history.

## When to use

The action treats **Git tags as the version source**: it tags and releases without changing any files. If the version also lives in `Cargo.toml`, `package.json`, or `pyproject.toml`, run [`flopha release`](https://flopha.solaqua.dev/docs/release-workflows#flopha-release-in-github-actions) with a `flopha.toml` instead; it writes the new version into those files before tagging.

## Inputs

| Input | Default | Description |
|---|---|---|
| `auto` | `true` | Detect bump level from conventional commits: `feat`→minor, `feat!`/`BREAKING CHANGE`→major, anything else→patch. |
| `increment` | `patch` | Bump level when `auto: false`: `major`, `minor`, or `patch`. |
| `pattern` | `v{major}.{minor}.{patch}` | Tag pattern. Use `{major}`, `{minor}`, `{patch}` as placeholders, or `{semver}` as shorthand. |
| `pre` | | Pre-release channel: `alpha`, `beta`, `rc`, etc. Produces tags like `v1.2.3-rc.1`. |
| `rule` | | Custom bump rules, one per line, as `level:regex`. Replaces built-in conventional-commit defaults entirely. |
| `changelog` | `false` | Generate a changelog since the previous tag. Exposed as the `changelog` output, and used as the release body when `create-release` is true and `release-body` is empty. |
| `changelog-groups` | | Custom changelog groups, one per line, as `TITLE:REGEX`. Replaces the built-in defaults entirely. |
| `changelog-other` | `Other Changes` | Title for commits that match no group. |
| `changelog-suppress-other` | `false` | Omit commits that match no group. |
| `create-release` | `false` | Create a GitHub Release for the new tag. |
| `draft` | `false` | Create the release as a draft. |
| `release-title` | tag name | Title for the GitHub Release. |
| `release-body` | | Body text for the release. Takes precedence over `generate-release-notes`. |
| `generate-release-notes` | `false` | Auto-generate release notes from commits (GitHub API). |
| `dry-run` | `false` | Compute and print the next tag without creating or pushing anything. |
| `flopha-version` | `latest` | Pin the flopha binary version, e.g. `v0.3.0`. |
| `github-token` | `github.token` | Token used to push the tag and create the release. |


## Outputs

| Output | Description |
|---|---|
| `tag` | The version tag created (or would-be, on dry-run), e.g. `v1.3.0`. |
| `version` | Bare version number without prefix, e.g. `1.3.0`. |
| `release-url` | URL of the GitHub Release. Empty when `create-release: false` or `dry-run: true`. |
| `changelog` | Generated changelog (Markdown). Empty when `changelog: false`. |

## Examples

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

**Tag + GitHub Release with a changelog:**

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

**Pre-release on non-main branches:**

```yaml
- uses: sjquant/flopha@v1
  with:
    pre: ${{ github.ref_name != 'main' && 'rc' || '' }}
    create-release: true
```

**Custom bump rules:**

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
    pattern: 'app@{semver}'
```

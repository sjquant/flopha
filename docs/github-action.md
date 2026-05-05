# flopha GitHub Action

Auto-tag the next semantic version and optionally create a GitHub Release.

```yaml
- uses: sjquant/flopha@v1
  with:
    create-release: true
```

Requires `permissions: contents: write` in the calling workflow.

## Inputs

| Input | Default | Description |
|---|---|---|
| `auto` | `true` | Detect bump level from conventional commits: `feat`→minor, `feat!`/`BREAKING CHANGE`→major, anything else→patch. |
| `increment` | `patch` | Bump level when `auto: false`: `major`, `minor`, or `patch`. |
| `pattern` | `v{major}.{minor}.{patch}` | Tag pattern. Use `{major}`, `{minor}`, `{patch}` as placeholders. |
| `pre` | | Pre-release channel: `alpha`, `beta`, `rc`, etc. Produces tags like `v1.2.3-rc.1`. |
| `major-pattern` | | Regex for a major bump. Replaces built-in defaults when set (see note). |
| `minor-pattern` | | Regex for a minor bump. Replaces built-in defaults when set (see note). |
| `rule` | | Custom bump rules, one per line, as `level:regex`. Overrides `major-pattern`, `minor-pattern`, and built-in defaults. |
| `create-release` | `false` | Create a GitHub Release for the new tag. |
| `draft` | `false` | Create the release as a draft. |
| `release-title` | tag name | Title for the GitHub Release. |
| `release-body` | | Body text for the release. Takes precedence over `generate-release-notes`. |
| `generate-release-notes` | `false` | Auto-generate release notes from commits (GitHub API). |
| `dry-run` | `false` | Compute and print the next tag without creating or pushing anything. |
| `flopha-version` | `latest` | Pin the flopha binary version, e.g. `v0.3.0`. |
| `github-token` | `github.token` | Token used to push the tag and create the release. |

> **Note on `major-pattern` / `minor-pattern`:** supplying either one replaces the built-in conventional-commit defaults entirely. Levels not covered fall through to patch. Use `rule` for full control.

## Outputs

| Output | Description |
|---|---|
| `tag` | The version tag created (or would-be, on dry-run), e.g. `v1.3.0`. |
| `version` | Bare version number without prefix, e.g. `1.3.0`. |
| `release-url` | URL of the GitHub Release. Empty when `create-release: false` or `dry-run: true`. |

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

**Custom bump patterns:**

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

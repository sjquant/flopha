# flopha GitHub Action

Auto-tag the next semantic version and optionally create a GitHub Release.

```yaml
- uses: sjquant/flopha@v1
  with:
    create-release: true
```

Requires `permissions: contents: write` in the calling workflow.

## When to use this action

flopha treats **Git tags as the authoritative version source**. It reads existing tags, computes the next one from commit history, and creates the new tag.

This fits projects where:
- The version lives entirely in Git tags (no version file to update)
- You want commit-message-driven bumps with no manual version management

**It is not the right tool** when the version is stored in a file (`Cargo.toml`, `package.json`, `pyproject.toml`, etc.) and that file is the source of truth. In that pattern, the typical workflow is:

1. Developer bumps the version in the file and opens a PR
2. On merge, CI reads the file version, compares it to the latest release tag, and creates the release if the version changed

Example for a Cargo project:

```yaml
name: release
on:
  push:
    branches: [main]

jobs:
  release:
    runs-on: ubuntu-latest
    permissions:
      contents: write
    steps:
      - uses: actions/checkout@v4

      - name: Check version change
        id: check
        run: |
          latest=$(curl -s https://api.github.com/repos/${{ github.repository }}/releases/latest \
            | jq -r '.tag_name // "v0.0.0"' | tr -d v)
          cargo=$(grep -m1 '^version' Cargo.toml | awk -F'"' '{print $2}')
          echo "changed=$([[ "$latest" != "$cargo" ]] && echo true || echo false)" >> "$GITHUB_OUTPUT"
          echo "version=$cargo" >> "$GITHUB_OUTPUT"

      - name: Create release
        if: steps.check.outputs.changed == 'true'
        run: gh release create "v${{ steps.check.outputs.version }}" --generate-notes
        env:
          GH_TOKEN: ${{ github.token }}
```

flopha itself follows this pattern — its version lives in `Cargo.toml`, so it cannot dogfood its own action.

## Inputs

| Input | Default | Description |
|---|---|---|
| `auto` | `true` | Detect bump level from conventional commits: `feat`→minor, `feat!`/`BREAKING CHANGE`→major, anything else→patch. |
| `increment` | `patch` | Bump level when `auto: false`: `major`, `minor`, or `patch`. |
| `pattern` | `v{major}.{minor}.{patch}` | Tag pattern. Use `{major}`, `{minor}`, `{patch}` as placeholders. |
| `pre` | | Pre-release channel: `alpha`, `beta`, `rc`, etc. Produces tags like `v1.2.3-rc.1`. |
| `rule` | | Custom bump rules, one per line, as `level:regex`. Replaces built-in conventional-commit defaults entirely. |
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
    pattern: 'app@{major}.{minor}.{patch}'
```

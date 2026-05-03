#!/usr/bin/env bash
set -euo pipefail

# Ensure flopha installed by the action is on PATH regardless of whether
# $GITHUB_PATH propagation between composite-action steps is reliable.
export PATH="${HOME}/.flopha/bin:${PATH}"

# ── git identity ────────────────────────────────────────────────────────────
git config --local user.name  "github-actions[bot]"
git config --local user.email "github-actions[bot]@users.noreply.github.com"

# ── build flopha args ────────────────────────────────────────────────────────
ARGS=()
if [ "${INPUT_AUTO:-true}" = "true" ]; then
  ARGS+=(--auto)
  [ -n "${INPUT_INCREMENT:-}" ] && echo "Note: 'increment' is ignored when 'auto: true'"
else
  ARGS+=(--increment "${INPUT_INCREMENT:-patch}")
fi
ARGS+=(--pattern "${INPUT_PATTERN:-v{major}.{minor}.{patch}}")
[ -n "${INPUT_PRE:-}" ]  && ARGS+=(--pre "$INPUT_PRE")

# rule > major/minor-pattern > built-in conventional-commit defaults
if [ -n "${INPUT_RULE:-}" ]; then
  while IFS= read -r rule; do
    [ -n "$rule" ] && ARGS+=(--rule "$rule")
  done <<< "$INPUT_RULE"
elif [ -n "${INPUT_MAJOR_PATTERN:-}" ] || [ -n "${INPUT_MINOR_PATTERN:-}" ]; then
  [ -n "${INPUT_MAJOR_PATTERN:-}" ] && ARGS+=(--rule "major:${INPUT_MAJOR_PATTERN}")
  [ -n "${INPUT_MINOR_PATTERN:-}" ] && ARGS+=(--rule "minor:${INPUT_MINOR_PATTERN}")
fi

# ── dry-run: compute only, no side effects ───────────────────────────────────
if [ "${INPUT_DRY_RUN:-false}" = "true" ]; then
  NEW_TAG=$(flopha next-version "${ARGS[@]}")
  echo "tag=$NEW_TAG"         >> "$GITHUB_OUTPUT"
  echo "release-url="         >> "$GITHUB_OUTPUT"
  echo "Dry run — next tag would be: $NEW_TAG"
  exit 0
fi

# ── create and push the version tag ─────────────────────────────────────────
NEW_TAG=$(flopha next-version "${ARGS[@]}" --create)

if ! PUSH_OUT=$(git push origin "$NEW_TAG" 2>&1); then
  git tag -d "$NEW_TAG" 2>/dev/null || true
  echo "::error::Failed to push tag '$NEW_TAG': $PUSH_OUT"
  echo "::error::Make sure the calling workflow has 'permissions: contents: write'."
  exit 1
fi

echo "tag=$NEW_TAG" >> "$GITHUB_OUTPUT"
echo "Created and pushed tag: $NEW_TAG"

# ── optionally create a GitHub Release ──────────────────────────────────────
if [ "${INPUT_CREATE_RELEASE:-false}" != "true" ]; then
  echo "release-url=" >> "$GITHUB_OUTPUT"
  exit 0
fi

RELEASE_ARGS=("$NEW_TAG")
RELEASE_ARGS+=(--title "${INPUT_RELEASE_TITLE:-$NEW_TAG}")

[ "${INPUT_DRAFT:-false}"    = "true" ] && RELEASE_ARGS+=(--draft)
[ -n "${INPUT_PRE:-}" ]                 && RELEASE_ARGS+=(--prerelease)

# --notes and --generate-notes are mutually exclusive in gh CLI
if [ -n "${INPUT_RELEASE_BODY:-}" ]; then
  RELEASE_ARGS+=(--notes "$INPUT_RELEASE_BODY")
elif [ "${INPUT_GENERATE_RELEASE_NOTES:-true}" = "true" ]; then
  RELEASE_ARGS+=(--generate-notes)
fi

if ! RELEASE_OUT=$(gh release create "${RELEASE_ARGS[@]}" --json url --jq '.url' 2>&1); then
  echo "::error::Failed to create GitHub Release for tag '$NEW_TAG': $RELEASE_OUT"
  echo "::error::Make sure the calling workflow has 'permissions: contents: write'."
  exit 1
fi

RELEASE_URL="$RELEASE_OUT"
echo "release-url=$RELEASE_URL" >> "$GITHUB_OUTPUT"
echo "Created GitHub Release: $RELEASE_URL"

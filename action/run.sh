#!/usr/bin/env bash
set -euo pipefail

# Ensure flopha installed by the action is on PATH regardless of whether
# $GITHUB_PATH propagation between composite-action steps is reliable.
export PATH="${HOME}/.flopha/bin:${PATH}"

# ── git identity ────────────────────────────────────────────────────────────
git config --local user.name  "github-actions[bot]"
git config --local user.email "github-actions[bot]@users.noreply.github.com"

# ── build flopha args ────────────────────────────────────────────────────────
# All INPUT_* vars are guaranteed set by action.yml defaults — do NOT use
# ${VAR:-default} for values containing {}, as bash finds the first } and
# closes the expansion early, corrupting the value.
ARGS=()
if [ "$INPUT_AUTO" = "true" ]; then
  ARGS+=(--auto)
  [ -n "$INPUT_INCREMENT" ] && echo "Note: 'increment' is ignored when 'auto: true'"
else
  ARGS+=(--increment "$INPUT_INCREMENT")
fi
ARGS+=(--pattern "$INPUT_PATTERN")
[ -n "$INPUT_PRE" ] && ARGS+=(--pre "$INPUT_PRE")

if [ -n "$INPUT_RULE" ]; then
  while IFS= read -r rule; do
    [ -n "$rule" ] && ARGS+=(--rule "$rule")
  done <<< "$INPUT_RULE"
fi

# ── dry-run: compute only, no side effects ───────────────────────────────────
if [ "$INPUT_DRY_RUN" = "true" ]; then
  NEW_TAG=$(flopha next-version "${ARGS[@]}")
  echo "tag=$NEW_TAG"         >> "$GITHUB_OUTPUT"
  echo "release-url="         >> "$GITHUB_OUTPUT"
  echo "changelog="           >> "$GITHUB_OUTPUT"
  echo "Dry run — next tag would be: $NEW_TAG"
  exit 0
fi

# ── capture previous tag before creating the new one ─────────────────────────
PREV_TAG=$(flopha last-version --pattern "$INPUT_PATTERN" --format json | jq -r '.version // empty')

# ── create and push the version tag ─────────────────────────────────────────
if ! NEW_TAG=$(flopha next-version "${ARGS[@]}" --create --push); then
  echo "::error::Failed to create or push tag."
  echo "::error::Make sure the calling workflow has 'permissions: contents: write'."
  exit 1
fi

echo "tag=$NEW_TAG" >> "$GITHUB_OUTPUT"
echo "Created and pushed tag: $NEW_TAG"

# ── optionally generate changelog ────────────────────────────────────────────
GENERATED_CHANGELOG=""
if [ "$INPUT_CHANGELOG" = "true" ] && [ -n "$PREV_TAG" ]; then
  CL_ARGS=(--from "$PREV_TAG" --to "$NEW_TAG")
  if [ -n "$INPUT_CHANGELOG_GROUPS" ]; then
    while IFS= read -r grp; do
      [ -n "$grp" ] && CL_ARGS+=(--group "$grp")
    done <<< "$INPUT_CHANGELOG_GROUPS"
  fi
  if [ "$INPUT_CHANGELOG_SUPPRESS_OTHER" = "true" ]; then
    CL_ARGS+=(--other "")
  elif [ -n "$INPUT_CHANGELOG_OTHER" ]; then
    CL_ARGS+=(--other "$INPUT_CHANGELOG_OTHER")
  fi
  GENERATED_CHANGELOG=$(flopha changelog "${CL_ARGS[@]}")
fi
# Emit changelog output using a randomised delimiter so commit message content
# cannot prematurely close the multiline GitHub Actions output block.
CL_EOF="__FLOPHA_CL_${RANDOM}${RANDOM}__"
{
  echo "changelog<<${CL_EOF}"
  printf '%s\n' "$GENERATED_CHANGELOG"
  echo "${CL_EOF}"
} >> "$GITHUB_OUTPUT"

# ── optionally create a GitHub Release ──────────────────────────────────────
if [ "$INPUT_CREATE_RELEASE" != "true" ]; then
  echo "release-url=" >> "$GITHUB_OUTPUT"
  exit 0
fi

RELEASE_ARGS=("$NEW_TAG")
RELEASE_ARGS+=(--title "${INPUT_RELEASE_TITLE:-$NEW_TAG}")

[ "$INPUT_DRAFT" = "true" ] && RELEASE_ARGS+=(--draft)
[ -n "$INPUT_PRE" ]         && RELEASE_ARGS+=(--prerelease)

# --notes and --generate-notes are mutually exclusive in gh CLI
# Priority: release-body > generated changelog > generate-notes
if [ -n "$INPUT_RELEASE_BODY" ]; then
  RELEASE_ARGS+=(--notes "$INPUT_RELEASE_BODY")
elif [ -n "$GENERATED_CHANGELOG" ]; then
  RELEASE_ARGS+=(--notes "$GENERATED_CHANGELOG")
elif [ "$INPUT_GENERATE_RELEASE_NOTES" = "true" ]; then
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

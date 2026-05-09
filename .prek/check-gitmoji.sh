#!/usr/bin/env bash
# Validates that the commit message starts with an official gitmoji emoji.
# See https://gitmoji.dev for the full list.
set -euo pipefail

# Skip Git-generated messages
git rev-parse -q --verify MERGE_HEAD       >/dev/null 2>&1 && exit 0
git rev-parse -q --verify CHERRY_PICK_HEAD >/dev/null 2>&1 && exit 0

first_line=$(head -1 "$1")
case "$first_line" in squash!*|fixup!*) exit 0 ;; esac

# Strip variation selectors (U+FE0E = \xef\xb8\x8e, U+FE0F = \xef\xb8\x8f)
normalized=$(printf '%s' "$first_line" | sed 's/\xef\xb8\x8e//g; s/\xef\xb8\x8f//g')

# Official gitmoji — https://gitmoji.dev — must be followed by a space
PATTERN="^(🎨|⚡|🔥|🐛|🚑|✨|📝|🚀|💄|🎉|✅|🔒|🔐|🔖|🚨|🚧|💚|⬇|⬆|📌|\
👷|📈|♻|➕|➖|🔧|🔨|🌐|✏|💩|⏪|🔀|📦|👽|🚚|📄|💥|🍱|♿|💡|\
🍻|💬|🗃|🔊|🔇|👥|🚸|🏗|📱|🤡|🥚|🙈|📸|⚗|🔍|🏷|🌱|🚩|🥅|💫|\
🗑|🛂|🩹|🧐|⚰|🧪|👔|🩺|🧱|🧑‍💻|💸|🧵|🦺|✈|🦖) "

if printf '%s' "$normalized" | grep -qE "$PATTERN"; then
    exit 0
fi

echo "ERROR: Commit message must start with a gitmoji emoji followed by a space." >&2
echo "  Example: ✨ Add new feature" >&2
echo "  Full list: https://gitmoji.dev" >&2
exit 1

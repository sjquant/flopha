#!/usr/bin/env bash
# Validates that the commit message starts with an official gitmoji emoji.
# See https://gitmoji.dev for the full list.
set -euo pipefail

# Skip Git-generated commit messages
git rev-parse -q --verify MERGE_HEAD      >/dev/null 2>&1 && exit 0
git rev-parse -q --verify CHERRY_PICK_HEAD >/dev/null 2>&1 && exit 0

first_line=$(head -1 "$1")

case "$first_line" in squash!*|fixup!*) exit 0 ;; esac

if ! command -v python3 >/dev/null 2>&1; then
  echo "WARNING: python3 not found, skipping gitmoji check" >&2
  exit 0
fi

if python3 - "$first_line" <<'PYEOF'
import sys, re

# Official gitmoji set — https://gitmoji.dev (variation selectors stripped on comparison)
GITMOJI = {
    '🎨','⚡','🔥','🐛','🚑','✨','📝','🚀','💄','🎉',
    '✅','🔒','🔐','🔖','🚨','🚧','💚','⬇','⬆','📌',
    '👷','📈','♻','➕','➖','🔧','🔨','🌐','✏','💩',
    '⏪','🔀','📦','👽','🚚','📄','💥','🍱','♿','💡',
    '🍻','💬','🗃','🔊','🔇','👥','🚸','🏗','📱','🤡',
    '🥚','🙈','📸','⚗','🔍','🏷','🌱','🚩','🥅','💫',
    '🗑','🛂','🩹','🧐','⚰','🧪','👔','🩺','🧱',
    '🧑‍💻','💸','🧵','🦺','✈','🦖',
}

strip_vs = re.compile(r'[︎️]')

def normalize(s):
    return strip_vs.sub('', s)

line = sys.argv[1]
norm = normalize(line)
if any(norm.startswith(normalize(e)) for e in GITMOJI):
    sys.exit(0)

sys.exit(1)
PYEOF
then
    exit 0
fi

echo "ERROR: Commit message must start with a gitmoji emoji." >&2
echo "  Example: ✨ Add new feature" >&2
echo "  Full list: https://gitmoji.dev" >&2
exit 1

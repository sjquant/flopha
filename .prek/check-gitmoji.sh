#!/usr/bin/env bash
# Validates that the commit message starts with a gitmoji (emoji or :alias: format).
set -euo pipefail

if ! command -v python3 >/dev/null 2>&1; then
  echo "WARNING: python3 not found, skipping gitmoji check" >&2
  exit 0
fi

first_line=$(head -1 "$1")

if python3 -c "
import sys, re
line = sys.argv[1]
# U+1F300-U+1FAFF: misc symbols, emoticons, transport, activities, objects, etc.
# U+2190-U+2BFF:  arrows, math operators, misc technical, dingbats, misc symbols
if re.match(r'^(:[a-z0-9_]+:|[\U0001F300-\U0001FAFF←-⯿])', line):
    sys.exit(0)
sys.exit(1)
" "$first_line"; then
    exit 0
fi

echo "ERROR: Commit message must start with a gitmoji." >&2
echo "  Emoji:  ✨ Add new feature" >&2
echo "  Alias:  :sparkles: Add new feature" >&2
exit 1

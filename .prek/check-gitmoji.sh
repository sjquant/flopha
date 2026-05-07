#!/usr/bin/env bash
# Validates that the commit message starts with a gitmoji (emoji or :alias: format).
set -e

first_line=$(head -1 "$1")

if python3 -c "
import sys, re
line = sys.argv[1]
if re.match(r'^(:[a-z0-9_]+:|[^\x00-\x7f])', line):
    sys.exit(0)
sys.exit(1)
" "$first_line"; then
    exit 0
fi

echo "ERROR: Commit message must start with a gitmoji." >&2
echo "  Emoji:  ✨ Add new feature" >&2
echo "  Alias:  :sparkles: Add new feature" >&2
exit 1

#!/bin/sh
# Copies .agents/skills to .claude/skills. With --check, only reports a difference.
set -eu
root=$(cd "$(dirname "$0")/.." && pwd)
if [ "${1:-}" = "--check" ]; then
  diff -r "$root/.agents/skills" "$root/.claude/skills" >/dev/null || {
    echo ".claude/skills differs from .agents/skills; run scripts/sync-skills.sh" >&2
    exit 1
  }
  exit 0
fi
mkdir -p "$root/.claude"
rm -rf "$root/.claude/skills"
cp -R "$root/.agents/skills" "$root/.claude/skills"

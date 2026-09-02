#!/bin/bash
# Undoes tools/stage-sessions.sh: kills the held processes, removes the fixture,
# and puts the config back exactly as it was — including removing it if it had
# not existed.
set -euo pipefail

FIXTURE="${TMPDIR:-/tmp}/agent-pet-fixture"
CONFIG="$HOME/.config/agent-pet/config.json"

if [ -d "$FIXTURE/sessions" ]; then
  for f in "$FIXTURE"/sessions/*.json; do
    [ -e "$f" ] || continue
    pid="$(basename "$f" .json)"
    kill "$pid" 2>/dev/null && echo "killed $pid" || true
  done
fi
rm -rf "$FIXTURE"

if [ -f "$CONFIG.before-staging" ]; then
  mv "$CONFIG.before-staging" "$CONFIG"
  echo "restored your config"
else
  rm -f "$CONFIG"
  echo "removed the staging config (it had not existed before)"
fi
echo "unstaged."

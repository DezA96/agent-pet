#!/bin/bash
# Stages four fake Claude sessions so the pet's states can be seen on demand.
#
# Each is anchored to a real `sleep` process and that process's real procStart,
# so the liveness rule is satisfied rather than bypassed — the pet is being shown
# real evidence about fake sessions, not talked into ignoring its own rule.
#
# Development aid, not shipped. Undo with tools/unstage-sessions.sh.
set -euo pipefail

FIXTURE="${TMPDIR:-/tmp}/agent-pet-fixture"
CONFIG="$HOME/.config/agent-pet/config.json"

rm -rf "$FIXTURE"
mkdir -p "$FIXTURE/sessions"

# One session: a held PID, a registry file anchored to it, and a transcript.
stage() {
  local name="$1" status="$2" extra="$3" transcript="$4"
  local cwd="/Users/staged/$name"
  # Detached, so the held process outlives this script rather than being hung up
  # with it — a session that quietly dies mid-check is worse than no fixture.
  nohup sleep 100000 >/dev/null 2>&1 &
  disown 2>/dev/null || true
  local pid=$!
  local start
  start="$(TZ=UTC ps -o lstart= -p "$pid" | sed 's/^ *//;s/ *$//')"
  cat > "$FIXTURE/sessions/$pid.json" <<JSON
{"pid":$pid,"sessionId":"staged-$name","cwd":"$cwd","procStart":"$start","entrypoint":"cli","status":"$status"$extra}
JSON
  local slug="${cwd//[^a-zA-Z0-9]/-}"
  mkdir -p "$FIXTURE/projects/$slug"
  [ -n "$transcript" ] && printf '%s\n' "$transcript" > "$FIXTURE/projects/$slug/staged-$name.jsonl"
  echo "  $name: pid $pid"
}

echo "staging into $FIXTURE"
stage working busy '' \
  '{"message":{"content":[{"type":"tool_use","name":"Bash","input":{"description":"Counting to a large number"}}]}}'
stage idle idle '' ''
stage waiting waiting ',"waitingFor":"input needed"' ''
# Errored needs both halves: the newest entry is an API error AND the status is
# not busy. An error the agent retried through is not an errored session.
stage errored idle '' \
  '{"type":"assistant","isApiErrorMessage":true,"apiErrorStatus":529,"error":"server_error","message":{"role":"assistant","content":[{"type":"text","text":"API Error: 529 Overloaded."}]}}'

mkdir -p "$(dirname "$CONFIG")"
[ -f "$CONFIG" ] && cp "$CONFIG" "$CONFIG.before-staging" && echo "saved your config to $CONFIG.before-staging"
cat > "$CONFIG" <<JSON
{"watchDirectories": ["$FIXTURE"]}
JSON

echo
echo "staged. the pet should show four rows within two seconds."
echo "kill one at a time to step the creature down the priority list:"
echo "  kill <pid>     # errored -> waiting -> working -> idle -> asleep"

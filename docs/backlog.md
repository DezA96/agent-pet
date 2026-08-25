# Candidate Backlog

Candidates are possibilities, not promises. Use `Target Release` only after an item is planned; a blank target means no commitment.

Statuses: Candidate → Planned → In Progress → Released (or → Rejected; → Parked while a missing prerequisite blocks planning)

| ID | Candidate | Evidence | Source | Status | Target Release |
|---|---|---|---|---|---|
| C-001 | Floating always-on-top pet window (macOS, transparent, small) | Core value in charter | charter harvest | In Progress | 001 |
| C-002 | Claude Code CLI integration (reads activity/status via what the agent exposes) | Day-one agent constraint | charter harvest | In Progress | 001 |
| C-003 | Codex CLI integration | Day-one agent constraint | charter harvest | In Progress | 001 |
| C-004 | Short "what the agent is working on" status line beside the pet | Goal: state = short activity status | charter harvest | In Progress | 001 |
| C-005 | The pet itself — a single pet anchors the surface, and its appearance visibly changes when an agent needs attention (waiting / error) | Goal: glanceable state | charter harvest | Planned | 001 |
| C-006 | One status row per live session, each session distinguishable | Charter goal: concurrent sessions each with their own clear signal | charter harvest (corrected at scope 001; pet moved to C-005 at implement round) | In Progress | 001 |
| C-007 | Sound or attention cue when an agent flips to waiting-for-input or error | Developer idea | charter harvest | Candidate | — |
| C-008 | Click pet to focus that session's terminal window/tab | Developer idea | charter harvest | Candidate | — |
| C-009 | Menu-bar / status-bar variant of the status | Developer idea | charter harvest | Candidate | — |
| C-010 | Adjustable pet size | Goal: stays out of the way | charter harvest | Candidate | — |
| C-011 | Toggle: pet visible with status line hidden | Goal: stays out of the way | charter harvest | Candidate | — |
| C-012 | Live-session discovery — show only agent sessions actually running, drop stale ones | Both agents leave transcripts on disk after a session ends; naive discovery would show stale pets | scope 001 | In Progress | 001 |
| C-013 | Pet expression summarizes the most urgent state across all live sessions (error > waiting > working) | Saves scanning rows when many sessions are live; cut from 001 as redundant with per-row state | scope 001 | Candidate | — |
| C-014 | Add-directory picker — user adds agent directories to watch via a folder chooser | Sessions live under multiple profile directories (observed: ~/.claude and a second, non-default profile directory); defaults alone can't cover a third | spec round, story 001 | Planned | 001 |
| C-015 | Persist profile directories the pet learned from live processes, instead of relearning each tick | Learned directories are in-memory only in 001; persisting them survives a restart with no session running | design round, story 001 | Candidate | — |
| C-016 | Pet window is placeable and controllable — drag to move, position remembered, menu bar icon for show/hide and quit | Pet is unusable fixed at top-right; with clicks no longer passing through it must be movable, and an accessory app with no dock icon has no visible way to be closed | build, story 001 | In Progress | 001 |
| C-017 | Status age counts from when the status actually changed (`statusUpdatedAt`), not from when the pet first observed it | Claude Code writes `statusUpdatedAt` on every registry file and the adapter never reads it; observed live — a session idle 96 minutes displayed as 0s after relaunch | spec round, story 002 | Planned | 001 |
| C-018 | Per-agent icon or colour on each session row, instead of the agent's name as text | Glanceability is the release goal and an icon reads faster than a word in a narrow row; deferred because it needs the pet to hold a table of agent names, which the agent-agnostic premise forbids — the honest form is an adapter supplying its own display token | spec round, story 003 | Candidate | — |

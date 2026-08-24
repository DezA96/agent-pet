# Story: Live Claude Code session status on a floating pet surface

## Release
001 — Glanceable Agent Status ([plan](../releases/001-glanceable-agent-status.md))

## Status
Done

## User Outcome
As the developer, I want every currently running Claude Code CLI session to appear on a small always-visible
surface with a very short line saying what it is doing right now, so that I can tell what my agents
are up to without switching to the terminal.

## Acceptance Criteria
- Given one or more Claude Code CLI sessions are running, when the pet is running, then each live
  session is shown as its own row, whatever window is focused and with no terminal visible.
- Given a session row is shown, then it identifies which project the session belongs to, and remains
  distinguishable from another session even when both report the same derived name.
- Given a session is working, when its row is shown, then the row shows a very short description of
  its current activity, taken from the agent's own wording where the agent supplied one.
- Given a session is not working, when its row is shown, then the row shows `idle`.
- Given a session's working state cannot be read, when its row is shown, then the state is shown as
  unknown; it is never reported as `idle` or working by inference.
- Given a status is displayed, then the row shows a count-up timer of seconds since that status was
  observed, so its age is always visible without a tuned staleness threshold.
- Given a session stops running, when the pet next refreshes, then its row disappears within a few
  seconds; sessions that are not running are never shown.
- Given a session was force-killed and its registry file remains, when discovery runs, then no row is
  produced for it. A session counts as live only when all three hold: the registry file exists, the
  PID it names is running, and that process's actual start time matches the file's recorded
  `procStart`.
- Given sessions exist under more than one profile directory, when discovery runs, then live sessions
  from every configured directory are shown.
- Given the pet starts with no configuration, then it watches the default Claude Code and Codex
  directories, and the watched directory list lives in the pet's own config file.
- Given a watched directory holds sessions the pet cannot yet interpret (Codex, this release), then
  they are ignored silently and no error is surfaced.
- Given discovery itself fails, then the pet shows an error state, distinct from the empty state shown
  when no agents are running.
- Given no sessions are running, then the pet shows an empty state rather than disappearing or erroring.
- Given the pet is running, then it never takes keyboard focus — including when it is clicked.
- Given the pet is clicked, then the click does not reach the window beneath it.
  (Amended mid-build. This criterion originally required the opposite, that clicks pass through. Real
  use showed that made the pet an invisible trap over controls the user cannot see: a click landed on
  a window's close button and ended a live session. Passing clicks through is also incompatible with
  moving the pet, which cannot be dragged if it ignores the mouse.)
- Given the pet is running, then no agent activity, prompt, or code leaves the machine.
- Given the pet is running, then it creates and modifies no file owned by Claude Code or any other
  agent; all observation is read-only.
- Given a session's displayed status is spot-checked against the real agent, then it matches that
  agent's actual activity within a few seconds.

## Excluded From This Change
- Codex CLI integration (C-003) — separate story, same release. Its directory is watched, its sessions
  are not yet rendered.
- Visually distinct attention states (C-005) — `idle`, working and unknown are distinguished by text
  only; no animation or colour semantics yet.
- Distinguishing "blocked awaiting permission" from plain `idle`.
- The add-directory folder picker — follow-up story in this release (C-014); this story ships defaults
  plus a hand-editable config file.
- Sessions launched by the VS Code extension. Its process publishes no status and no transcript
  (see Edge Cases); release 001 targets the CLI, as its plan states.
- Any control of the agent from the pet (charter non-goal — read-only).
- Any scrollback, transcript, or history view (charter non-goal).
- Automatic installation of anything into agent configuration.

## Edge Cases
- A session force-killed (SIGKILL) cannot delete its own registry file; the leftover must not produce
  a row.
- PID reuse: a recycled PID must not be mistaken for the original session — hence the `procStart` check.
- Sessions are split across profile directories (observed: `~/.claude` for a VS Code-launched process,
  a custom `CLAUDE_CONFIG_DIR` for three CLI sessions). Scanning only one profile silently loses live sessions.
- Two sessions in the same project both derived the name `agent-agnostic-pet-02`; the displayed name is
  not a unique identifier.
- The VS Code extension process (`--output-format stream-json`) registers once, never publishes
  `status`, writes no transcript, and does not appear in peer enumeration — a live process that is not
  an observable session.
- A session that has just started has no tool activity yet; its row must render without an activity line.
- A click-through surface sits over controls the user cannot see; forwarding the click can trigger a
  destructive action such as closing a window.
- Status fidelity varies by entrypoint: `cli` sessions publish `status` with a live `statusUpdatedAt`;
  others may publish neither.

## Data or API Changes
- None persisted about agents. Live state is held in memory only.
- The pet owns one config file holding the watched-directory list. No schema shared with any agent.

## Testing Notes
- Automated: the three-part liveness rule against fixtures covering an orphaned registry file, a reused
  PID with a mismatched `procStart`, and a healthy session.
- Automated: activity-text derivation from captured transcript lines, including the missing-description
  fallback and a transcript with no tool activity yet.
- Automated: discovery across two configured directories at once, and a configured directory that is
  absent or unreadable.
- Manual: two real Claude Code CLI sessions in different projects — both rows appear with correct
  project attribution and activity matching the real agent within a few seconds.
- Manual: let a session go idle and confirm the row flips to `idle`.
- Manual: quit one session; its row disappears within a few seconds.
- Manual: `kill -9` a session; no stale row survives.
- Manual: click the pet and confirm the click does not reach the window beneath, and that focus stays
  wherever it was.

Verified so far:
- Automated: 40 tests in `core/` cover the three-part liveness rule (orphan, recycled PID, healthy),
  activity derivation including the fallback and a transcript with no tool activity, and discovery
  across two directories with one absent. Run with
  `cargo test --manifest-path core/Cargo.toml`.
- Real sessions: three live CLI sessions across two profile directories rendered with correct project
  attribution, and the activity line matched the agent's actual current command.
- Real `kill -9`: the row disappeared within ~5 s with the registry file still on disk; separately, the
  liveness rule was exercised against the real process table for the healthy, mismatched-`procStart`
  and killed cases.
- Idle: two real sessions rendered as `idle`.
- Error state: a malformed config produced `sessions unreadable`, distinct from the empty state, and
  the pet recovered when the file was removed.
- Read-only and local: no sockets held, no writable file descriptors, no write/create/remove calls
  outside tests.

- Empty state: confirmed by the developer with every session stopped — the pet showed
  `No agents running`.
- Count-up timer: two captures six seconds apart read `5s` then `11s` on an unchanged status, with no
  reset. Previously it restarted every discovery tick; four tests now pin the behaviour, including that
  a changed state or a changed activity line does restart the count.

- Clicking the pet: confirmed by the developer after the click behaviour was reversed — the click stops
  at the pet and does not reach the window beneath.

## Design Requirement
- Technical design required — platform choice and the agent-adapter seam are cross-cutting and
  expensive to reverse. Link: [docs/design-docs/001-observation-channel-and-pet-surface.md](../design-docs/001-observation-channel-and-pet-surface.md) — Accepted.
- Hard constraint: the pet observes agents without modifying any agent-owned configuration or file.
- Observation channel (settled by investigation): per-profile registry file `<profile>/sessions/<pid>.json`
  for discovery, project, liveness and `status: busy|idle`; per-session transcript
  `<profile>/projects/<cwd-slug>/<sessionId>.jsonl` tailed for the activity description. Both are files
  the agent already writes; nothing is hooked, injected, or configured.
- Rejected during investigation, with reasons: agent hook configuration (modifies agent-owned settings);
  the per-process IPC socket (undocumented, Claude-specific, and unnecessary once the registry file was
  found to publish status); terminal/PTY capture (no single tap point across VS Code's integrated
  terminal and plain shells, and fragile TUI parsing); macOS Accessibility API (permission-gated,
  breaks when windows are hidden); process-tree inspection (misses in-process tools, no idle signal).
- Settled by the design doc: portable Rust observation core behind a one-function JSON FFI, with a
  Swift + AppKit `NSPanel` frontend on macOS; adapters own liveness so the pet needs no change when an
  agent is added; profile directories are re-derived every tick from defaults, config, and the
  `CLAUDE_CONFIG_DIR` of every live `claude` process; config at `~/.config/agent-pet/config.json`;
  polling (not filesystem watching) because a force-killed process changes no file on disk.

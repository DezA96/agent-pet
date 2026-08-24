# Release 001: Glanceable Agent Status

Charter: [docs/project-charter.md](../project-charter.md)

## Status
In Progress

## Release Goal
From any focused window, a single glance shows every live Claude Code CLI and Codex CLI session on the machine — a very short status of what each is currently working on, plus whether it is waiting for input or has errored.

## Target Users
The developer (solo), the charter's sole target user, on macOS. No other audience in this release.

## Planned Work
- **C-001 Floating always-on-top pet surface** — small, transparent, visible over any window, never steals focus.
- **C-002 Claude Code CLI integration** — live session state read from what Claude Code already exposes locally.
- **C-003 Codex CLI integration** — the same outcome through a mechanically different agent; this is what proves the seam.
- **C-004 Short activity status line** — a very short description of what each session is currently working on.
- **C-005 Attention states visually distinct** — waiting-for-input and errored are tellable from working at a glance, without reading text.
- **C-006 One pet, one status row per live session** — a single pet anchors the surface; each live session gets its own row (agent, project, current activity), so concurrent sessions each carry their own clear signal and are distinguishable from one another.
- **C-012 Live-session discovery** — only sessions actually running are shown; ended and stale sessions disappear.

## Release Acceptance Criteria
- With no terminal visible — whatever window is focused, or none at all — every currently running Claude Code and Codex session is represented on screen.
- For each represented session, a very short status of its current activity is readable without interacting with the pet.
- A waiting-for-input session and an errored session are each tellable from a working session at a glance, without reading the status text.
- With two or more sessions running at once, each has its own status row, and which agent and which project it belongs to is identifiable without interacting with the pet.
- A session that ends stops being shown within a few seconds; sessions that are not running are never shown.
- Spot-checked against the real agent, a session's displayed status matches its actual activity within a few seconds.
- The pet never takes keyboard focus and never blocks interaction with the window beneath it.
- Adding Codex required no change to the pet itself — the change is confined to its own integration.
- No agent activity, prompt, or code leaves the machine.

## Success Measures
- The developer leaves it running all day, every day, for a week, rather than closing it as noise.
- Over that week, the developer stops switching to the terminal purely to check what an agent is doing.
- CPU and battery impact are not noticeable in normal use.

## Important Risks
- **Story breakdown unknown — C-002 and C-003.** Each agent integration spans session discovery, event ingestion, and mapping to a displayed status, and is visibly larger than one story. C-004's per-agent mapping may fold into them at refinement. The remaining selected items look story-sized, but refinement decides. Splitting happens there, not here; if an item proves far bigger than assumed, it returns to this plan as an explicit scope change.
- **The two agents are mechanically asymmetric.** Claude Code exposes structured hooks (`PreToolUse`, `PostToolUse`, `PostToolUseFailure`, `Stop`, `StopFailure`, `PermissionRequest`). Codex has no free hook slot — `notify` is single-slot and already occupied by another tool — so it must be read from its rollout transcript at `~/.codex/sessions/**/rollout-*.jsonl`. The adapter boundary has to absorb push-vs-tail. Reduced by building Claude Code first, then Codex within the same release, while the boundary is still cheap to move.
- **Liveness is unproven.** Both agents leave transcripts on disk long after a session ends, so naive discovery would paint a row of stale pets. Reduced by making "no stale pets" a release acceptance criterion, forcing the definition to be settled during the release rather than assumed.
- **Error detection may be weaker for Codex.** Claude Code signals failure explicitly; Codex's transcript may not. If Codex errors prove not reliably detectable, that is an explicit scope change, not a silent gap.
- **Shrink lever, if the release overruns:** coarsen the activity line, or drop animated states to static distinct ones. Concurrency and the second agent are *not* available as cuts — both are expensive to retrofit and both are load-bearing for the goal.

## Release Strategy
Single-user local release: the developer runs it on their own machine, no rollout stages. Rollback is quitting the pet and going back to tabbing — nothing persists that needs undoing, and no agent's behavior depends on it running.

## Explicit Scope Changes
- **Added C-014 add-directory picker** (spec round, story 001). Investigation found live sessions split
  across multiple profile directories (`~/.claude` and a custom `CLAUDE_CONFIG_DIR` on the development machine), so a fixed default
  list cannot cover every case. Chosen deliberately as a conscious expansion over folding the picker
  into the first story, to keep the walking skeleton to one question. This story ships defaults plus a
  hand-editable config file; the picker is UI over that same config.
- **VS Code-launched sessions confirmed out of scope** (spec round, story 001). Not a scope reduction:
  the plan always named the *CLI*. Recorded because investigation established the reason — the VS Code
  extension process publishes no status, writes no transcript, and does not appear in peer enumeration.
- **Push-vs-tail risk retired** (spec round, story 001). The plan assumed Claude Code would need hooks
  while Codex needed transcript tailing. Claude Code's session registry file and transcript are both
  readable on disk in real time, so both agents are read the same way and the adapter seam no longer
  has to absorb an asymmetry.

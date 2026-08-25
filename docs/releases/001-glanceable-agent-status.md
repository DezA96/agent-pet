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
- With no terminal visible — whatever window is focused, or none at all — every currently running Claude Code and Codex session that has taken at least one turn is represented on screen. (Amended during story 003 — see Explicit Scope Changes.)
- For each represented session, a very short status of its current activity is readable without interacting with the pet.
- A waiting-for-input session and an errored session are each tellable from a working session at a glance, without reading the status text.
- With two or more sessions running at once, each has its own status row, and which agent and which project it belongs to is identifiable without interacting with the pet.
- A session that ends stops being shown within a few seconds; sessions that are not running are never shown.
- Spot-checked against the real agent, a session's displayed status matches its actual activity within a few seconds.
- The pet never takes keyboard focus, including when it is clicked, and a click on the pet does not reach the window beneath it. (Amended during story 001 — see Explicit Scope Changes.)
- Adding Codex required no change to the pet itself — the change is confined to its own integration.
- No agent activity, prompt, or code leaves the machine.

## Success Measures
- The developer leaves it running all day, every day, for a week, rather than closing it as noise.
- Over that week, the developer stops switching to the terminal purely to check what an agent is doing.
- CPU and battery impact are not noticeable in normal use.

## Important Risks
- **Story breakdown unknown — C-002 and C-003.** Each agent integration spans session discovery, event ingestion, and mapping to a displayed status, and is visibly larger than one story. C-004's per-agent mapping may fold into them at refinement. The remaining selected items look story-sized, but refinement decides. Splitting happens there, not here; if an item proves far bigger than assumed, it returns to this plan as an explicit scope change.
- **The two agents are mechanically asymmetric.** ~~Claude Code exposes structured hooks. Codex has no free hook slot — `notify` is single-slot and already occupied by another tool — so it must be read from its rollout transcript. The adapter boundary has to absorb push-vs-tail.~~ **Corrected (design round, story 001):** the stated reason was wrong on both halves. `notify` *is* occupied (`~/.codex/config.toml` → another tool's `turn-ended` hook), but `~/.codex/hooks.json` exists with eight array-valued hook types (`SessionStart`, `UserPromptSubmit`, `PreToolUse`, `PostToolUse`, `PermissionRequest`, `SubagentStart`, `SubagentStop`, `Stop`), so a hook slot is free. Hooks are still not used, on the stronger ground that they would modify agent-owned configuration, which the story forbids outright. The real asymmetry is different and larger: Claude Code publishes a PID-anchored registry file per session (`<profile>/sessions/<pid>.json`) carrying `status`, while Codex writes only date-partitioned rollout transcripts (`~/.codex/sessions/YYYY/MM/DD/rollout-*.jsonl`) that **record no PID anywhere**. Liveness therefore cannot be a pet-level rule; it lives inside each adapter. See [design doc](../design-docs/001-observation-channel-and-pet-surface.md).
- **Liveness is unproven.** Both agents leave transcripts on disk long after a session ends, so naive discovery would paint a row of stale pets. Reduced by making "no stale pets" a release acceptance criterion, forcing the definition to be settled during the release rather than assumed. **Retired (spike, story 003).** Both halves are now proven, not assumed: Claude via its PID-anchored registry file (story 001), Codex via an open `lsof` handle on its own rollout, held continuously for the life of the session (533 consecutive samples, no gap). Neither agent's liveness rests on file recency, so no stale row can be produced by either.
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
- **Codex hook risk corrected** (design round, story 001). Recorded above: `~/.codex/hooks.json` does
  have free slots, so the plan's original reason for avoiding Codex hooks was factually wrong. The
  decision stands on the no-agent-owned-writes constraint instead. The genuine asymmetry — Codex
  rollout files carry no PID — is what shaped the adapter seam.
- **Click-through reversed as an urgent correctness fix** (build, story 001). Story 001 originally
  required clicks to pass through the pet to the window beneath. In real use this made the pet an
  invisible trap: a click landed on a window's close button that the pet was covering and ended a live
  agent session. The criterion was amended mid-build — the pet still never takes keyboard focus, but it
  now catches its own clicks. Taken under the urgent-correctness exception rather than backlogged,
  because the pet was already in daily use and the failure destroys work.
- **Added C-016 placeable and controllable pet window** (build, story 001). Conscious expansion, not a
  silent one: it delays the release. Two reasons, both from real use — the pet fixed at the top-right
  covers whatever is there, and now that clicks no longer pass through it must be movable to be usable
  at all; and an accessory app with no dock icon had no visible way to be closed or hidden, only
  `pkill`. Kept as its own story rather than folded into 001, which is already built and verified
  against its own criteria; splitting belongs at refinement, not build.
- **First-turn criterion amended** (spec round, story 003). The criterion above said *every* currently
  running session is represented. A Codex CLI session that has taken no turn yet writes no rollout file
  and no database row — confirmed twice, and today's session went 27 seconds before its file existed —
  so it is observable only as a bare process with no id, no project record and nothing to say. The
  criterion now reads "that has taken at least one turn". Amended rather than quietly reinterpreted:
  closing the release against a criterion knowingly not met is the thing to avoid. Showing such a
  process as an `Unknown` row was the alternative and was rejected, because distinguishing it from
  ChatGPT.app's `codex … app-server` — which is also a live process named `codex` holding no rollout —
  buys a row that says nothing.
- **Push-vs-tail risk retired** (spec round, story 001). The plan assumed Claude Code would need hooks
  while Codex needed transcript tailing. Claude Code's session registry file and transcript are both
  readable on disk in real time, so both agents are read the same way and the adapter seam no longer
  has to absorb an asymmetry.

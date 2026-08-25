# Story: Codex CLI integration

## Release
001 — Glanceable Agent Status ([plan](../releases/001-glanceable-agent-status.md))

## Status
Implemented

## User Outcome
As the developer, using more than one coding agent, I want every currently running Codex CLI session to
appear on the pet beside my Claude Code sessions, so that a single glance covers both agents rather than one of them.

## Acceptance Criteria
- Given Codex CLI sessions are running and each has taken at least one turn, when the pet is running,
  then each appears as its own row beside any Claude rows, whatever window is focused.
- Given any session row is shown, then which agent it belongs to is identifiable without interacting
  with the pet, by the agent's name in the row's text.
- Given a Codex row is shown, then it identifies the project from the rollout's `cwd`, and stays
  distinguishable from another row that derives the same name.
- Given a session's most recent turn boundary is `task_started`, then its state is working and the row
  shows a very short description of its current activity.
- Given the most recent turn boundary is `task_complete`, then the row shows `idle` and no activity line.
- Given the rollout cannot be read, or holds no turn boundary within the window read, then the state is
  unknown — never inferred as idle or working.
- Given a Codex process exits, when the pet next refreshes, then its row disappears within a few seconds.
- Given a rollout file exists on disk that no running process holds open, then no row is ever produced
  for it.
- Given a live process holds a `guardian` subagent rollout open alongside its own, then exactly one row
  is produced.
- Given ChatGPT.app's `codex … app-server` is running, then no row is produced for it.
- Given a Desktop / `source='vscode'` rollout is held open by a live process, then no row is produced.
- Given a live Codex session has taken no turn yet, then no row is produced.
- Given the pet first sees a rollout of 74 MB, then the poll completes without a visible stall, and
  state and activity reflect the end of the file.
- Given the core observes Codex, then no file under `~/.codex` is created, modified or deleted, and
  `~/.codex/state_5.sqlite` is never opened.
- Given Codex discovery fails, then the poll still succeeds and Claude rows are unaffected.
- Given the Codex adapter is added, then no file under `macos/` changes except the generic agent-name
  render, and no Swift source names any agent.
- Given `./test.sh` is run, then it passes, and the Rust tests pass with
  `cargo test --manifest-path core/Cargo.toml`.

## Excluded From This Change
- **A live session that has taken no turn yet.** It writes no rollout file, so nothing is shown until
  its first turn — settled in the spec round. The honest reading is that such a process is not yet a
  session: it has no id, no project record and nothing to say. Consequence for the release criterion
  "every currently running Codex session is represented on screen" is recorded in the release plan.
- **Attention states — waiting-for-input and errored (C-005).** No rollout on disk contains any error
  or approval event type, across 8 rollouts and 3 CLI versions, so there is nothing to read yet. This
  story ships `Working | Idle | Unknown`, the same states story 001 gave Claude.
- **Codex subagent threads** (`thread_source='subagent'`, e.g. `guardian`). A live process holds these
  rollouts open alongside its own, and they are filtered out rather than rendered as sessions.
- **Codex Desktop / `source='vscode'` threads.** The release targets the CLI.
- **Reading `~/.codex/state_5.sqlite`.** Nothing the pet needs is only in the database.

## Edge Cases
Gathered during the spike, to be carried into the spec round:
- A CLI session that has taken no turn yet writes **no** rollout file and **no** `threads` row. It is a
  live process that is not yet an observable session — the Codex analogue of story 001's VS Code case,
  except here the session is genuinely invisible rather than merely statusless.
- Codex subagent threads (`thread_source='subagent'`, e.g. `guardian`) get their own rollout files and
  their own `threads` rows. Observed on this machine in both CLI and desktop sessions.
- `Codex Desktop` / `source='vscode'` threads share the same directories and the same database as CLI
  threads. The release targets the CLI, so they have to be filtered out rather than rendered.
- Rollout files reach 74 MB (observed). Whatever produces the activity line cannot re-read the file on
  every tick the way story 001's Claude transcript reader does.
- A process named exactly `codex` is not necessarily a CLI session: the ChatGPT desktop app runs
  `/Applications/ChatGPT.app/…/codex … app-server` with cwd `/`. Discovery must not match on process
  name alone.
- The per-process lock at `~/.codex/tmp/arg0/codex-arg0XXXX/.lock` outlives its process, so it carries
  the same staleness trap as an orphaned Claude registry file and is not a liveness signal.

## Data or API Changes
No persisted format changes and no migration: the pet stores nothing about Codex, and everything is
re-derived each tick. Three internal contracts move:
- **`ProcessTable` gains two generic methods** — roughly `pids_of_command(name)` and `open_paths(pids)`
  — implemented over `ps` and `lsof`, and mirrored on `FakeProcessTable`. No agent name enters the trait.
- **`AgentSession.agent_id` starts carrying `"codex"`.** The field already exists and is already
  serialized; nothing about the payload's shape changes.
- **The Swift row renders `agentId`**, which `Core.swift:18` already decodes and currently discards.
  Rendered generically — no Swift source names an agent.

## Testing Notes
- **Real rollout fixtures, reduced and committed.** Three, because they are the three shapes the filter
  must tell apart: a CLI user session (`source='cli'`, `thread_source='user'`), a `guardian` subagent
  (`thread_source='subagent'`), and a Desktop thread (`source='vscode'`). Reduced to `session_meta`,
  `task_started`, `item_completed` and `task_complete` lines with captured command output scrubbed —
  ~20–30 KB each, down from 1.3 MB. Story 001 committed no fixtures and hand-built its lines; that was
  right for Claude's small `tool_use` blocks and is wrong here, where a hand-written fixture would only
  prove the parser agrees with its author about shapes Codex has never been checked to emit.
- **Synthetic lines only for conditions that cannot be captured on demand**: a mid-write truncated
  final line, the 9-starts-to-8-completes mismatch observed on disk, and an oversized file exercising
  the bounded backward read.
- **The no-flood invariant is asserted, not assumed**: one process's set of open rollout handles yields
  at most one row. Today's live session held two rollouts open — its own and a `guardian` subagent's —
  so a format change that ever let a subagent through must break a test rather than silently double
  every row on the pet. Deliberately not enforced at runtime by capping rows per PID: that would hide
  the fault instead of surfacing it.
- **Liveness against fixtures, not against the machine.** The fake `ProcessTable` supplies both the
  named PIDs and their open paths, so the join is tested without a Codex session running.
- **What the fixtures actually landed as** (build). Two corrections to the estimate above.
  `formatted_output` holds a second copy of every command's captured output — 44 KB on a
  single line — and had to be scrubbed alongside `stdout`; without it the "reduced" CLI
  rollout was still 349 KB and the Desktop one 4.2 MB. And the Desktop fixture is now
  **line 1 plus turn boundaries only**: the `source='vscode'` filter stops at
  `session_meta`, so its item lines were never parsed by any test, while carrying an
  unrelated client project's name and document paths into this repository. Final sizes
  are 22 KB, 17 KB and 1.4 KB. The CLI fixture is kept whole because it is the one a test
  parses end to end.
- `./test.sh` passes, and `cargo test --manifest-path core/Cargo.toml` passes.
- **Manual verification** in a real session, since no fixture proves the join itself: a live `codex`
  TUI appears as a row on its first turn, its activity line tracks what the session is doing, the row
  disappears within a few seconds of quitting it, and neither ChatGPT.app's `app-server` nor a
  `guardian` subagent ever produces a row.

## Design Requirement
**No separate design needed.** This is the second implementation of an adapter seam that design doc
001 already settled and documents; every decision below is recorded either here or in the spike's
`Decision`, and a design doc would restate them in a second place free to drift. Design doc 001's
"Agent-adapter seam" section is amended instead, because that is where a future reader looks. No ADR:
it fails the first gate — nothing here is expensive to reverse. The spike lands here per
`lifecycle.md` step 6, below.

### Implementation notes settled in the spec round
- **One story, not two.** The spike removed the liveness reason to split, and discovery, turn state and
  the activity line together are the vertical slice that tests "adding Codex required no change to the
  pet itself" — which is the whole reason the second agent is in this release.
- **Turn state: last boundary event wins.** `event_msg/task_started` last → `Working`;
  `event_msg/task_complete` last → `Idle`. Not pairing or counting: one rollout on disk carries 9
  starts to 8 completes, so any counting rule is already wrong on real data. Accepted limitation — a
  turn the user interrupts leaves `task_started` last, so the row reads `Working` while the session sits
  idle. No timeout is invented for it; story 001's count-up age timer exists precisely so a wrong status
  ages in plain sight rather than hiding behind a tuned staleness threshold.
- **First sight of a rollout reads backward from EOF over a bounded window (~256 KB), then tails
  forward** as `claude::transcript::Tailer` already does. Reading from offset 0 is what story 001 does
  (`core/src/claude/transcript.rs:31`) and it is safe at Claude's ~102 KB; a Codex rollout on this disk
  is 74 MB and one single-turn session today reached 1.3 MB, so a whole-file read would stall the poll
  for seconds at pet startup. Starting at bare EOF was rejected as worse: a session already mid-turn
  would show `Unknown` until its next event, breaking the release's within-a-few-seconds spot-check.
  This is the only genuinely new machinery in the story.
- **Corrected during live verification: a fixed window cannot find a working session's
  boundary.** The note above sized the window at ~256 KB, on the observation that a
  rollout's last turn boundary sits near its end. That holds only for a *settled*
  session: `task_complete` lands 214 to 4,799 bytes from EOF across every rollout on this
  disk, because it is written as the turn ends. A session still **working** has
  `task_started` as its last boundary, and that is as far back as the turn is long — the
  first live session tested read 987 KB, and the widest gap between two boundaries on
  this disk is 22.8 MB. The pet showed `unknown` for the one session it most exists to
  show. First sight now falls back to scanning backward for a boundary in 4 MB chunks
  when the window holds none, rejecting the enormous `item_completed` lines with a
  substring test before any JSON parse: 2 ms on the 70 MB rollout, ~15 ms at the 22.8 MB
  worst case, against 130 ms for a whole cold poll. Past 64 MB the state is honestly
  `unknown` and the next tick picks up the boundary as it is written.
- **Session identity comes from line 1 of the rollout, never from `~/.codex/state_5.sqlite`.** Every
  rollout opens with a `session_meta` line carrying `cwd`, `id`, `source`, `thread_source`,
  `originator` and `cli_version` — verified identical in shape across 0.143.0, 0.147.0 and
  0.148.0-alpha, CLI and Desktop. That file is already being opened for state and activity, so line 1
  is free; the database is WAL-mode and held open by the live process, would add a SQLite dependency,
  and has plainly churned (36 columns, most added by `ALTER`, filename already at version 5).
  `source='cli'` and `thread_source='user'` is the filter that excludes subagent and Desktop threads.
- **Activity line: newest renderable item wins, no priority table.** `Reasoning.summary_text` is the
  direct analogue of Claude's `description` — the agent's own words, and the most frequent item type
  (1016 across all rollouts), so Codex should reach derived phrasing far less often than Claude's 41%.
  Strip the surrounding `**`. Renderable: `Reasoning`, `AgentMessage`, `CommandExecution`, `FileChange`,
  `McpToolCall`, `DynamicToolCall`, `Extension`, `ImageView`. Truncated to 45 chars as story 001 does.
- **`AgentMessage` is included, first sentence only.** Checked rather than assumed: only 38 of 173
  land immediately before `task_complete`; the other 78% are mid-turn progress narration that reads
  like a status line already ("I'll inspect the available SSH host entries and test them…"). The
  turn-final ones never display anyway — `task_complete` follows them, the state becomes `Idle`, and
  story 001 already drops activity for a session that is not working.
- **`ProcessTable` gains generic primitives, not agent-named ones.** Something like
  `pids_of_command(name)` and `open_paths(pids)`, with no agent name in the trait. `claude_profile_dirs`
  is already an agent-named wart; a second one makes it the pattern, and agent knowledge leaking out of
  an agent's own module is what the adapter seam exists to prevent. `claude_profile_dirs` itself is left
  alone — it reads process environments, a different job, and rewriting it is not this story's work.
- **`codex exec` is not special-cased.** No `exec` rollout exists on this disk, so what `source` it
  writes is unknown and not worth quota to establish. The filter is the definition: whatever a live
  process holds open with `source='cli'` and `thread_source='user'` is a session. If an `exec` run
  surfaces as a row, that is correct — it is a running CLI session doing work. Recorded as untested.
- **The agent's name goes in the row's text**, rendered from whatever `agentId` carries. Not a
  per-agent icon or colour, which would need the pet to hold a table of agent names and would break
  "adding Codex required no change to the pet itself" on the very first agent added. Icons deferred as
  C-018. This closes the outstanding half of C-006, which release 001 defined as a row of *agent*,
  project and activity; story 001 could not deliver the agent half because only one agent existed.
- **The ~70 ms `lsof` cost is accepted unoptimised.** Against the existing ~20 ms `ps` call at a
  2-second interval. A capacity limit, not a correctness threat (`engineering.md`), and the release's
  own "CPU and battery impact are not noticeable" measure is what decides whether it needs work. Story
  002 parked the same call on the same grounds.

---

# Spike: Can a running Codex CLI session be joined to its own session record?

<!-- Throwaway research answering ONE uncertain question. Not production implementation. -->

## Uncertainty
Claude Code publishes `<profile>/sessions/<pid>.json`, which names its own PID and process start time,
so story 001 could prove liveness exactly. Codex publishes no such file. Investigation found two
session records — the rollout transcript and the `threads` table in `~/.codex/state_5.sqlite` — and
**neither records a PID**. So there is a set of live `codex` processes on one side, a set of session
records on the other, and no established join between them.

The open question is narrow: **does a running Codex CLI process hold its own rollout `.jsonl` open
while it works?** If it does, `lsof` supplies the join and Codex liveness is as exact as Claude's. If
it does not, every remaining join is a heuristic.

## Why It Matters
The release promises "a session that ends stops being shown within a few seconds; sessions that are
not running are never shown", and the plan already flags liveness as the release's unproven risk. With
an exact join, that promise holds for Codex as it does for Claude. Without one, the fallbacks each
give something up:

- **Process-cwd matching** — bind live `codex` PIDs to the most recently updated `threads` rows sharing
  that cwd. Never shows a dead session, but mis-attributes activity when two CLI sessions run in one
  project — the case story 001 hit twice on this machine.
- **Recency window** — treat a thread as live if `updated_at_ms` is recent. Both shows dead sessions
  briefly and hides live ones that are thinking, which breaks the criterion outright.

It also decides the story's shape. An exact join makes discovery a straightforward port of story 001's
adapter; a heuristic join makes liveness its own body of work with its own edge cases, and C-003 likely
splits at refinement — which the release plan already anticipates.

## Smallest Useful Test
Run one trivial prompt through the Codex CLI in a scratch directory. While the turn is in flight, and
again after it settles, run `lsof -p <pid>` on the `codex` process and look for a handle on the rollout
file that the turn creates.

Cost: a few cents of quota and one small rollout file added to `~/.codex/sessions`. Nothing is written
by hand into any agent-owned file — Codex writes its own file, as it does on any run.

## Success or Failure Threshold
- **Pass** — the process holds an open handle on its own rollout for the duration of the turn. `lsof`
  is then the join, and the story can promise Claude-grade liveness for Codex.
- **Partial** — the handle appears only in bursts around writes. Sampling would be racy; treat as a
  fail for a 2-second poll and record how long the handle persists.
- **Fail** — no rollout handle at any point. Fall back to process-cwd matching, and the spec's criteria
  say what happens when two CLI sessions share a project rather than pretending they cannot.

## Findings
Established before the decisive test, from free probes (no quota spent, nothing written by hand):
- The Codex CLI has run on this machine — three rollouts dated 2026-08-08 carry
  `originator: "codex-tui"`, `source: "cli"`, `cli_version` 0.143.0 and 0.147.0. The installed CLI is
  `codex-cli 0.147.0` at `~/.local/bin/codex`.
- Rollout `session_meta` carries `cwd`, `originator`, `source`, `cli_version` and thread ids, and no PID
  — confirming the release plan's corrected risk.
- `~/.codex/state_5.sqlite` holds a `threads` table indexing every session: `id`, `rollout_path`, `cwd`,
  `source`, `thread_source`, `updated_at_ms`, `title`, `preview`, `cli_version`. `source='cli'` with
  `thread_source='user'` isolates real CLI sessions from desktop and subagent threads exactly. It has
  no PID and no liveness column, so it identifies sessions but cannot prove any of them is running.
- Launching the TUI and leaving it 20 s produced no rollout file and no `threads` row; a live process
  holds the shared sqlite databases and its arg0 lock, and **no rollout handle at idle**.
- There is no per-session registry file anywhere under `~/.codex` — checked `thread-writer-locks/`
  (one global `.coordination.lock`), `state/`, `sqlite/`, `ipc/` (one shared `ipc.sock`), `tmp/arg0/`.

### Decisive test — run 2026-08-24, one TUI session in this repo

One Codex CLI TUI session (PID 35023, cwd = this working tree), one real turn, `lsof` sampled every
0.5 s from 20:41:08 to 20:47:02 across all three phases.

- **Before the first turn** — 40 consecutive samples over 27 s: process alive, cwd correct, **no rollout
  handle**. The rollout file did not exist yet.
- **From the first turn onward** — 533 consecutive samples, **no gap**, every one showing an open handle
  on the session's own rollout under `~/.codex/sessions/`.
- **Idle after the turn settled** — the handle was still held 22 s after the last write to the file. It
  is not burst-scoped to writes, which rules out the Partial threshold and its sampling race.

Three further facts the test established, all load-bearing for the spec round:

- **The rollout filename lies about creation time.** `rollout-2026-08-24T20-41-17-….jsonl` was born at
  20:41:35 — the name carries the *thread start*, while the file itself is created lazily at the first
  turn. This is why the earlier idle probe found no file, and the two timestamps must never be treated
  as the same value.
- **One live process holds several rollout handles.** It held its own `source='cli'`,
  `thread_source='user'` rollout *and* a `guardian` subagent rollout (`thread_source='subagent'`), and
  kept the subagent handle open long after that subagent stopped writing. The join yields a *set* of
  rollouts per process, not one, so the `threads` filter is what picks the session out of it.
- **The ChatGPT desktop app runs a process named exactly `codex`.**
  `/Applications/ChatGPT.app/Contents/Resources/codex … app-server`, cwd `/`, holding the shared sqlite
  databases and an arg0 lock but **no rollout handle**. The same rule that finds CLI sessions excludes
  it for free; a discovery rule matching on process name alone would not.

## Decision
**Pass.** A running Codex CLI process holds an open handle on its own rollout for as long as the session
lives, so `lsof` supplies the join the spike went looking for:

> A Codex CLI session is live iff some running `codex` process holds an open handle on a rollout file
> whose `threads` row has `source='cli'` and `thread_source='user'`.

That is liveness by the same standard story 001 gives Claude — proven from the running process, never
inferred from file recency — so the release criterion "sessions that are not running are never shown"
holds for Codex with no heuristic. Process-cwd matching and the recency window are both dropped, and
with them the two-sessions-in-one-project mis-attribution that would otherwise have had to be written
into the acceptance criteria as a known wrong answer.

Consequence for the story's shape: discovery becomes a port of story 001's adapter rather than its own
body of work, so C-003 has no liveness-driven reason to split at refinement. Whether it still splits on
size — discovery versus extracting an activity line from a rollout observed at 74 MB — is a question
for the spec round, not a settled outcome.

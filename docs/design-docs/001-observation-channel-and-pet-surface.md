# Technical Design: Observation channel and pet surface

## Status
Implemented

## Related Work
- Charter: [docs/project-charter.md](../project-charter.md)
- Release plan: [001 — Glanceable Agent Status](../releases/001-glanceable-agent-status.md)
- Story: [001 — Live Claude Code session status](../stories/001-live-claude-session-status.md)

## Design Scope
Two questions the story spec left open — the pet's platform/runtime, and how profile directories are
enumerated and defaulted — plus the agent-adapter seam that must absorb Codex without changing the pet.

The observation channel itself is settled by the spec and not re-opened here: per-profile registry file
`<profile>/sessions/<pid>.json` for discovery, project, liveness and status; per-session transcript
`<profile>/projects/<cwd-slug>/<sessionId>.jsonl` for the activity line. Both read-only.

## Requirements and Constraints
- Never takes keyboard focus; never blocks interaction with the window beneath (story AC).
- Creates and modifies no file owned by any agent; observation is strictly read-only (story AC).
- Nothing leaves the machine (charter constraint).
- Negligible CPU and battery footprint (charter constraint; release success measure).
- A session's row disappears within a few seconds of the session ending (story AC).
- Adding Codex changes only its own integration, not the pet (release AC).

## Current System
Greenfield. No source code exists; the repository holds planning artifacts only.

## Proposed Design

### Refresh strategy — SETTLED
Two clocks, both polling. No filesystem watching.

- **1 Hz UI tick** — re-renders only. Touches no I/O. The count-up-seconds criterion forces a per-second
  render regardless, so the timer costs nothing extra.
- **2 s discovery tick** — enumerates profile directories, lists registry files, runs one batched process
  check for liveness, and reads each transcript **incrementally from a remembered byte offset**.

Rationale for polling over filesystem events: **a force-killed process changes no file on disk.** An
event-driven design structurally cannot satisfy the SIGKILL edge case — liveness requires an active
process check on a timer no matter what. Given that check is unavoidable, watching adds a second
mechanism for no gain.

Transcripts must never be re-read whole: this project's own transcript is 102 KB and growing, and the
largest Codex rollout on disk is 74 MB. Seek to the last known offset, read forward, remember the new
offset.

`procStart` comparison must normalise timezones: the registry file renders it in UTC
(`"Mon Aug 24 04:16:25 2026"`) while `ps -o lstart` renders local (`Mon Aug 24 00:16:25 2026`) for the
same live PID. A naive string compare marks every live session dead.

### Platform and runtime — SETTLED
A portable observation core in Rust, and the strongest native frontend for each OS it ever runs on. On
macOS that frontend is Swift + AppKit: a borderless `NSPanel` created with `.nonactivatingPanel`, at
`.statusBar` level, with `ignoresMouseEvents = true` and
`collectionBehavior = [.canJoinAllSpaces, .stationary, .fullScreenAuxiliary]`, shipped as an `.app` with
`LSUIElement`. Two of the story's acceptance criteria — never takes keyboard focus, never blocks the
window beneath — are those exact AppKit primitives rather than an emulation of them.

The core ships as a Rust static library linked into the Swift app. Exactly one function crosses the
boundary: the core returns the current session list as a JSON string and Swift decodes it with `Codable`.
The seam is already one call (see Agent-adapter seam), so the FFI is about twenty lines on each side and
does not grow. The same JSON payload works unchanged if the core is later run as a separate process, or
paired with a Windows or GTK frontend.

Rejected: **Electron** — a Chromium runtime against a charter constraint of negligible footprint.
**Tauri** — its macOS window is an `NSWindow`, so the non-activating panel needs a native escape hatch
anyway, and the webview's advantage is styling this release has explicitly excluded. **Rust driving
AppKit directly via `objc2`** — workable and avoids the FFI, but AppKit is not idiomatic through
Objective-C message sends, and C-005's attention-state animation lands far more cheaply in Swift.

Cross-platform support is not a requirement in any current artifact (the charter, the release plan and
C-001 all say macOS). The split above is not built to satisfy a requirement; it keeps the expensive half
— the observation core — free of an OS lock-in that would be costly to undo, while the cheap half, a
window drawing a few rows of text, stays fully native per platform.

### Agent-adapter seam — SETTLED
Each agent has its own adapter. The pet asks every adapter, once per discovery tick, for the list of its
sessions that are running *right now*. Each returned session carries: which agent, which project, the
name to display, its state (`working` | `idle` | `unknown`), one short activity line, and when that was
observed. The pet renders that list and nothing else — it never learns what a process ID, a registry
file, or a JSONL transcript is.

**Liveness lives inside the adapter, not the pet.** Claude Code's test is the story's three-part rule
(registry file exists, its PID is running, that process's start time matches `procStart`). Codex cannot
use that rule: its rollout files record no PID anywhere. Had the pet owned liveness, the release
criterion "adding Codex required no change to the pet itself" would fail on the first adapter added.

Pull, not push: the pet owns the clock and asks; adapters never announce. Cadence and therefore battery
behaviour stay controlled in one place.

**Amended (spec round, story 003).** Two things the seam learned from the second adapter:

- **`ProcessTable` carries generic primitives, not agent-named ones.** Codex liveness needs the open
  file handles of a running process (`lsof`), where Claude's needs process start times (`ps`). Both are
  phrased as neutral questions — which PIDs bear a command name, which paths a PID holds open — so no
  agent name enters the trait. `claude_profile_dirs` predates this and is the counter-example: agent
  knowledge leaking out of an agent's own module is exactly what this seam exists to prevent.
- **The row renders `agentId` as text, generically.** The pet holds no table of agent names, no
  per-agent icon and no per-agent branch. An agent wanting a richer token must supply it through the
  session payload rather than have the pet learn about it (C-018).

### Profile directory enumeration — SETTLED (location of the config file still open)
There is no separate startup path. Every discovery tick rebuilds the candidate directory list from
scratch:

1. Defaults (`~/.claude`, `~/.codex`), union the entries in the pet's config file, union the
   `CLAUDE_CONFIG_DIR` value read from the environment of every `claude` process currently running.
2. For each directory, list the registry files and apply the adapter's liveness test.

Startup is simply the first tick. A session launched while the pet is already running appears within one
tick (~2 s), because step 1 re-reads live processes every tick rather than once at boot — including a
session started under a profile directory the pet has never seen before.

Reading `CLAUDE_CONFIG_DIR` off a live process is not the rejected "process-tree inspection": that was
rejected as a *status* channel. Here the process only reveals a directory; the registry file remains the
sole source of state. Verified: `ps eww -p <pid>` exposes `CLAUDE_CONFIG_DIR=/Users/.../.dev` for a live
session, and this machine's profiles are split precisely because `~/.zshrc` aliases
`dev='CLAUDE_CONFIG_DIR="$HOME/.dev" claude'`.

**Learned directories are held in memory only and never written back to the config file.** Writing would
mean the pet editing a file the user hand-maintains, and dead profile directories would accumulate with
nothing ever removing them. Re-learning costs nothing — the process check runs every tick for liveness
anyway. Persisting them is backlogged as C-015 for a later release.

### Activity line — SETTLED
Sourced from the newest `tool_use` entry in the session's transcript.

Where the agent supplied its own wording (a `description` field) that wording is used verbatim. Where it
did not, the line is composed from the tool name plus its most salient argument — `Editing config.json`,
`Reading backlog.md`, `Fetching docs.rs`. Truncated to ~45 characters on a word boundary.

The fallback is not an edge case. Across 733 tool calls in 25 transcripts from both profiles, only `Bash`
(428) and `Agent` (3) carry a `description`: 59% arrive with the agent's own sentence, 41% do not.
`Edit` (145), `Write` (52) and `Read` (42) never do, and their `file_path` is the most useful token
available for the row.

**The activity line renders only when the registry file reports `status: busy`.** The transcript's last
`tool_use` goes stale the instant a session stops working, so an idle session showing the last thing it
did would read as busy at a glance — the precise failure the pet exists to prevent. The registry
`status` is the authority; the transcript only supplies wording.

### Non-working surfaces — SETTLED
Three situations, three phrases distinct enough that none can be mistaken for another at a glance:

| Situation | Shown |
|---|---|
| A session's working state cannot be read | `state unknown` on that row |
| Discovery itself failed | `sessions unreadable`, replacing the list |
| Discovery succeeded, nothing is running | `no agents running`, replacing the list |

`state unknown` is never resolved to `idle` or working by inference. The count-up timer keeps running on
an unknown row, so a state unknown for two seconds and one unknown for four minutes are distinguishable.
Underlying error detail — which directory failed and why — goes to the log rather than onto the surface,
which is too small to carry it.

## Data Model and Migration
Nothing persisted about agents; live state is in memory only. The pet owns one config file holding the
watched-directory list. No schema is shared with any agent.

**Location: `~/.config/agent-pet/config.json`**, honouring `XDG_CONFIG_HOME` when it is set. Every agent
this tool observes keeps its own config in the home directory — `~/.claude`, `~/.codex`, a custom `CLAUDE_CONFIG_DIR` — and
the charter's sole target user is a terminal-native developer, not a general Mac user; config under
`~/Library/Application Support` would be the only thing in this ecosystem living there. JSON, decoded
through Foundation `Codable` and Rust `serde` with no added dependency. No comment support: the file
holds a list of directories, and with live-process discovery that list is usually empty.

Rejected: `~/Library/Application Support/agent-pet/` — the macOS convention, and where a general Mac user
with no docs would look, but the wrong neighbourhood for this audience. The portability argument for
`~/.config` was *not* the deciding one and is in fact weak: the genuinely portable choice is the
per-OS config directory (`%APPDATA%` on Windows), which Rust's `directories` crate returns in one call.

## Authorization and Security
Single-user local tool, no network, no trust boundary crossed. All agent-owned files are opened
read-only. The pet reads its own user's processes only. Transcripts contain prompts and code and are
never written anywhere, never logged, and never leave memory.

## Failure and Recovery Behavior
- A configured directory that is absent or unreadable is skipped; discovery continues with the rest.
- A registry file that is malformed or mid-write is skipped for that tick.
- Discovery failing as a whole surfaces an error state, distinct from the empty state.
- Sessions the pet cannot interpret (Codex, this release) are ignored silently.

## Alternatives Considered
Recorded in the story spec's Design Requirement and not re-opened: agent hook configuration, the
per-process IPC socket, terminal/PTY capture, the macOS Accessibility API, and process-tree inspection
as a status channel.

## Trade-Offs and Consequences
- Polling accepts a bounded staleness window (~2 s) in exchange for detecting force-kills at all.
- Adapter-owned liveness accepts that each agent's rule is written twice over, in exchange for the pet
  never needing to change when an agent is added.
- Re-deriving the directory list every tick accepts a repeated process scan in exchange for newly
  launched sessions and unseen profile directories needing no special handling.
- A Rust core behind a JSON FFI accepts a two-language build (cargo staticlib, Swift link, hand-rolled
  `.app` bundle) in exchange for a frontend that is fully native on each OS and a core that is not
  locked to any.

## Testing and Rollout
Rollback is quitting the pet. Nothing persists that needs undoing and no agent's behaviour depends on
it running.

Verified during build (release 001):

- 40 automated tests in `core/`, covering the three-part liveness rule against fixtures (orphaned
  registry file, recycled PID with mismatched `procStart`, healthy session), activity-text derivation
  including the missing-description fallback and a transcript with no tool activity yet, incremental
  reading with a half-written trailing line, and discovery across two directories with one absent.
- The liveness rule end-to-end against the real process table: a spawned process with a matching
  `procStart` produced a row; the same file with a deliberately wrong `procStart` produced none; and
  after `kill -9`, with the registry file still on disk, it produced none.
- Live against this machine: three CLI sessions across two profile directories, one of which
  (a custom `CLAUDE_CONFIG_DIR`) is not a default and was found only through a running process's `CLAUDE_CONFIG_DIR`. The
  VS Code-launched session was correctly absent, and `~/.codex` was watched and ignored without error.
- On screen: rows over both a dark and a light foreground window; activity text matching the agent's
  real current command; two same-project sessions disambiguated; a session started under a
  previously-unseen profile directory appearing within one tick; that row disappearing within ~5 s of
  `kill -9`; and `sessions unreadable` shown for a malformed config, then recovering when it was
  removed.
- Read-only and local: `lsof` shows the running app holds no sockets and no writable file descriptors
  beyond stdout/stderr, and no write, create or remove call exists outside test modules.
- Footprint: 39.6 MB resident, 0.32 s of CPU over 90 s (~0.36%).

Confirmed by the developer in real use: the empty state (`No agents running` with every session
stopped), and that a click on the pet stops there rather than reaching the window beneath.

### Amended after real use
The surface originally passed clicks through (`ignoresMouseEvents = true`), which the story required.
In daily use that made the pet an invisible trap over controls the user could not see — a click landed
on a window's close button it was covering and ended a live agent session. The panel now catches its
own clicks. It still never takes keyboard focus: `.nonactivatingPanel` with `canBecomeKey` and
`canBecomeMain` both false means clicking it moves focus nowhere, which is the guarantee that actually
mattered. Passing clicks through is also incompatible with dragging the pet, so the two findings
pointed the same way.

The count-up timer was re-stamping its observation time on every discovery tick, so it reset every two
seconds instead of showing a status's real age. The timestamp is now pinned to when a status first
appeared and restarts only when the state or the activity line changes.

## Open Questions
None. Every question this document was opened to answer is settled above.

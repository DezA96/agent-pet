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
- **C-005 Attention states** — waiting-for-input and errored are tellable from working at a glance, without reading the status text. (Rescoped twice — see Explicit Scope Changes.)
- **C-020 The pet itself** — a drawn creature below the rows, offset by its position on the display, with the status rows as a speech bubble that switches sides at the screen border. (Split from C-005 — see Explicit Scope Changes.)
- **C-006 One status row per live session** — each live session gets its own row (agent, project, current activity), so concurrent sessions each carry their own clear signal and are distinguishable from one another. (Rescoped — the pet itself moved to C-005; see Explicit Scope Changes.)
- **C-012 Live-session discovery** — only sessions actually running are shown; ended and stale sessions disappear.
- **C-014 Add-directory picker** — the watched-directory list is editable from the pet, not only by hand-editing config. (Added mid-release — see Explicit Scope Changes.)
- **C-016 Placeable and controllable pet window** — drag to move, position remembered, menu bar icon for show/hide and quit. (Added mid-release — see Explicit Scope Changes.)
- **C-017 Status age counts from the real status change** — the age beside a session is time since the status actually changed, not since the pet first looked. (Added mid-release — see Explicit Scope Changes.)
- **C-028 The status age survives a state the pet polled straight past** — a row never counts from a moment that has stopped being true, whichever agent it belongs to. (Added mid-release — see Explicit Scope Changes.)

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
- **Error detection may be weaker for Codex.** Claude Code signals failure explicitly; Codex's transcript may not. If Codex errors prove not reliably detectable, that is an explicit scope change, not a silent gap. **Settled (spike, story 004) — in the direction feared, and worse than "weaker".** A real CLI session was driven to a genuine approval prompt the developer accepted; across the whole rollout, zero approval or error events of any type, and during the full 15.0 s the session sat blocked the rollout wrote nothing at all. A waiting Codex session is byte-for-byte indistinguishable from a slow command. Recorded as an explicit scope change below, not a silent gap.
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
- **Pet avatar moved from C-006 into C-005** (implement round, after story 001). Story 001 built C-006's
  per-session rows but not its other half — "a single pet anchors the surface" — leaving the charter's
  central noun unbuilt: the surface is a list of text with no pet on it. Not a release expansion; the
  avatar was always planned scope. Reallocated because the pet's entire job in the charter is to convey
  state, so building the creature in one story and the states it expresses in another splits one feature
  down the middle. C-006 is now rows only and is satisfied by story 001; C-005 delivers the pet and its
  attention states together.
- **Added C-017 status age counts from the real status change** (spec round, story 002). Conscious
  expansion. Claude Code writes `statusUpdatedAt` — a unix-ms timestamp of when a session's status last
  actually changed — on every registry file, and `RegistryEntry` never parses it; the adapter stamps
  `observed_at = now_ms()` instead. The age therefore measures when the *pet* looked, not when the
  status began. Observed live while specifying story 002: a session idle for 96 minutes would display
  as `0s` after a relaunch. Not a defect against story 001, whose criterion says "since that status was
  observed" and is met — the criterion turned out weaker than the data already on disk. Expanded rather
  than backlogged because a glanceable surface stating a false age is worse than stating none, and the
  release goal is a glance that can be trusted. Kept as its own story rather than folded into 002:
  it is an observation-core change with nothing to do with window placement.
- **C-013 uncut and absorbed into C-005** (spec round, story 004). Conscious expansion. C-013 — the
  pet's own appearance summarising the most urgent state across all live sessions — was cut at scope
  time as "redundant with per-row state". That reasoning assumed a pet already existed and the
  aggregate would merely repeat what the rows said. It does not: the surface has no creature on it at
  all, so C-005 is drawing one for the first time, and a creature that never reacts to anything is
  decoration rather than the thing the charter describes as mirroring agent state. The release
  criterion stays per-row and is unchanged; the aggregate is what makes the pet worth drawing, not a
  substitute for the rows. Absorbed into C-005 rather than tracked as its own story, for the same
  reason the avatar was: one creature and the states it expresses are one feature.
- **Attention-state observability corrected** (spec round, story 004). The plan's risk "Error detection
  may be weaker for Codex" was half right, and the Claude half was wrong in the pet's favour. Reading
  the shipped CLI bundle rather than inferring from files on disk: Claude Code publishes
  `status: busy | idle | waiting | shell` with a `waitingFor` reason string, and marks transcript
  errors as `isApiErrorMessage` with `apiErrorIsTransient` — so both attention states are readable
  from channels the pet already reads, and story 001's exclusion "distinguishing blocked-awaiting-
  permission from plain idle" was never the hard problem it looked like. Codex remains open: it defines
  the approval and error events but has written none across 18 rollouts and 4 CLI versions between this
  round and story 003's. Resolved by spike, not assumption.
- **Codex attention states dropped from the release** (spike, story 004). The plan's own risk clause
  required this be an explicit scope change rather than a silent gap, so here it is. Codex's rollout —
  the pet's only channel, since the story forbids touching agent-owned config — records nothing while a
  session waits on the user. The alternative was timing out an unanswered `custom_tool_call`, which is
  a tuned staleness threshold: the exact mechanism story 001 rejected, and one that would fire on every
  long build. Codex keeps `Working | Idle | Unknown`. The release criterion "a waiting-for-input session
  and an errored session are each tellable from a working session at a glance" is therefore met for
  Claude and not for Codex; the criterion is left as written rather than weakened, so the gap stays
  visible at release close. Consequence accepted deliberately: Claude Code is where the developer's day
  is spent, and a blocked Codex session still shows as `Working`, which is incomplete but not false.
- **C-005 split into C-005 and C-020** (spec round, story 004). Not a scope change to the release —
  the same work ships, in two stories instead of one. The pet's visual form settled during this round
  as a drawn creature sitting below the rows, offset left or right by where it sits on the display,
  with the rows drawn as a speech bubble that switches sides at the screen border. That is a
  substantially larger piece than the attention states it was bundled with, and it reshapes the window
  frame that story 002's remembered-position and 40x40 validity rules operate on. Split at refinement,
  where splitting belongs, on three grounds: C-005 alone satisfies the release's attention-state
  criterion, because that criterion is per-row and the row indicator delivers it; nothing C-005 builds
  is thrown away by C-020, since the row indicator lives inside the bubble unchanged; and the release
  keeps moving while the visual work takes the time it needs. C-013's aggregate expression travels with
  the creature into C-020.
- **Added C-028, a proper fix for the age pin** (build review, story 006). Conscious expansion, taken
  with the release's last planned story already built and reviewed. Story 006 makes the age count from
  the agent's own record of when a status began, and holds that number still for as long as the
  displayed state holds — which its criterion 7 requires, so a `busy` session dropping into `shell`
  does not snap to `0s` over a change the user cannot see. Three review rounds established that the
  same hold also discards a *genuinely* newer time: a turn boundary crossed entirely between two
  two-second polls, an error that recovered and recurred inside one interval, and a timestamp that was
  unreadable on a row's first tick and readable afterwards. Each leaves a row counting from a moment
  that is no longer true, which is the exact failure this release added C-017 to remove — a glanceable
  surface stating a false age is worse than stating none. Expanded rather than backlogged for that
  reason, and because the release's own goal is a glance that can be trusted. Not folded into story
  006, which meets every criterion it was written against: the fix needs `AgentSession` to distinguish
  a state that has run since T from one that restarted at T, and that is a change to the single
  contract every adapter is written against, settled per-agent inside each adapter rather than in the
  pet — the same conclusion story 003 reached about liveness. Story 006 therefore closes with the limit
  recorded rather than hidden, and C-028 carries the fix.

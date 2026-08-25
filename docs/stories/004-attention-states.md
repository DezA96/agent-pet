# Story: Attention states

## Release
001 — Glanceable Agent Status ([plan](../releases/001-glanceable-agent-status.md))

## Status
Done

## User Outcome
As the developer, I want a session that is blocked on me or has died to look different from one that is working,
so that a glance answers "does anything need me?" without my reading a single row of text.

## Acceptance Criteria
- Given a Claude session's registry file reports `status: "waiting"`, when its row is shown, then its
  state is waiting — not `unknown`, which is what it renders as today.
- Given a waiting session's registry file carries a `waitingFor` reason, then that reason is shown as
  the row's detail text, in the agent's own wording.
- Given a Claude session's registry file reports `status: "shell"`, then its state is working. Shell
  mode is not an attention state: nothing is wrong and nothing is wanted from the user.
- Given a Claude session's newest transcript entry is an `isApiErrorMessage`, and the registry status is
  not `busy`, then its state is errored. Both halves are required: an error the agent retried through is
  followed by later entries and a `busy` status, while one it died on stays newest.
- Given an errored session produces any later transcript entry, then it stops being errored. The state
  never latches.
- Given the errored entry carries an `apiErrorStatus`, then the row's detail reads `Error: <status>`
  (e.g. `Error: 529`); given it does not, the row reads `Errored`. The code is shown as fact and never
  interpreted — observed live, a `429` was a hard session limit, not a rate limit to wait out.
- Given a `tool_result` carrying `is_error`, then the state is not errored — these are failures the
  agent recovers from on its own; all 16 in this project's transcripts are auto-mode classifier denials.
- Given any session row is shown, then it carries a state indicator whose **shape** differs per state,
  so the state is readable without reading the status text and without relying on colour.
- Given the surface is drawn over a light foreground window and over a dark one, then every indicator
  stays legible on both — no state is carried by a colour that disappears against either.
- Given a session is waiting or errored, then its indicator animates; given it is working, idle or
  unknown, then its indicator is still. Movement on the surface always means something wants the user.
- Given the indicator animates, then it does so on the existing 1 Hz display timer — no second timer,
  and no measurable change in CPU over the footprint story 001 recorded (0.32 s over 90 s).
- Given a Codex session, then its state remains `Working | Idle | Unknown` — unchanged by this story,
  for the reason the spike settled.
- Given the automated tests are run with `./test.sh`, then they pass.

## Excluded From This Change
- **Codex attention states.** Resolved by spike, not assumed — see Design Requirement. Codex's rollout
  records nothing while a session waits on the user, so waiting and errored are unobservable through
  the only channel the pet has. Codex keeps `Working | Idle | Unknown`, as story 003 shipped. Recorded
  as an explicit scope change in the release plan.
- **The pet itself** (C-020, Planned for this release) — the drawn creature, its placement below and
  offset by display position, the speech-bubble rows, the side-flip at the screen border, and C-013's
  aggregate expression. Split from this story at refinement; this story's row indicator lives inside
  that bubble unchanged.
- **A sprite-sheet or raster-animated creature** (C-019, Candidate). C-020 draws the pet as vector
  paths; an asset pipeline is not something the current hand-rolled `build.sh` should grow for it.
- **Per-agent icon or colour on a row** (C-018, Candidate) — unchanged by this story.
- **Sound or any attention cue outside the surface** (C-007, Candidate).
- **Any control of the agent from the pet**, including answering the prompt a waiting session is
  blocked on (charter non-goal — the pet is read-only).

## Edge Cases
- A session blocked on a permission prompt renders as `State unknown` today, because
  `core/src/claude/mod.rs` buckets every status outside `busy`/`idle` into `Unknown`. This story is what
  fixes that.
- `waitingFor` is not a closed set: `topDialogWaitingFor` passes through whatever the live dialog calls
  itself, so the row must render arbitrary text and truncate it by the existing rule.
- An errored session is still a *live* session — the process is running. It must keep its row rather
  than disappearing, which is what makes the error visible at all.

## Data or API Changes
- `core/src/session.rs`: `State` gains `Waiting` and `Errored`. This is the payload the Swift frontend
  decodes, so both sides change together; no persisted data and no migration.
- `core/src/claude/registry.rs`: `RegistryEntry` gains `waiting_for`.
- No change to any file the pet writes, and no change to the config file.

## Testing Notes
- Automated (Rust, `./test.sh`): `status: "waiting"` maps to `Waiting` and not `Unknown`; `waitingFor`
  reaches the session payload; `status: "shell"` maps to `Working`; an unrecognised status is still
  `Unknown`.
- Automated (Rust): the error rule in both directions — an `isApiErrorMessage` that is newest with a
  non-`busy` status is errored; the same entry followed by later entries, or with a `busy` status, is
  not; a `tool_result` with `is_error` never is.
- Automated (Rust): `Error: <apiErrorStatus>` where the code is present, bare `Errored` where it is not.
- Automated (Rust): the existing suites still pass unchanged — the new states must not disturb Codex's
  mapping or the liveness rules.
- Manual, against the real agent: a live Claude session driven to a permission prompt shows `waiting`
  in its registry file on disk **and** a waiting row on the pet. This is the one criterion resting on a
  value read from the CLI bundle rather than observed, so it is not called met until seen live.
- Manual: the surface over a light and a dark foreground window, confirming every indicator stays
  legible on both.

Run 2026-08-25: `./test.sh` — 18 Swift tests and 104 Rust tests pass (13 of the Rust tests are new).

Verified against the running app 2026-08-25, build `86306`:

- **`status: "waiting"` observed live on disk**, which is what the bundle read alone could not settle.
  A watcher polling every registry file every 250 ms caught `status='waiting' waitingFor='input needed'`
  on two independent sessions — `64999.json` at 01:40:27 and `72185.json` at 01:42:07 — each at the
  moment its session put a question to the user. Under the previous build both rows read
  `State unknown`.
- **All five states drawn and distinguishable**: filled disc (working), hollow ring (idle), orange
  triangle (waiting), red cross (errored), hollow diamond (unknown). Confirmed on screen with a
  waiting row and an errored row live beside real sessions.
- **The errored path end to end**: a staged profile whose transcript ended in an `isApiErrorMessage`
  with `apiErrorStatus: 529` produced a row reading `Error: 529`, from a registry file anchored to a
  real live PID and its real `procStart` — the liveness rule was satisfied, not bypassed. Staging was
  reverted: fixture processes killed, fixture directory deleted, and `~/.config/agent-pet/config.json`
  removed (it had not existed beforehand).
- **Only the attention states move.** Two captures one second apart show the cross and the triangle
  visibly dimmed while the idle ring is unchanged.
- **Legible over light and dark.** Captured over a dark editor and over a white full-window image; all
  indicators stay readable on both, and shape distinguishes every state without reference to colour.
- **Footprint — measurable change not detectable by this method, which is weaker than proving there is
  none.** Three 90-second samples of the same process: 0.43 s CPU with 3 static rows; 0.74 s with 7
  rows of which 4 were animating; 1.58 s with the same 7 rows and none animating. The run *with*
  animation was cheaper than its own control, so run-to-run variance on a working machine exceeds
  whatever the animation costs. All samples are under 2% of one core. The structural claim is the
  firmer one: animation reuses the existing 1 Hz display timer, adds no second clock, and a still state
  redraws nothing (`StateIndicatorView.advance()` returns early unless the state wants attention).

## Design Requirement
- No separate design needed. The observation channels are the ones the pet already reads — the Claude
  registry file and its transcript — and the adapter seam settled in
  [design doc 001](../design-docs/001-observation-channel-and-pet-surface.md) absorbs new states
  without change: `State` is part of the session payload the pet renders generically. Nothing here is
  cross-cutting, novel, or expensive to reverse.
- No ADR. Nothing decided here is hard to reverse; a state mapping is a few lines in one adapter.

### Spike: can a waiting Codex session be observed? — RESOLVED, NO
**Question.** Codex defines `exec_approval_request`, `apply_patch_approval_request`,
`request_user_input`, `elicitation_request`, `stream_error` and `turn_aborted`, but none had ever
appeared in a rollout — 18 rollouts across 4 CLI versions between this round and story 003's. Unknown
whether they were never triggered (this machine sets `approvals_reviewer = "auto_review"`) or never
persisted.

**Method.** A real CLI session in `/private/tmp/codex-spike` under `codex -a on-request -s read-only`,
outside the `trust_level = "trusted"` roots. The developer asked it to force an approval prompt and
accepted it; the agent confirmed "Approval succeeded", and a second approval followed for the cleanup.
Rollout: `~/.codex/sessions/2026/08/25/rollout-2026-08-25T01-25-37-01a03761-961c-….jsonl`.

**Result.** Zero approval or error events, of any type, in the whole file. During the full 15.0 seconds
the session sat blocked on the developer, the rollout wrote **nothing** between the `custom_tool_call`
at 05:29:31.085 and its output at 05:29:46.083. The only `approval` strings present are `turn_context`
configuration (`approval_policy`, `approvals_reviewer`), which record settings rather than state.

**Conclusion.** A Codex session waiting on the user is byte-for-byte indistinguishable, in its rollout,
from one running a slow command. Detecting it would mean timing out an unanswered `custom_tool_call` —
a tuned staleness threshold, the mechanism story 001 rejected, and one that would fire on every long
build. Codex attention states are therefore excluded from this story and recorded as an explicit scope
change in the release plan. Under story 003's existing mapping such a session already shows as
`Working`, which is incomplete but not false: it is genuinely mid-turn.

**Retired risk.** The release plan's "Error detection may be weaker for Codex" is now settled in the
direction it feared, and settled by experiment rather than assumption.

Investigation established, by reading the shipped agent binaries rather than inferring from files on
disk:

- **Claude Code publishes `status: busy | idle | waiting | shell`**, not the `busy|idle` story 001
  assumed. `waiting` carries a `waitingFor` reason — `"sandbox request"`, `"input needed"`,
  `"worker request"`, `"dialog open"`, or the live dialog's own label. Both fields are validated by the
  CLI's own pid-file read-back parser, alongside `statusUpdatedAt`, `state`, `detail`, `tempo`, `needs`.
  This reverses story 001's exclusion "Distinguishing 'blocked awaiting permission' from plain `idle`":
  it is a published value, not a heuristic. Read from the bundle, not yet seen in a live pid file — a
  `waiting` status must be observed on disk during the build before the criterion is called met.
- **Claude Code marks errors in the transcript** as `isApiErrorMessage: true` — in the same JSONL the
  pet already tails for the activity line. The bundle also defines `apiError` and `apiErrorIsTransient`,
  but **neither appears in any real entry**: all three found across both profiles carry only
  `apiErrorStatus` (a numeric HTTP code) and `error` (a slug — `server_error`, `rate_limit`). So
  "transient" cannot be read off a flag, and the newest-entry-plus-status pairing above does that work
  instead.
- **Codex defines the events but never writes one** — settled by the spike above.

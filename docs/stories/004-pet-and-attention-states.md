# Story: The pet and its attention states

## Release
001 — Glanceable Agent Status ([plan](../releases/001-glanceable-agent-status.md))

## Status
Draft

## User Outcome
As the developer, I want a pet on the surface whose appearance tells me when an agent is waiting on me or has
died, so that a glance answers "does anything need me?" without my reading a single row of text.

## Acceptance Criteria
_Settled in the spec round; visual vocabulary still open (round 2)._

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
- Given the pet is running, then a drawn creature is present on the surface, and its appearance reflects
  the most urgent state across all live sessions (errored > waiting > working > idle).
- Given no session is live, then the creature is still present.
- Given a session is waiting and another has errored, then the creature reflects the errored one.

## Excluded From This Change
- **Codex attention states.** Resolved by spike, not assumed — see Design Requirement. Codex's rollout
  records nothing while a session waits on the user, so waiting and errored are unobservable through
  the only channel the pet has. Codex keeps `Working | Idle | Unknown`, as story 003 shipped. Recorded
  as an explicit scope change in the release plan.
- **A sprite-sheet or raster-animated creature** (C-019, Candidate). This story draws the pet as vector
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
_Settled after round 2._

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

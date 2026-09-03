# Story: Status age counts from the real status change

## Release
001 — Glanceable Agent Status ([plan](../releases/001-glanceable-agent-status.md)); backlog row C-017.

## Status
Ready

## User Outcome
As the developer, I want the age beside a session to say how long that session has actually been in the state
it is in, so that a glance tells me a session has been stuck for forty minutes instead of restarting
the count every time I relaunch the pet.

## Acceptance Criteria

**Where the age comes from**
- Given a Claude session whose registry file carries a `status` and a `statusUpdatedAt`, when its row
  is shown, then the age counts from `statusUpdatedAt`. A session idle for 96 minutes reads 96 minutes,
  whether or not the pet was running when it went idle.
- Given the pet is relaunched while a session's status has not changed, then that row's age carries on
  from the status's real start rather than restarting at `0s`. This is the defect C-017 was raised for.
- Given a Claude session is errored, then the age counts from the `timestamp` of the transcript entry
  that made it errored — the `isApiErrorMessage` line — and not from `statusUpdatedAt`. The two name
  different facts: errored outranks every registry status, so a session that errors and later has a
  dialog opened on it still shows errored while `statusUpdatedAt` has moved on to the dialog.
- Given a Codex session's state came from a turn boundary, then the age counts from that boundary
  line's own `timestamp`, for `task_started` and `task_complete` alike, and whether the boundary was
  found by the forward tail or by the backscan.
- Given a session's state could not be read from anything the agent timestamped — a registry file
  publishing no `status`, or a Codex rollout with no turn boundary inside the backscan — then its age
  behaves exactly as it does today: counted from when the pet first saw that reading, restarting on
  relaunch. `statusUpdatedAt` is read only when a `status` was read with it.

**What moves the age and what does not**
- Given a working session's activity line changes while its state does not, then the age does not
  restart. On Claude the age therefore measures the current turn rather than the time since the last
  tool call.
- Given the agent's status changes but the state the pet displays does not — `busy` to `shell`, both
  of which the pet shows as working — then the age keeps the earlier time. The age measures how long
  what is on screen has been true, so a change the user cannot see never moves the number.
- Given a waiting session's `waitingFor` reason changes while it stays waiting, then the age likewise
  keeps the earlier time.
- Given any row is shown, then the activity line reads exactly as it did before this story and updates
  exactly as often — the tool currently in use is still named and still changes as the session works.
  This story changes what the age counts from and nothing else on the row.

**How the age reads**
- Given a status under a minute old, then the age reads as it does today: `47s`.
- Given a status under an hour old, then it reads `2m 05s` — minutes unpadded, seconds padded.
- Given a status an hour old or more, then it reads `1h 03m 47s` — hours unpadded, minutes and seconds
  padded. A unit is padded to two digits only when a larger unit precedes it, so a label's width is
  fixed within a tier and steps only when a unit is gained.
- Given a status older than a day, then there is no day unit: it reads `51h 20m 14s`.
- Given an agent-supplied time in the future — a clock change, or skew — then the age reads `0s`
  rather than counting down or showing a negative number.

## Excluded From This Change
- The activity line: what it says, where it comes from, and how often it changes. Untouched, on the
  developer's explicit constraint — the row must still name the tool currently in use exactly as now.
- The state rules. Which state a session is in belongs to stories 001, 003 and 004; this story changes
  only what the age beside it counts from.
- Persisting ages across a relaunch for sessions the agent gave no time for. Those keep today's
  in-memory behaviour rather than gaining a store of their own.
- A day unit in the age, and any coarsening that drops seconds. Settled against: seconds always show.
- Any change to the row's layout, the state dot, the creature, or the bubble.
- C-015 (persisting learned profile directories) — unrelated, and still a candidate.

## Edge Cases
- **`busy` to `shell`.** The agent's status changed, the displayed state did not. The age holds.
  Settled above; called out here because reading `statusUpdatedAt` verbatim would snap it to `0s`.
- **Errored, then a dialog opens.** Status moves to `waiting`, the row still reads errored because
  errored outranks it, and the age must still date the error. This is the case that separates the two
  timestamp sources, and the reason the errored row does not read `statusUpdatedAt`.
- **A registry file with `statusUpdatedAt` but no `status`.** The state is unknown, so there is no
  status whose age this would be. Falls back rather than dating a status the pet never read.
- **A session that disappears and comes back.** Treated as new, as today — and the age it comes back
  with is whatever the agent then reports, which may legitimately be an old time.
- **A very long-lived status.** No day unit, so it keeps counting in hours.
- **Codex `started_at` versus `timestamp`.** `task_started` carries both; only `timestamp` is used,
  because it is the one field every boundary line has and using one field for both boundaries avoids
  two rules that must be kept agreeing.

## Data or API Changes
- `AgentSession.observed_at` is renamed to `status_since` (`statusSince` over the FFI JSON and in
  Swift's `AgentSession`), because after this story the field means "when this state began", not "when
  this observation was taken". Not an incremental change: the field is internal, has exactly one
  consumer, and Rust core and Swift app ship as one binary from `build.sh`, so there is no version of
  the pet in which an old reader meets a new writer.
- Nothing is persisted, so no stored structure changes and no migration exists. The config file is
  untouched.

## Testing Notes

1. `RegistryEntry` parses `statusUpdatedAt`, and reads as absent when the field is missing or is not a
   number. Unit (Rust).
2. A Claude session with `status` and `statusUpdatedAt` reports `status_since` equal to that value, for
   `busy`, `idle`, `waiting` and `shell`. Unit (Rust).
3. An errored Claude session reports `status_since` taken from the `isApiErrorMessage` entry's
   `timestamp`, staged so that `statusUpdatedAt` holds a different, later value — the dialog-after-error
   case — and the error's time is the one that wins. Unit (Rust).
4. A Codex session reports `status_since` from its boundary line's `timestamp`, for `task_started` and
   for `task_complete`, and by the backscan path as well as the forward tail. Unit (Rust).
5. A displayed state unchanged across two ticks keeps its earlier `status_since` even though the
   agent moved something underneath it — `busy` then `shell`, and a `waiting` session whose
   `waitingFor` reason changes. Unit (Rust).
6. A session with no agent-supplied time keeps today's behaviour: first-seen on the first tick, pinned
   across later ticks while state and activity hold. Unit (Rust) — the existing `lib.rs` tests, still
   passing against the renamed field.
7. A `status_since` in the future renders as `0s`. Unit (Swift).
8. Age formatting at each tier and at both boundaries: `0s`, `47s`, `59s`, `1m 00s`, `2m 05s`,
   `59m 59s`, `1h 00m 00s`, `1h 03m 47s`, `51h 20m 14s`. Unit (Swift).
9. No occurrence of `observed_at` or `observedAt` remains anywhere in `core/src`, `macos/Sources`,
   `macos/Tests`, `core/examples` or `tools/`. Mechanical pass (grep).
10. `./test.sh` passes. Run.
11. The C-017 observation, re-run: a Claude session left idle for a known span, the pet quit and
    relaunched, and the row's age read against that span rather than restarting at `0s`. Manual.
12. A Claude session working through several tool calls: the activity line changes as before and names
    the tool in use, while the age climbs continuously across those changes rather than resetting.
    Manual.
13. A live Codex row's age read against the `timestamp` of the last boundary in its own rollout file.
    Manual.
14. A row whose project name sits near its wrap point does not re-wrap as the age ticks within a tier,
    and the dot and age stay level with the name's first line as story 004 requires. Manual.

## Design Requirement
- Design: none — routine work following an existing pattern. Each adapter already reads its agent's own
  files and fills a field on `AgentSession`; this adds one more field from the same reads, and the
  rendering change is a formatter in a view that already has one.
- Spike: none
- Prototype: none
- Decision: none

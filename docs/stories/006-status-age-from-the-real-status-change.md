# Story: Status age counts from the real status change

## Release
001 — Glanceable Agent Status ([plan](../releases/001-glanceable-agent-status.md)); backlog row C-017.

## Status
Implementing

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
- Given an agent-supplied time older than the session it belongs to — a value in the wrong unit, which
  `statusUpdatedAt` arrives as a bare number and so fails no parse — then it is refused and the age
  falls back to first-seen, rather than rendering an age of half a million hours. Added during build:
  before this story the age came from the pet's own clock and could not be absurd, so making it
  agent-supplied is what created the case. (See Amended During Build.)

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
- Holding the age correct across a state the pet polled straight past. Criterion 7 requires the age to
  hold while the displayed state holds, and the same hold also discards a genuinely newer agent time
  when a turn boundary, an error recovery, or an unreadable first reading falls entirely between two
  polls. Established across this story's three review rounds, not at spec time. Every criterion here is
  met as written; fixing this needs `AgentSession` to distinguish a state that has run since T from one
  that restarted at T, which is a change to the contract every adapter is written against. Recorded as
  C-028 and expanded into release 001 rather than folded in here.

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
      - Method: run
      - Observed: `claude::registry::tests::the_status_change_time_is_read` and
        `::a_missing_or_unusable_status_time_reads_as_absent_and_keeps_the_entry`
        (`core/src/claude/registry.rs`). The second is the load-bearing one: the field is read through
        a `lenient_ms` deserializer (`registry.rs:57`) rather than straight into `Option<u64>`, so a
        string, a float, a null, an array or a negative reads as absent and the entry survives. Read
        into `Option<u64>` directly, any of those would have failed the whole entry to parse and cost
        the row, not just its age.
2. A Claude session with `status` and `statusUpdatedAt` reports `status_since` equal to that value, for
   `busy`, `idle`, `waiting` and `shell`. Unit (Rust).
      - Method: run
      - Observed: `claude::tests::a_status_dates_the_row_from_the_agents_own_status_change`
        (`core/src/claude/mod.rs`), which loops all four statuses through the real adapter.
3. An errored Claude session reports `status_since` taken from the `isApiErrorMessage` entry's
   `timestamp`, staged so that `statusUpdatedAt` holds a different, later value — the dialog-after-error
   case — and the error's time is the one that wins. Unit (Rust).
      - Method: run
      - Observed: `claude::tests::an_errored_row_dates_the_error_not_the_registry_status`. The fixture
        is the exact case: `status: "waiting"`, `waitingFor: "dialog open"`,
        `statusUpdatedAt: 1787599999999`, and an error entry timestamped `2026-08-25T00:41:35.372Z`.
        The row reads `Errored` and dates the error, not the dialog. Also
        `claude::transcript::tests::an_error_carries_the_time_of_the_entry_it_stopped_on`, and
        `::an_error_with_no_readable_timestamp_still_reports_the_error` — an unreadable time costs the
        age, never the state.
4. A Codex session reports `status_since` from its boundary line's `timestamp`, for `task_started` and
   for `task_complete`, and by the backscan path as well as the forward tail. Unit (Rust).
      - Method: run
      - Observed: `codex::rollout::tests::a_boundary_dates_the_state_from_its_own_timestamp` (both
        boundaries) and `::a_boundary_found_by_the_backscan_is_dated_too`, which pads past
        `FIRST_READ_WINDOW * 3` so the state can only come from `last_boundary_before`. The
        `task_started` fixture carries a deliberately wrong `started_at` (`2020-01-01`) alongside its
        real `timestamp`, so a build that read the payload field instead would fail rather than pass by
        luck. Also `::a_rollout_with_no_boundary_has_no_time_to_date` and
        `::a_boundary_with_an_unreadable_timestamp_still_decides_the_state`.
5. A displayed state unchanged across two ticks keeps its earlier `status_since` even though the
   agent moved something underneath it — `busy` then `shell`, and a `waiting` session whose
   `waitingFor` reason changes. Unit (Rust).
      - Method: run
      - Observed: `tests::a_status_moving_under_an_unchanged_state_does_not_move_the_age`
        (`core/src/lib.rs`), covering both. The pin key is now the displayed state alone; it was
        `(state, activity)` before, which is what made check 6's old behaviour.
6. A session with no agent-supplied time keeps today's behaviour: first-seen on the first tick, pinned
   across later ticks while state and activity hold. Unit (Rust) — the existing `lib.rs` tests, still
   passing against the renamed field.
      - Method: run
      - Observed: `tests::an_unchanged_status_keeps_counting_up_instead_of_restarting`,
        `::a_changed_state_takes_the_agents_new_time` and `::a_session_that_left_and_returned_starts_fresh`,
        all still passing. One test in that group was inverted deliberately and renamed:
        `a_changed_activity_restarts_the_count` is now
        `a_changed_activity_does_not_restart_the_count`, which is the criterion this story adds. Two
        more cover the "no agent time at all" path end to end:
        `claude::tests::a_session_the_agent_timestamped_nothing_for_falls_back_to_this_tick` asserts a
        registry file carrying `statusUpdatedAt` but no `status` is dated from this tick, never from a
        status the pet never read.
7. A `status_since` in the future renders as `0s`. Unit (Swift).
      - Method: run
      - Observed: `aStatusTimeInTheFutureReadsAsZeroRatherThanCountingDown`
        (`macos/Tests/PetStateTests/PetStateTests.swift`), covering `-1`, `-90_000`, and a start an hour
        ahead of `now`.
8. Age formatting at each tier and at both boundaries: `0s`, `47s`, `59s`, `1m 00s`, `2m 05s`,
   `59m 59s`, `1h 00m 00s`, `1h 03m 47s`, `51h 20m 14s`. Unit (Swift).
      - Method: run
      - Observed: `theAgeReadsInTiersThatOnlyWidenWhenAUnitIsGained` covers every value listed;
        `aVeryOldStatusKeepsCountingInHoursRatherThanGainingADayUnit` covers `51h 20m 14s`. The
        formatter is `ageText` in `macos/Sources/PetState/Age.swift`, put in `PetState` rather than in
        the row view so it is reachable without a window — the same reason the state priority lives
        there.
9. No occurrence of `observed_at` or `observedAt` remains anywhere in `core/src`, `macos/Sources`,
   `macos/Tests`, `core/examples` or `tools/`. Mechanical pass (grep).
      - Method: mechanical pass
      - Observed: `grep -rn "observedAt\|observedDate\|observed_at" macos core tools` returns exactly
        one line, `core/src/session.rs:66`, inside the doc comment on `status_since` explaining why the
        field is no longer called that. Prose, not an identifier: no code, no JSON key and no test
        refers to the old name. Named here rather than removed, because the sentence is the record of
        the rename and a grep hit that a reader can resolve in one line is worth more than a silent
        comment.
10. `./test.sh` passes. Run.
      - Method: run
      - Observed: passes. 137 Rust tests (up from 116) and the full Swift suite, including the four new
        `PetState` age tests. `./build.sh` also completes, which is what proves the Swift half compiles
        at all — SwiftPM builds only `PetGeometry` and `PetState`, so the app's own sources reach a
        compiler only through `build.sh`. Run three times over to confirm no ordering flakiness after a
        pre-existing test-isolation defect was fixed (see check 6's neighbours and the note below).
        The count rose from 116 to 137 across three review rounds and the fix that followed them, which added tests for the FFI key and
        state-vocabulary contract, the date parser's year bound and its rejection of signed components,
        the Codex backscan's `task_complete` dating, both adapters' last hop into `status_since`, and an
        unrecognised Claude status; the third added no behaviour, only tests for branches that survived
        mutation — the leap-second clamp, a fraction longer than a `u64`, the Swift rounding, and Codex's
        own fallback to first-seen — and replaced one assertion that had become tautological. The Swift
        suite is 47 tests. A limit the rounds established rather than a defect against this story's
        criteria is recorded as C-028 and scoped into this release; see `## Excluded From This Change`.
11. The C-017 observation, re-run: a Claude session left idle for a known span, the pet quit and
    relaunched, and the row's age read against that span rather than restarting at `0s`. Manual.
12. A Claude session working through several tool calls: the activity line changes as before and names
    the tool in use, while the age climbs continuously across those changes rather than resetting.
    Manual.
13. A live Codex row's age read against the `timestamp` of the last boundary in its own rollout file.
    Manual.
14. A row whose project name sits near its wrap point does not re-wrap as the age ticks within a tier,
    and the dot and age stay level with the name's first line as story 004 requires. Manual.
15. A status time older than its own session is refused and the row falls back to first-seen, for both
    agents: Claude against `procStart`, Codex against its rollout's first line. Unit (Rust).
      - Method: run
      - Observed: `claude::registry::tests::a_status_time_older_than_the_process_is_refused_rather_than_shown`
        (unix seconds where ms were meant, and `0`, both refused; a plausible value and a value exactly
        at the process start both kept), `::an_unreadable_proc_start_applies_no_bound_rather_than_dropping_the_age`,
        `::proc_start_reads_as_the_utc_moment_the_process_began`, and
        `codex::rollout::tests::a_boundary_dated_before_its_own_session_is_refused_not_shown` plus
        `::the_session_start_is_read_from_line_one`, which reads the bound off the committed fixture.

## Amended During Build

- **A status time older than its own session is refused** (build, after the third review round). The
  spec clamped the future direction and said nothing about the past, because before this story the age
  came from `now_ms()` and could not be absurd. Making it agent-supplied created the case:
  `statusUpdatedAt` arrives as a bare number with no parsing to fail, so seconds where milliseconds
  were meant reads as 1970 and renders as `496766h 00m 01s` — and `SessionRowView` gives the age label
  `.required` compression resistance against the project name's `.defaultLow`, so the name is what
  gives way. Treated as this story's own defect rather than a candidate for later, because the story is
  what made it reachable. The bound is each session's own start — Claude's `procStart`, Codex's first
  rollout line — so it is a fact rather than a threshold somebody chose, and both were already being
  read for other reasons. Refused rather than clamped: the pet cannot tell what the agent meant, and
  first-seen is honest where a clamp would assert a precision nobody has.

## Design Requirement
- Design: none — routine work following an existing pattern. Each adapter already reads its agent's own
  files and fills a field on `AgentSession`; this adds one more field from the same reads, and the
  rendering change is a formatter in a view that already has one.
- Spike: none
- Prototype: none
- Decision: none

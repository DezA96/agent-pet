# Story: The pet itself

## Release
001 — Glanceable Agent Status ([plan](../releases/001-glanceable-agent-status.md)); backlog row C-020,
which carries C-013's aggregate expression (absorbed at the spec round of story 004).

## Status
Done

## User Outcome
As someone who uses various coding agents, I want the surface to be a creature that reacts to my
agents rather than a list of text, so that one glance at the creature tells me whether anything needs me
across every live session, before I read a single row.

## Acceptance Criteria
**The creature and the bubble**
- Given the pet is showing, then a single drawn creature is on the surface below the rows, and the rows
  are drawn as a speech bubble above it with a tail toward the creature. The creature is drawn from
  vector paths in Core Animation layers, is roughly 56 points tall, and fits within the bubble's
  existing 300-point width, so the surface is no wider than today.
- Given this story, then the creature is a simple placeholder: the properties above and the expressions
  below are what it must satisfy, and its look is not otherwise specified. (Settled in the alignment
  round: properties for now, a placeholder creature for now.)
- Given a session row is shown inside the bubble, then it is the row story 004 left: state dot, agent
  and project, detail line, age. Nothing in the row changes.
- Given the bubble is drawn, then the row indicator, the empty message and the error message read as
  they did before this story, and the bubble's width stays 300 points.

**Side and growth**
- Given the pet sits toward the right of its screen, then the creature is under the bubble's right
  corner and the bubble extends to its left; given it sits toward the left, then the creature is under
  the left corner and the bubble extends to its right.
- Given the bubble on its current side would cross a side edge of the screen's visible frame, then it
  switches to the creature's other side; given it still fits, then it stays on the side it is on. The
  flip is triggered by the bubble leaving the screen, never by crossing the screen's midpoint.
- Given the pet sits in the upper half of its screen, when a row is added or removed, then the frame's
  top edge holds and the creature moves down or up with the bubble's bottom. Given it sits in the lower
  half, then the frame's bottom edge holds, the creature stays put and the bubble grows or shrinks
  upward. Story 002's anchor rule is unchanged and there is no vertical flip.
- Given a drag carries the creature to where the bubble would leave the screen, then the bubble flips
  during the drag, live, and the creature stays under the pointer throughout.
- Given displays change while the pet is running and the bubble's current side no longer fits, then the
  bubble re-picks its side and the creature does not move.

**The remembered position**
- Given I have dragged the creature, when I quit and relaunch, then the creature reappears where I left
  it, with the bubble on the side it was on. What is remembered is the creature's position, the anchored
  edge, top or bottom, and the bubble's side; the frame is rebuilt from those.
- Given only story 002's keys exist, when the pet launches for the first time after this story, then
  the creature's position is derived from the remembered frame with the bubble on the side that fits,
  and written under the new keys. The old keys are left in place and are never written again.
- Given a remembered position where less than 40x40 points of the **creature** fall inside any
  connected screen's visible frame, when the pet launches, then it appears at the default top-right
  instead and the unusable position is not kept. The bubble's visibility is not part of the test.
- Given the pet has never been moved, when it launches, then the creature sits at the top-right of the
  display holding the menu bar, clear of its edges, under the bubble's right corner.
- Given the remembered side no longer fits at launch, then the side that fits is used from that tick
  on, and nothing is written. The fitting side is recomputed from the creature's position at every
  launch, so a stale remembered side costs one comparison and never reaches what is shown; it is
  written down the next time a drag writes anything. (Settled at the build's alignment round: as
  first written this criterion said "used and remembered", which contradicted the rule below that
  only a drag writes. Displaying the fitting side is what the criterion was for, and that needs no
  write.)
- Given any programmatic move — launch placement, growth by a row, a side flip, a display-change
  rescue — then nothing is written to the remembered position. Only a drag writes it.

**What the creature expresses (C-013)**
- Given one or more live sessions, then the creature shows the most urgent state among them, in this
  order: errored, waiting, working, unknown, idle. Unknown ranks above idle, so a surface holding only
  unreadable sessions never shows a content creature.
- Given each of those five states, then the creature has a distinct expression for it, carried by pose
  and face. The creature is not tinted in the state colour; colour stays the row dots' job.
- Given the aggregate state is errored or waiting, then the creature moves with the same fast, deep
  rhythm as those rows' dots (1.1 s to 0.28); given it is working, with the same slow, shallow rhythm
  (2.4 s to 0.55); given it is idle or unknown, then it holds still. Motion means the same thing on the
  creature as on a dot.
- Given the creature moves, then it does so through Core Animation rather than a redraw on a timer, so
  the surface's per-second work is unchanged from story 004.
- Given the creature's expression, then it is a function of the current poll and nothing else: no
  memory, no mood, nothing accumulated between polls.

**The surfaces with no rows**
- Given discovery succeeds and no sessions are running, then the bubble reads `No agents running` as
  before and the creature is asleep. The creature does not disappear.
- Given discovery fails, then the bubble reads `Sessions unreadable` as before and the creature shows
  its unknown expression: the pet cannot see, and no agent has failed.

**Clicks and focus**
- Given I press and drag on the creature, then the whole pet follows the pointer and keyboard focus
  stays wherever it already was. The creature is the only drag handle.
- Given I click or press on the bubble, then nothing happens, and the click does not reach the window
  beneath. The bubble is not a drag handle.
- Given I click on the transparent area beside the creature, then the click reaches the window beneath
  it: what is under a transparent gap is visible, so this is not the trap story 001 hit.
- Given the pet is running, then it never takes keyboard focus, including when the creature or the
  bubble is clicked.

**Unchanged guarantees**
- Given show/hide from the menu bar, then the whole surface, creature included, hides and shows.
- Given the automated tests are run with `./test.sh`, then they pass.

## Excluded From This Change
- **Per-row attention states** — delivered by story 004 and unchanged here. The row indicator lives
  inside the bubble exactly as it is.
- **Codex attention states** — Codex keeps `Working | Idle | Unknown`, as the release records; the
  creature aggregates whatever states the rows carry and invents none.
- **A sprite-sheet or raster-animated creature** (C-019, Candidate). The creature is drawn as vector
  paths; an asset pipeline is not something the hand-rolled `build.sh` grows for it.
- **Adjustable pet size** (C-010, Candidate).
- **Creature visible with the rows hidden** (C-011, Candidate). Show/hide stays whole-surface, as
  story 002 built it.
- **Per-agent icon or colour on a row** (C-018, Candidate), and any per-agent creature: one creature,
  whatever agents are live, since the pet holds no table of agents.
- **Sound or any cue outside the surface** (C-007, Candidate).
- **Clicking the creature or a row to focus that session's terminal** (C-008, Candidate).
- **Any behaviour on a click inside the bubble** (C-021, Candidate, recorded this round). The bubble
  catches clicks and does nothing with them; what a row, an agent name or an activity line should do
  when clicked is a later story.
- **The creature's finished drawing** (C-022, Candidate, recorded this round). This story ships a
  simple placeholder that satisfies the stated properties and expressions; a properly drawn vector
  creature was judged too much work to fit here.
- **Any control of the agent from the pet** (charter non-goal — read-only).
- **Virtual-pet mechanics** — no feeding, moods with memory, XP or evolution (charter non-goal). The
  creature's expression is a function of the current poll and nothing else.

## Edge Cases
- **A state changes every poll.** The creature's expression is a function of the current poll, so a
  session flapping between states flaps the creature with it, as the row dot already does. Expression
  changes are instant, like a dot's colour change; only the breath animates.
- **Mixed sessions.** Two Claude sessions working and one Codex session idle show a working creature;
  the moment one Claude session waits, the creature waits. Killing sessions in order of urgency steps
  the creature down the priority list one poll at a time.
- **Only Codex sessions live.** The creature can express at most working, idle or unknown, because
  those are the only states Codex rows carry. Not a defect: the release records why.
- **Neither side fits.** A screen narrower than the bubble plus the creature exists on no display this
  pet runs on; the rule keeps the current side rather than oscillating.
- **macOS rescues the pet first.** Story 002 recorded that on a display disconnect AppKit relocates
  windows onto a remaining screen, roughly where they last sat there, before the change notification
  arrives; the developer has since observed the same again. The side rule runs after that relocation
  and only re-picks the bubble's side, never the creature's position. Anyone reading a bug into where
  the creature landed after unplugging a monitor should check whether macOS moved it, not this code.
- **Hidden across a display change.** The side and validity rules still run while hidden, so showing
  the pet later never reveals a bubble off-screen or a creature stranded.
- **The bubble tail.** The tail points at the creature from whichever corner it sits under, so the two
  always read as one object after a flip.
- **The gap is not a drag handle and not a click trap.** A press in the transparent area reaches the
  window beneath; a press on the bubble is swallowed; only the creature drags. A user who grabs the
  bubble expecting to drag gets nothing, which is the settled behaviour, not a defect.

## Data or API Changes
- **New `UserDefaults` keys** under `gg.deza.agent-pet` for the creature's x and the bubble's side.
  Story 002's edge-y and anchor keys keep their meaning, since the frame's top is still the bubble's
  top and its bottom is now the creature's bottom. Story 002's frame-x key is read once, when the new
  creature-x key is absent, to derive the creature's position; it is then left in place and never
  written again. Addition alongside, not replacement.
- **`macos/Sources/PetGeometry/`** gains the side arithmetic as free functions over `CGRect`: which
  side the bubble takes given the creature's rect, the current side and the screen; the frame rebuilt
  from a creature position, side and anchor; validity tested on the creature's rect; and the one-time
  derivation of a creature position from a story 002 frame. All testable with no AppKit.
- **The aggregate state is computed in the Swift surface, not the Rust core.** It is a rendering
  concern over the session list the pet already receives, and the FFI payload does not change, so no
  adapter is touched. The priority function is pure and lives where the Swift tests can reach it.
- **A new `macos/Sources/PetState/` module**, with its own SwiftPM target and test target — an
  explicit scope change made at the build and recorded here at its close. Reaching the Swift tests
  meant a module they compile, and `PetGeometry` is documented as the placement arithmetic and
  nothing else, so a state-priority rule dropped into it would have made that description untrue.
  The new module holds `SessionState`, moved out of `Core.swift`, together with the urgency order,
  `aggregate`, and the breath's `(period, floor)` pair. That last one is the part worth having:
  the row dot and the creature now read the same definition of the rhythm, where two copies would
  have become two rhythms the first time either was tuned. `build.sh` compiles it into the app and
  `Package.swift` gains the two targets; `./test.sh` runs both suites unchanged.
- **A process-wide mouse-moved monitor**, added at the build and recorded here at its close rather
  than settled up front. Letting clicks through the transparent band beside the creature turns out to
  need `ignoresMouseEvents` toggled from the pointer's position — declining the point in `hitTest`
  looks like it should do the job and does not, measured — so the app installs an `NSEvent` global
  monitor paired with a local one, each being blind to what the other sees. It watches `.mouseMoved`
  only, which needs no Accessibility permission the way key events would, and it is torn down while
  the pet is hidden.
- **No change** to the Rust core, the FFI contract, the session payload, or
  `~/.config/agent-pet/config.json`.

## Testing Notes
1. Unit (swift-testing, `./test.sh`): side selection — a bubble that fits on its current side stays
   there; one that would cross the left edge flips to the creature's right; one that would cross the
   right edge flips to its left; with room on both sides the current side is kept.
      - Method: run
      - Observed: `./test.sh` — 42 swift-testing tests pass. `bubbleSide` in
        `macos/Sources/PetGeometry/PetGeometry.swift:113`; the four cases are
        `aBubbleThatStillFitsStaysOnTheSideItIsOn`, `aBubbleThatWouldCrossTheLeftEdgeFlips…`,
        `…CrossTheRightEdgeFlips…`, plus `theFlipIsDrivenByTheEdgeAndNotByTheMidpoint`, which pins the
        rule to the screen edge rather than the midpoint, and
        `aScreenTooNarrowForEitherSideKeepsTheSideItIsOn`.
2. Unit (swift-testing): the frame rebuilt from a creature position, side and anchor — a flip moves the
   frame's x by the bubble width less the creature width and leaves the creature's rect unchanged; the
   frame's top holds under a top anchor and the creature's bottom under a bottom anchor.
      - Method: run
      - Observed: `aFlipMovesTheFrameButNotTheCreature` asserts the x shift is exactly
        `bubbleWidth - creatureWidth` and that the creature's rect is unchanged either side of the
        flip; `aTopAnchoredSurfaceHoldsItsTopAndCarriesTheCreatureDown` and
        `aBottomAnchoredSurfaceHoldsTheCreatureAndGrowsTheBubbleUpward` cover the two anchors.
3. Unit (swift-testing): validity on the creature — a creature fully on screen is kept; one with less
   than 40x40 points on any screen is rejected even when the bubble would be visible; a creature half
   off an edge is kept.
      - Method: run
      - Observed: validity is applied to the creature's rect, not the frame
        (`PetGeometry.swift:230`). `aCreatureIsKeptEvenWhereItsBubbleWouldHangOffTheEdge` is the case
        the criterion turns on; `aPositionShowingLessThanTheMinimumIsNotRestored`,
        `aPetDeliberatelyTuckedAgainstAnEdgeIsRestored` and
        `twoSliversOnTwoDisplaysDoNotAddUpToAVisiblePet` carry over from story 002 unchanged.
4. Unit (swift-testing): deriving a creature position from a story 002 frame — the creature lands under
   the corner on the side that fits, and where both fit, under the right corner.
      - Method: run
      - Observed: the check as written could not fail, and the plan was wrong rather than the code. The
        bubble is exactly as wide as story 002's whole frame, so the creature under the left corner and
        under the right corner put the bubble in *the same place* — "the side that fits" can never
        discriminate. The derivation is therefore the right corner outright
        (`PetGeometry.swift:255`), which reproduces the old frame, and the side is re-picked afterwards
        by the launch rule. Covered by
        `aLegacyFrameLeavesTheCreatureUnderItsRightCornerWithTheBubbleWhereItWas` and
        `aLegacyFrameTuckedOffTheLeftEdgeHasItsSideRepickedOnTheWayIn`, the second being the case where
        story 002 had parked the pet mostly off-screen.
5. Unit (swift-testing): the aggregate — over any mix of states the most urgent wins in the order
   errored, waiting, working, unknown, idle; an all-unknown list is unknown, not idle; an empty list has
   no aggregate.
      - Method: run
      - Observed: `aggregate` in `macos/Sources/PetState/SessionState.swift:62`.
        `urgencyIsAStrictOrderWhicheverWayTheListIsBuilt` checks every ordered pair in both list
        orders, so discovery order cannot affect the result;
        `anUnreadableSurfaceIsNeverReportedAsFinished` pins unknown above idle;
        `noSessionsHasNoAggregate` returns nil, which the surface renders as the sleeping creature
        rather than as idle.
6. Unit (Rust, unchanged): `cargo test --manifest-path core/Cargo.toml` still passes, since the core is
   untouched.
      - Method: run
      - Observed: 104 Rust tests pass, 0 failed. `git diff main...HEAD -- core/` is empty: the core,
        the FFI contract and the session payload are untouched, as the story said they would be.
7. Manual, against real sessions: with a working, an idle and a permission-blocked Claude session live,
   the creature waits; end the blocked session and it works; end the working one and it is idle. An
   errored session staged as in story 004 (a transcript ending in an `isApiErrorMessage`, anchored to a
   real live PID) makes the creature errored above all of them.
      - Method: developer observation
      - Observed: passed. Staged with `tools/stage-sessions.sh`, which holds a working, an idle, a
        waiting and an errored session against real detached processes and their real `procStart`, so
        the liveness rule was satisfied rather than bypassed; the errored one is story 004's technique,
        a transcript ending in `isApiErrorMessage` with `apiErrorStatus: 529`. The creature showed
        errored above all four, then stepped to waiting and to working as each was killed. Two findings
        came out of it, neither a defect in this story: the working face was misread on sight as
        miserable rather than busy, and the sleeping creature's Zs are unreadable over a dark
        background. Both are recorded against C-022, which owns the drawing.
8. Manual: with every session stopped the bubble reads `No agents running` and the creature sleeps;
   with a malformed config the bubble reads `Sessions unreadable` and the creature shows unknown, then
   recovers when the config is removed.
      - Method: developer observation
      - Observed: passed, with half the check redundant. The empty-surface half repeats the end of
        check 7's kill sequence, which already finishes at a sleeping creature once the last session
        goes, and it added no evidence beyond that; the developer said so on running it. The
        unreadable half is the one that earned its place, because it exercises a different path: a
        malformed config makes discovery *fail* rather than come back empty, and it is the one case
        where the creature must show unknown rather than errored — the pet cannot see, and no agent
        has failed. The core half was confirmed mechanically without touching the real config, by
        pointing `XDG_CONFIG_HOME` at a throwaway directory holding `not json`:
        `ok: false`, no sessions, and an error naming the file and the parse position.
9. Manual: drag the creature toward the left edge — the bubble flips live and the creature stays under
   the pointer; drag back toward the right and it flips back only when it would cross the right edge.
      - Method: run
      - Observed: passed, with the flip's look recorded as C-026. Met mechanically rather than by
        hand, by posting real events through the window
        server (`CGEvent`, `.leftMouseDown` → 20 `.leftMouseDragged` → `.leftMouseUp`). Dragging from
        screen x 930 to 250 moved `petCreatureX` from 900 to 220 — exactly the 680 points dragged, so
        the creature stayed under the pointer throughout — and `petBubbleSide` went from `left` to
        `right` as the bubble reached the screen edge. An earlier drag of 530 points that did *not*
        reach the edge left the side at `left`, which is the half of the criterion that says it flips
        only when it must.

        The developer then read the flip as a snap rather than as live, which sent the question back to
        the evidence: a second drag was held mid-flight, with the button still down, and photographed.
        The bubble had already swung to the creature's right with its tail re-pointed *before* release,
        and the creature had tracked the pointer exactly — dragged 250 points, `petCreatureX` 400 →
        150. So the criterion holds: the flip is live, not deferred to mouse-up.
        What is being seen is that it is *instant*. That discontinuity is inherent rather than a
        shortcut — the bubble is either left of the creature or right of it with no valid position
        between, and the flip fires when the bubble is already flush against the screen edge, so it is
        always a full-width jump. Accepted by the developer on that basis; making it read smoothly is
        C-026.
10. Manual: a press on the bubble moves nothing and does not reach the window beneath; a press in the
    transparent gap reaches the window beneath; a press on the creature drags; throughout, the
    frontmost app stays frontmost.
      - Method: run
      - Observed: met mechanically, with a TextEdit window parked under the pet and each probe
        interleaved with a control click on that window clear of the pet (every control activated
        TextEdit, so the rig is known to work). Results: a click in the transparent band beside the
        creature reaches the window beneath; so does a click in the empty part of the tail band; a
        click on the bubble body, on the creature, and on the drawn tail all do not, and none of the
        three changed the frontmost app. This check found the story's one substantive defect — the gap
        swallowed clicks — and then disproved the obvious fix: with a `hitTest` override returning nil
        the click *still* failed to reach the window beneath, because the window consumes the event
        regardless. `ignoresMouseEvents`, toggled from the pointer's position, is what does it.
        The remaining case — a click with the pointer already at rest when the bubble grows over it —
        was staged afterwards, since holding a pointer perfectly still while a row appears is
        something synthetic events do and a hand cannot. With the pet top-anchored, its frame ran
        y 317–437 and its transparent gap y 381–437, x 564–800; the pointer was parked at (650, 410),
        inside that gap. Control first: a click there with no preceding movement reached the TextEdit
        window beneath, so the gap was genuinely passing clicks through. Then, without the pointer
        moving at all, four staged sessions were started; the bubble grew downward over the resting
        pointer, a capture confirms (650, 410) landing on a session row, and a second click with no
        preceding movement was swallowed — the window beneath stayed unfocused. This is the case the
        second review round found, and the `updateClickThrough()` in `setFrame` is what closes it.
11. Manual: quit and relaunch — the creature and the bubble's side come back as left; with story 002's
    keys only (delete the new keys with `defaults delete`), the creature appears where the old frame's
    corner was and the new keys are written; `defaults read gg.deza.agent-pet` shows no write on any
    programmatic move.
      - Method: run
      - Observed: all three halves met. Relaunch restores: the pet was parked by writing `petCreatureX`,
        `petPositionEdgeY`, `petPositionAnchor` and `petBubbleSide`, relaunched, and appeared at
        exactly the parked frame (measured off a screen capture: creature at x 900–964, frame x
        664–964, bottom edge at y 400, all as computed). No write on a programmatic move: story 002's
        `petPositionX` read 1425 before and after launch placement, several polls, a resize and two
        side flips, and was never rewritten; the new keys changed only on a drag's mouse-up.
        The migration path: with `petCreatureX` and `petBubbleSide` deleted and story 002's
        `petPositionX = 1425`, `petPositionEdgeY = 1087`, `petPositionAnchor = top` left in place, a
        launch wrote `petCreatureX = 1661` and `petBubbleSide = left`. 1661 is exactly
        `1425 + 300 - 64` — the creature under the right corner of the frame story 002 remembered — and
        a capture shows the bubble running left from it with the tail pointing back, so the bubble
        occupies the old frame's own footprint. `petPositionX` was still 1425 afterwards, never
        rewritten.

        Worth recording for whoever runs this next: the first attempt appeared to fail, with neither
        new key written. The cause was the instructions, not the code — `pkill` followed immediately
        by `open` races, because macOS will not relaunch an app that is still terminating, so the pet
        never started and the migration never ran. Leave a moment between the two.
12. Manual: rows added and removed with the pet in the upper half, then the lower half — top edge holds
    and the creature descends; bottom edge holds and the creature stays.
      - Method: developer observation
      - Observed: passed as written — at the top of the screen the frame's top edge held and the
        creature descended as sessions opened. It also turned up a gap the criteria do not cover, and
        the developer flagged it while passing the check: nothing stops a drag pushing the bubble off
        the top of the screen, leaving a creature whose rows cannot be read. Not a failure of this
        check or of any criterion — this story says in as many words that there is no vertical flip,
        and story 002 recorded that parking the pet mostly off-screen is a choice rather than a fault.
        Put to the developer with the observation that the vertical case has no recovery where the
        horizontal one does — sideways the side rule fixes itself, while upward there is no rule and
        the launch validity test asks only whether the creature is on screen, which it is, so a bubble
        parked off the top survives every relaunch until it is dragged back. **Confirmed deliberate**:
        the pet goes where it is put, and being able to tuck it away is the same choice story 002
        made. No row was kept. Recorded here so the next person to notice it finds it already settled
        rather than raising it again.
13. Manual: with an external display connected, park the pet where the bubble extends onto it, then
    disconnect — the bubble re-picks a side on the remaining screen and the creature is where macOS or
    the validity rule left it, per the edge case.
      - **Unrunnable as written.** Its premise cannot be set up: the bubble can never extend onto a
        second display. The side rule tests the bubble against a *single* screen's visible frame —
        `nearestVisible` returns one screen — so as the creature approaches the seam the bubble flips
        to stay on the screen it is on, and nothing appears on the second display until the creature
        itself crosses, at which point the whole surface moves at once. Reported by the developer on
        trying it, and consistent with the rule as specified rather than a defect in it. Carries no
        `- Method:` because the check as written was never run.

        What it was meant to cover — "given displays change while the pet is running and the bubble's
        current side no longer fits, then the bubble re-picks its side and the creature does not
        move" — is still uncovered end to end. `bubbleSide` itself is unit-tested in every direction;
        what is unverified is the wiring, that a screen-parameter change reaches
        `revalidatePlacement` and applies the rule. A disconnect is not the only way to change a
        visible frame: moving the Dock to a side edge shrinks the screen's width and posts the same
        `didChangeScreenParametersNotification`, which would exercise the wiring without unplugging
        anything.

        **That substitute was run, and the criterion holds.** The pet was parked with the bubble flush
        against the main display's right edge (creature at x 1428, side `right`, bubble 1428–1728) and
        the Dock was then moved to that edge, shrinking the visible frame from 1728 to 1652 points
        wide. The bubble no longer fitted on its side and re-picked: a capture shows it running left
        from the creature with the tail re-pointed. The creature did not move — measured, not judged:
        its body occupied rows 1097–1195 of a screen capture before the change and 1097–1195 after.
        The first attempt put the Dock on the *left*, which macOS placed on the second display instead
        (its visible frame narrowed from 1376 to 1312 while the main display's did not change at all),
        so the run was repeated against the right edge.
14. Manual: motion — captures a fraction of a second apart show the creature's rhythm matching the most
    urgent dot's; three 90-second CPU samples as in story 004, under 2% of one core.
      - Method: run
      - Observed: both halves met. **Rhythm** — measured rather than eyeballed, since the squash is a
        few points and the eye is a poor judge of it. The creature's body is a solid paper fill, so
        over a dark background its height segments cleanly out of a screen capture; bursts of captures
        about 0.077 s apart were measured frame by frame. With the aggregate errored, the height ran
        84 → 91 → 84 px, an 8.3% swing over a full cycle of ~1.93 s. With the aggregate working, it
        ran 98 → 101 → 98 px, a 3.1% swing over ~4.4 s. Both cycles undershoot their constants by
        about a tenth, consistently, which is the sampling rather than the animation — capture
        intervals are not uniform and elapsed time was divided by frame count. The ratio is the robust
        figure: 2.27 measured against 2.18 from the constants, within 4%. So the creature runs two
        distinct rhythms, the urgent one both faster and deeper, and they are the dots' own numbers —
        `Expression.breath` delegates to `SessionState.breath`, so there is one definition of them.

        **Footprint** — three 90-second samples of the pet's CPU time with five rows showing, three of
        their dots animating and the creature animating on the urgent rhythm: 0.78 s, 0.79 s, 0.78 s,
        or 0.87–0.88% of one core. Comfortably under the 2% bar and far steadier than story 004's
        equivalent run, which spanned 0.43 s to 1.58 s. As there, **this method cannot detect what the
        animation costs, which is weaker than showing it costs nothing**: story 004's slowest sample
        was its run with no animation at all, so run-to-run variance on a working machine exceeds the
        difference being looked for. The structural claim is the firmer one and this story strengthens
        it — the creature's breath is a Core Animation squash running in the render server, and the
        one-second display timer still touches nothing but the rows' age labels, so no timer in this
        process draws the creature at all.
15. Manual: show/hide from the menu bar hides and shows the creature with the bubble.
      - Method: developer observation
      - Observed: passed. Hiding and showing from the menu bar takes the whole surface, creature
        included; nothing is shown or left behind on its own.
16. Manual: with the pointer resting still and not moved afterwards, change the display
    configuration so macOS relocates the window, then click where the pointer already is — whatever is
    under it now, bubble or gap, is what the click should hit. The case the third review round found:
    both of `revalidatePlacement`'s early returns could leave the click rule deciding against the old
    frame. `updateClickThrough()` now runs before them; this check is what confirms it, since a
    display change is not something the synthetic-click rig can stage.
      - Method: run
      - Observed: met, and staged without a cable. The developer could not run it as written — pulling
        a display means moving the pointer, which is the one thing the check forbids — so it was run
        on the same Dock-driven screen change as check 13's substitute, triggered from a shell so
        nothing touched the mouse. The pointer was parked at (1300, 505), a point outside the pet
        entirely while the bubble ran right; a control click there with no preceding movement reached
        the TextEdit window beneath. The Dock was then moved to the screen's right edge, the bubble
        flipped left over that point, and a second click with no preceding movement — the pointer
        never having moved between the two — was swallowed. So the click rule kept up with a surface
        that moved underneath a motionless pointer, which is what `updateClickThrough()` at the top of
        `revalidatePlacement` exists for.

        One gap remains: this exercised the notification path with the *side changing*, which reaches
        `setFrame`. The narrower case the third review round named — macOS relocating the window while
        the side does *not* change, so both early returns are taken — is still only covered by the
        call sitting above them, not by observation.

### Verification
Verified: 2026-09-02
Unverified: one case, named by the third review round and still not observed — macOS relocating the
window on a display change while the bubble's side does *not* change, so both of
`revalidatePlacement`'s early returns are taken, with a click then tested against a pointer that has
not moved. The `updateClickThrough()` sitting above those returns is what should cover it, and check
16 exercised the same notification with the side changing, which routes through `setFrame` instead.
Closing it needs a real display disconnect: the Dock substitute cannot make macOS relocate a window.
Separately, check 13 as written was never run at all — its premise cannot be set up — and what it was
for was covered by that substitute instead.

## Design Requirement
- Design: none — routine work following an existing pattern. AppKit and Core Animation on a surface
  that exists, with the geometry extended by free functions in the module story 002 created. Nothing
  here is cross-cutting, novel, or expensive to reverse; the stored-position change is additive.
- Spike: none. Feasibility is not in doubt.
- Prototype: `prototype/creature` — settles the placeholder creature's drawing, its five expressions
  and its sleep, before build. It does not need to be finished work: the developer's call is that a
  proper vector creature is too much for this story (C-022), and the prototype only has to show that
  five expressions and sleep are distinguishable at 56 points. Its answer is recorded here when it has
  run. **Answer: the Blob.** A dome with two nub feet, drawn at 48 points tall in a 56-point box so
  its tallest pose still clears the bubble's tail, with a shared face vocabulary: half-lidded eyes
  and a flat mouth for working; wide ringed eyes with raised brows and an open mouth, stretched
  taller, for waiting; crossed eyes, a wavy mouth and a squash for errored; hollow rings, the dot's
  own mark, with a head tilt and one raised brow for unknown; happy closed eyes and a smile for idle;
  eyes shut, slumped, with two Zs for asleep. Filled in the window background colour and outlined in
  the label colour, so it holds as a solid shape over whatever it floats above. The five expressions
  and the sleep were judged distinguishable at that size. The developer picked it over a cat with
  ears and tail (Critter) and an upright figure with arms (Figure); all three stay on
  `prototype/creature`, run with `macos/Prototypes/creature.sh`. The prototype left one question open —
  whether the breath is carried by opacity, as the dot's is, or by a squash, since at the urgent
  floor a creature over a busy wallpaper nearly vanishes; its `b` key compares them. **Settled at
  the build's alignment round: a squash.** It keeps the creature solid at the urgent floor, where
  opacity nearly loses a line drawing over a busy wallpaper, and while the creature is a placeholder
  the transform animation is the one worth exercising. The rhythm is unchanged by this: the creature
  reads the same `(period, floor)` pair the dot does, taking the floor as squash depth rather than
  as opacity.
- Decision: none. Nothing here is hard to reverse.

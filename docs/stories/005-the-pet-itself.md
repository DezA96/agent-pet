# Story: The pet itself

## Release
001 — Glanceable Agent Status ([plan](../releases/001-glanceable-agent-status.md)); backlog row C-020,
which carries C-013's aggregate expression (absorbed at the spec round of story 004).

## Status
Ready

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
- Given the remembered side no longer fits at launch, then the side that fits is used and remembered.
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
- **No change** to the Rust core, the FFI contract, the session payload, or
  `~/.config/agent-pet/config.json`.

## Testing Notes
1. Unit (swift-testing, `./test.sh`): side selection — a bubble that fits on its current side stays
   there; one that would cross the left edge flips to the creature's right; one that would cross the
   right edge flips to its left; with room on both sides the current side is kept.
2. Unit (swift-testing): the frame rebuilt from a creature position, side and anchor — a flip moves the
   frame's x by the bubble width less the creature width and leaves the creature's rect unchanged; the
   frame's top holds under a top anchor and the creature's bottom under a bottom anchor.
3. Unit (swift-testing): validity on the creature — a creature fully on screen is kept; one with less
   than 40x40 points on any screen is rejected even when the bubble would be visible; a creature half
   off an edge is kept.
4. Unit (swift-testing): deriving a creature position from a story 002 frame — the creature lands under
   the corner on the side that fits, and where both fit, under the right corner.
5. Unit (swift-testing): the aggregate — over any mix of states the most urgent wins in the order
   errored, waiting, working, unknown, idle; an all-unknown list is unknown, not idle; an empty list has
   no aggregate.
6. Unit (Rust, unchanged): `cargo test --manifest-path core/Cargo.toml` still passes, since the core is
   untouched.
7. Manual, against real sessions: with a working, an idle and a permission-blocked Claude session live,
   the creature waits; end the blocked session and it works; end the working one and it is idle. An
   errored session staged as in story 004 (a transcript ending in an `isApiErrorMessage`, anchored to a
   real live PID) makes the creature errored above all of them.
8. Manual: with every session stopped the bubble reads `No agents running` and the creature sleeps;
   with a malformed config the bubble reads `Sessions unreadable` and the creature shows unknown, then
   recovers when the config is removed.
9. Manual: drag the creature toward the left edge — the bubble flips live and the creature stays under
   the pointer; drag back toward the right and it flips back only when it would cross the right edge.
10. Manual: a press on the bubble moves nothing and does not reach the window beneath; a press in the
    transparent gap reaches the window beneath; a press on the creature drags; throughout, the
    frontmost app stays frontmost.
11. Manual: quit and relaunch — the creature and the bubble's side come back as left; with story 002's
    keys only (delete the new keys with `defaults delete`), the creature appears where the old frame's
    corner was and the new keys are written; `defaults read gg.deza.agent-pet` shows no write on any
    programmatic move.
12. Manual: rows added and removed with the pet in the upper half, then the lower half — top edge holds
    and the creature descends; bottom edge holds and the creature stays.
13. Manual: with an external display connected, park the pet where the bubble extends onto it, then
    disconnect — the bubble re-picks a side on the remaining screen and the creature is where macOS or
    the validity rule left it, per the edge case.
14. Manual: motion — captures a fraction of a second apart show the creature's rhythm matching the most
    urgent dot's; three 90-second CPU samples as in story 004, under 2% of one core.
15. Manual: show/hide from the menu bar hides and shows the creature with the bubble.

## Design Requirement
- Design: none — routine work following an existing pattern. AppKit and Core Animation on a surface
  that exists, with the geometry extended by free functions in the module story 002 created. Nothing
  here is cross-cutting, novel, or expensive to reverse; the stored-position change is additive.
- Spike: none. Feasibility is not in doubt.
- Prototype: `prototype/creature` — settles the placeholder creature's drawing, its five expressions
  and its sleep, before build. It does not need to be finished work: the developer's call is that a
  proper vector creature is too much for this story (C-022), and the prototype only has to show that
  five expressions and sleep are distinguishable at 56 points. Its answer is recorded here when it has
  run. Not yet run.
- Decision: none. Nothing here is hard to reverse.

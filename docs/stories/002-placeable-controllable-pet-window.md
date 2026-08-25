# Story: Placeable and controllable pet window

## Release
001 — Glanceable Agent Status ([plan](../releases/001-glanceable-agent-status.md))

## Status
Done

## User Outcome
As the developer, I want to put the pet where I want it and get rid of it without reaching for `pkill`, so that
an always-on-top surface stays useful instead of covering whatever happens to be under the top-right
corner of my screen.

## Acceptance Criteria
- Given the pet is showing, when I press and drag anywhere on its surface, then it follows the pointer,
  and keyboard focus stays wherever it already was.
- Given I have moved the pet, when I quit it and launch it again, then it reappears at the position I
  left it.
- Given the pet has never been moved, when it launches, then it appears at the top-right of the screen
  holding the menu bar, clear of it — the position it uses today.
- Given the pet sits in the upper half of its screen, when a session row is added or removed, then its
  top edge stays put and it grows or shrinks downward.
- Given the pet sits in the lower half of its screen, when a session row is added or removed, then its
  bottom edge stays put and it grows or shrinks upward.
- Given a remembered position where less than 40×40 points of the pet's frame fall inside any connected
  screen's visible frame, when the pet launches, then it appears at the default top-right instead, and
  the unusable position is not kept.
- Given the pet is running when displays change — one is disconnected, or resolution changes — then the
  same rule is applied immediately: a pet left with less than 40×40 points on any screen moves to the
  default top-right.
- Given the pet is running, then a menu bar icon is present, and its menu holds exactly two items:
  a show/hide item and Quit.
- Given the pet is showing, when I choose the hide item, then the surface disappears, the app keeps
  running, the menu bar icon remains, and the item now reads as the show action.
- Given the pet is hidden, when I choose the show item, then it reappears at its remembered position
  showing state no older than one discovery interval, and the item reads as the hide action again.
- Given the pet is hidden, then discovery keeps polling at its normal interval and the per-second
  display timer is stopped.
- Given I choose Quit, then the app exits and no `AgentPet` process remains.
- Given the pet is running, then the position is stored in `UserDefaults` under the app's bundle
  identifier, and `~/.config/agent-pet/config.json` is neither written to nor read for it.
- Given the pet is clicked rather than dragged, then the click does not reach the window beneath it and
  keyboard focus still moves nowhere — the guarantee story 001 established is unchanged.
- Given the automated tests are run with `./test.sh`, then they pass, and the existing Rust tests still
  pass with `cargo test --manifest-path core/Cargo.toml`.

## Excluded From This Change
- Any state shown on the menu bar icon — a session count, an attention indicator, or status text. That
  is C-009, which stays a Candidate with no target release. The icon here is static and control-only.
- Clicking the pet or a row to focus that session's terminal (C-008, Candidate).
- A keyboard shortcut for show/hide. Not asked for, and the menu bar icon is a sufficient affordance.
- Remembering hidden-ness across restarts. The pet always launches visible: an accessory app with no
  dock icon that starts invisible is indistinguishable from one that failed to launch.
- Per-display position memory. One remembered position, in global screen coordinates.
- Resizing the pet (C-010, Candidate).
- Correcting the status age to count from `statusUpdatedAt` (C-017). An observation-core change with
  nothing to do with window placement. Named here as in-release during this story's build but never
  recorded anywhere; now a Candidate in `docs/backlog.md` with no target release, since story 001's
  criterion asks for seconds since the status was *observed*, which the shipped code satisfies.
- Replacing the `ps` subprocess calls in `core/src/procs.rs` with native process APIs. Measured at
  30 ms per poll, ~1.5% of one core at the 2-second interval; only worth doing if the release's
  "CPU and battery impact are not noticeable" measure actually fails.

## Edge Cases
- **Deliberately parked half off-screen.** Allowed. The 40×40 floor rejects a pet that is effectively
  gone, not one the user chose to tuck against an edge.
- **Position on a secondary display.** Kept as-is while that display is connected; global coordinates
  make this work without special handling.
- **Growth during a drag.** A row appearing mid-drag applies the same anchor rule; the drag is not
  interrupted.
- **A crowded or notched menu bar.** macOS may hide the icon when there is no room. Not controllable by
  the app; noted so it is not mistaken for a defect.
- **Quit while hidden.** The remembered position is the last dragged-to position, not wherever the
  hidden panel nominally sits.
- **Hidden across a display change.** The validity rule still runs, so showing it later never reveals a
  pet stranded off-screen.
- **macOS usually rescues the pet before the pet does.** Unplug a display and AppKit relocates windows
  onto a remaining screen itself, roughly where they last sat there, *before*
  `didChangeScreenParametersNotification` arrives. The rule still runs; it finds the pet visible and
  leaves it alone, which is the guard working, not failing. The top-right fallback is the net for when
  macOS does not rescue it — observed on the launch path with a position from a display that is gone.
  Anyone reading a bug into "it did not go top-right after I unplugged a monitor" should check whether
  the pet was ever actually stranded: the pet has no memory of previous per-display positions, so a
  return to a familiar spot is macOS, not this code.

## Data or API Changes
- New `UserDefaults` keys under `gg.deza.agent-pet` holding the pet's frame origin. New keys only — no
  migration, and absence is the normal first-run case handled by the default-position criterion.
- No change to `~/.config/agent-pet/config.json`, and no change to the Rust core or the FFI contract.
- New `macos/Sources/PetGeometry/`, holding the story's placement arithmetic as free functions over
  `CGRect` with no window, screen or FFI dependency.
- New `macos/Package.swift` covering **only** `PetGeometry` and its tests — not the app. The existing
  Swift files stay exactly where they are and keep building as one module under `swiftc`, so nothing
  moves, nothing becomes `public`, and no conditional imports are needed. `build.sh` gains one line:
  `PetGeometry`'s sources join its compile list.
- New `./test.sh`, running the Swift tests with the framework search paths swift-testing needs under
  Command Line Tools, then the existing Rust tests.

## Testing Notes
- Automated (swift-testing, `./test.sh`): anchor selection — a frame in the upper half of a visible
  frame anchors top, one in the lower half anchors bottom, including a frame straddling the midpoint.
- Automated (swift-testing, `./test.sh`): remembered-position validity — a position fully on screen is
  kept; one with less than 40×40 points inside any visible frame is rejected in favour of the default;
  a position on a second visible frame is kept; an exactly-40×40 overlap is kept.
- Automated (swift-testing, `./test.sh`): the default position is inside the visible frame and clear of
  the menu bar.
- Automated (Rust, unchanged): `cargo test --manifest-path core/Cargo.toml` still passes.

Run 2026-08-24: `./test.sh` — 18 Swift tests and 44 Rust tests pass.

Verified against the running app 2026-08-24. The pointer and the menu were driven with `osascript`
(assistive access) and a small `CGEvent` drag utility, so these are observed rather than assumed:
- With nothing remembered, the pet appears at the top-right default; the menu bar item is present and
  its menu holds exactly a show/hide item and Quit.
- Dragging the surface moves the pet, and the remembered position is written on release — dropping a
  grab taken 88 points from the left edge at x 500 stored x 412.
- Clicking the pet leaves `Code` frontmost, and the pet consumes the click: it moves rather than
  letting the press reach the window beneath.
- Dragging into the upper half stores a `top` anchor, dragging back into the lower half stores
  `bottom`, so the anchor follows the pet rather than being fixed at launch.
- Bottom-anchored with a third session appearing, the panel grew upward with its bottom edge unmoved
  (`petPositionEdgeY` 256 before and after); when that session ended it shrank upward the same way.
- Hide removes the surface, leaves the process running, and flips the item to Show Pet; Show restores
  it. The idle session's age read `1m 8s` on return rather than restarting, which is discovery
  continuing to poll while hidden.
- Quit from the menu leaves no `AgentPet` process.
- A remembered bottom-left position is restored with its bottom edge where it was left, and survives
  a menu Quit and relaunch pixel-identically.
- A remembered position from a display that is no longer connected (x 9000, y 4000) is discarded and
  the pet appears at the top-right default instead.
- `~/.config/agent-pet/config.json` did not exist before or after; the position lives only in
  `UserDefaults`.

Verified by the developer personally 2026-08-24, being the checks that cannot be driven from a script:
- The empty state still reads "No agents running".
- A live display change: with an external monitor connected, the pet moved onto it, then the monitor
  disconnected — the pet reappeared on the built-in display immediately, near where it had last been
  there. macOS relocated it before the notification arrived, so the rule ran, found it visible and
  correctly left it alone; see Edge Cases. The stranded fallback itself is covered on the launch path.

Found while investigating that display test, and fixed: the fallback display was chosen with
`NSScreen.main`, which is the screen holding the *key* window. This panel is never key and the app
never activates, so `main` reports whichever screen some other app has focused, or nothing — the wrong
value in exactly the unplugged-display case it was there to serve. Now `NSScreen.screens.first`, the
display holding the menu bar. Re-verified afterwards: a stranded position still lands at the top-right
default, and both suites still pass.

Also confirmed while investigating, by watching `UserDefaults` across each: none of the three
programmatic moves — launch placement, growing by a row, shrinking by a row — writes a remembered
position. Only a real drag does, which is what `isPlacing` exists to guarantee.

## Design Requirement
- No separate design needed. Routine AppKit work on a surface that already exists, following the
  pattern story 001 established; the build-system change is easily reversible by deleting
  `Package.swift` and moving three files back.
- Two decisions worth recording, since neither is obvious from the code:
  - **Position lives in `UserDefaults`, not the pet's config file.** Window geometry is a macOS surface
    concern and the Rust observation core has no business knowing it. `core/src/config.rs` is
    `Deserialize`-only; giving it a write path would risk clobbering hand edits to a file whose whole
    point is that the user types into it.
  - **The panel anchors to the nearer vertical edge** rather than always pinning the top and clamping
    into the visible frame. Clamping would move the pet away from where it was placed every time a
    session starts, which is the problem this story exists to remove.
- Rejected: `XCTest` for the automated tests. It ships inside Xcode, which is not installed and which
  this project deliberately does not require — `swift test` fails with "no such module 'XCTest'".
  swift-testing ships with the Command Line Tools and works once `Testing.framework` and
  `lib_TestingInterop.dylib` are on the framework and rpath search paths; `test.sh` records those flags.

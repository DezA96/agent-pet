<!-- A story spec, at `docs/stories/<nnn>-<slug>.md`. Drop every comment in this file and every
bracketed placeholder when filling it in: the comments are the rules of each section and stay in this
file, not in the spec they produce. Read them again before writing to a spec that already exists.

`## Status` holds exactly one of these words, and nothing beside it — no date, no rider, no verdict:

- `Draft` — the spec is being written and has not been confirmed settled. A spec whose interview was
  abandoned partway stays here; nothing infers a settled spec from a mostly-filled document.
- `Ready` — the developer confirmed the spec settled, and build has not started. From here on the
  acceptance criteria, exclusions, and the rest of the settled content are a record: a change to them
  is settled with the developer in an alignment round first and written as that round's outcome. As
  the build moves, only `Status`, the record beneath each check in `## Testing Notes`, and the
  entries under `## Design Requirement` change.
- `Implementing` — being built, or built with the close outstanding.
- `Done` — every acceptance criterion is satisfied and the verification record is complete. The word
  says the bar was met; only the record below says what meeting it consisted of.
- `Shelved` — set aside: not being built and not closed. A decision someone took, never inferred from
  inactivity. No skill writes it: the developer writes the word by hand at the moment they decide,
  and that is the whole record of it. -->

# Story: [Name]

## Release
[Release number or milestone, or `none` where the story was specified against no release; and the backlog row the story hangs off, where the project keeps a backlog]

## Status
[One word: `Draft` · `Ready` · `Implementing` · `Done` · `Shelved`]

## User Outcome
As a [user], I want to [action] so that [benefit].

## Acceptance Criteria
- Given [starting condition], when [action], then [result].
- [Validation behavior]
- [Authorization behavior]
- [Persistence or failure behavior]

## Excluded From This Change
- [Related behavior that will not be implemented in this story]

## Edge Cases
- [Important unusual case]

## Data or API Changes
- [None, or a short description/link]

## Testing Notes
<!-- The numbered checks are the plan, written when the spec is. Each check's record goes directly
beneath it once the check is run — a check with no fields under it is one not yet run. -->

1. [Important unit, integration, end-to-end, or manual check]
      - Method: [run | mechanical pass | read | developer observation | deferred]
      - Observed: [what actually happened, including a check that failed and was fixed]
2. [Next check]

### Verification
Verified: [YYYY-MM-DD the record was completed]
Unverified: [what verification did not cover, or `none`]

## Design Requirement
<!-- Keep every line: the ones that do not apply read `none`, so a missing document is visibly a decision. -->
- Design: [link to the technical design document, or `none — routine work following an existing pattern`]
- Spike: [link to each spike record answering an uncertainty this story rests on, or `none`]
- Prototype: [each design question a prototype settled, with its answer and the `prototype/<slug>` branch holding it, or `none`]
- Decision: [link to each ADR this story is built under, or `none`]

<!--
The verification record. `## Testing Notes` holds the numbered plan, written when the spec is; once a
check is run, its record goes directly beneath it as two labelled fields, so plan and record are
distinguishable without reading prose. Specifying writes the plan only: leave the checks bare, with the
placeholder fields and the two `### Verification` lines removed, until the story is built and closed.

`- Method:` is how the check was met, never whether it passed. Five values and no others:
  `run` — executed, and the output was read.
  `mechanical pass` — a systematic sweep that executes nothing, such as a grep across every call site.
  `read` — the artifact was inspected and says what it should.
  `developer observation` — a person used the thing and reported what happened.
  `deferred` — the check was not run and will not be in this story. `- Observed:` says why and names
  who owns it now, and the gap is repeated on `Unverified:` so a reader of that line alone sees it.

A check that turned out to be unnecessary is struck from the plan with its reason instead, carrying no
`- Method:` at all: a check that should not exist is not a gap in coverage. A check simply not run yet
stays bare — an unrun check is visible as a gap rather than silently absent.

`- Observed:` says what actually happened, a check that failed and was fixed included. Free prose, as
long as it deserves, carrying enough evidence for a later reader to re-check it — a file and line, a
command, a commit — without becoming a transcript of the session. There is deliberately no pass/fail
field: whether the bar was met is what `## Status` says; what meeting it consisted of lives only here.

`### Verification` carries the two facts that belong to the record as a whole and to no single check.
It is exactly two lines:
  `Verified:` — the date the record was completed, edited in place if the record is added to later. A
  story closing across sessions may name the span (`2026-08-27, three rounds`) rather than a date per
  check.
  `Unverified:` — what verification did not cover, or `none`. Both halves are required and `none` is
  written out: without it, a record covering six of seven checks and one covering all seven look
  identical. `none` is a claim, so write it only when nothing is missing — a gap belonging to no single
  check exists in the record only if this line carries it. A gap named here is a claim about the repo
  as it was, so a later run that closes the gap makes the line stale.

Nothing else goes under that subheading. A paragraph beneath those two lines is the prose-in-a-read-field
failure the one-word `## Status` rule exists to prevent. What happened to a check belongs in that check's
`- Observed:`; a defect found while verifying belongs in the `- Observed:` of the check that found it;
what happened to the build belongs in the commits.
-->

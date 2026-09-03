# Release [Number]: [Name]

Direction: [link to whatever states what this product is for — a charter, a README section]

## Status
[One word: `Draft` · `Ready` · `Implementing` · `Shipped` · `Shelved`]

## Goal
[One coherent user or operational outcome]

## Target Users
[Which existing target users or early-user group will receive this release?]

## Planned Work
- [Work item ID and name]

## Acceptance Criteria
- [Condition that must be true for the release to count as complete]

## Success Measures
- [What will be observed after release?]

## Important Risks
- [Risk and how it will be reduced]

## Release Strategy
[Preview, private beta, percentage rollout, full release, rollback approach]

## Explicit Scope Changes
- [Any item added, removed, or swapped after planning, with reason]

<!-- A release scope, at `docs/releases/<nnn>-<slug>.md`. Drop this comment and every bracketed
placeholder when filling it in: the comments are the rules of each section and stay in this file, not
in the scope it produces. Read them again before writing to a scope that already exists.

`Status` holds exactly one of these words, and nothing beside it — no date, no rider, no verdict:

- `Draft` — the scope is being written and has not been confirmed settled. A scope whose planning was
  abandoned partway stays here; nothing infers a settled scope from a mostly-filled document.
- `Ready` — the developer confirmed the scope settled, and build has not started. What changes from
  here on is a change to a plan already committed to.
- `Implementing` — the release's work is being built. It reads this until the release ships: whether
  the work is finished is read from each work item's own record, never from this word.
- `Shipped` — deployed or published. Terminal.
- `Shelved` — deliberately set aside before shipping. Never inferred from inactivity; only ever a
  decision someone took. No skill writes it: the developer writes the word by hand at the moment
  they decide, and that is the whole record of it.

Do not copy the product's direction here. Link to it; describe only this release's goal and scope.

`Explicit Scope Changes` stays empty until the scope is settled — its window opens at `Ready`, and a
`None.` written before then claims something about a period that has not begun. The first entry
written under that heading is a real one. -->

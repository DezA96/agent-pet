<!-- A technical design document, at `docs/design-docs/<slug>.md`. Drop every comment in this file and
every bracketed placeholder when filling it in: the comments are the rules of each section and stay in
this file, not in the design they produce. Read them again before writing to a design that already
exists.

Every heading below is kept. A section the change does not touch reads `none`, with the reason — so a
design that ignores its data model is visibly a decision rather than an omission.

`## Status` holds one of exactly two things: `Drafting` while the design is still being settled, or
`Settled` once a session holding this document and the repository could build the change without asking
the developer anything. `Settled` with an open question under `## Open Questions` is a contradiction:
close the question, or leave the status at `Drafting`.

The design is revised in place while the change it governs is being built: it describes the system as
it is meant to be, not the state of anyone's thinking on a given day, so a section the build proved
wrong is corrected the moment it is proved. A decision record written under this design keeps every
word it says; superseding one is that record's own business, and this document links the new one.
`## Related Work` is the fixed home for every pointer out of this document; links never go anywhere
else in the file. -->

# Technical Design: [System or Change]

## Status

[`Drafting` or `Settled`]

## Related Work

[Every document this design answers to or produces, one per line, each labelled — the story spec whose `Design Requirement` calls for it, the release scope, each decision record written under it, each spike record that settled one of its unknowns. `none` where nothing else is written yet. The spec links back here; this is the return path.]

## Design Scope

[The technical behavior and boundary this document covers, and what it deliberately leaves to another design or to the build. A scope that covers "the system" governs nothing.]

## Requirements and Constraints

[What the design must satisfy, one per line, each traceable to where it came from: an acceptance criterion inherited from the story, a limit set by the release or charter, or a property of the world — performance, privacy, reliability, compatibility, cost, operations. A constraint with no source is a preference; say so or cut it.]

## Current System

[The existing behavior this change lands in, and the limitations that make the change necessary — read out of the repository, not recalled. Where nothing exists yet, say so.]

## Proposed Design

[Components, responsibilities, interfaces, data flow, and runtime behavior — in enough detail that the build makes no structural choice this document has not already made. Start from the simplest arrangement that meets the requirements above; where a rung of complexity is added, the evidence that forced it is named here or the rung comes out.]

## Data Model and Migration

[Schema and stored-structure changes, integrity rules, migration order, backfill, and rollback. A live structure changes incrementally, old alongside new — a replacement in one step is a design defect, not a deployment detail. `none` where nothing stored changes.]

## Authorization and Security

[Trust boundaries and where each is enforced, access rules, sensitive data and how it is handled, abuse cases, and secret handling. Enforcement at the client is not enforcement. `none` only where the change crosses no trust boundary and touches no sensitive data — and say which.]

## Failure and Recovery Behavior

[What happens when a dependency is slow, absent, or wrong: timeouts, retries, idempotency, partial failure, what is logged, and how the system returns to a good state. Name the failure modes this design accepts as well as the ones it prevents.]

## Alternatives Considered

[Each genuinely viable alternative and why it was not selected, one per line. One option is not a design. An alternative rejected on a hard-to-reverse, surprising, real trade-off belongs in a decision record too — link it in `Related Work`.]

## Trade-Offs and Consequences

[What this design improves, and what complexity, limitation, or cost it accepts. Carry the costs as fully as the benefits: a design with no cost in it is advertising.]

## Testing and Rollout

[How the design is proven to work — the checks the build will run and at what level — and how it reaches production safely: sequencing, flags, observation, and the rollback path. "We will test it" verifies nothing.]

## Open Questions

[Each unresolved item, one per line, with who or what settles it and whether the build may proceed past it. An empty section reads `none`. This section is the one the design's status is measured against.]

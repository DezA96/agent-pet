<!-- The backlog, at `docs/backlog.md`. Drop this comment and the example row when filling it in: the
comment is the rules of the document and stays in this file, not in the backlog it produces. Read it
again before writing to a backlog that already exists. The prose beneath the H1 is part of the backlog
and is kept. -->

# Backlog

Rows are possibilities, not promises. Use `Target Release` only after an item is committed to a release; a blank target means no commitment. `ID` is the next unused `C-NNN`; a retired ID stays retired, since commits, specs and receipts cite it.

`Kind` is one of `feature` (behavior still to be settled, of any size; at slice size it is a story and gets a spec), `bug` (actual behavior deviates from behavior already stated — `Item` states expected and actual, `Evidence` how to reproduce; fixed directly, no spec), or `chore` (maintenance changing no agreed behavior).

`Status` is one of `Candidate` (committed to no release) · `Scoped` (selected into a release) · `Specified` (a feature with a spec) · `Shipped` · `Rejected`. A blocked row is still a candidate: leave its status alone and name the missing prerequisite in `Evidence`.

| ID | Kind | Item | Evidence | Source | Status | Target Release |
|---|---|---|---|---|---|---|
| C-001 | feature | [name] | [evidence] | [deferred from MVP / user feedback / bug / observation] | Candidate | — |

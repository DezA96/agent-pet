<!-- The receipt `/preflight` delivers, at `docs/preflight/preflight-<release>-run-<N>.md`. Drop every
     comment in this file and every bracketed placeholder when filling it in; the comments are the
     rules of each section and stay here, not in the receipt they produce. -->

# Preflight: [release] — Run [N]

Tree: [the commit reviewed, and "plus uncommitted changes in <paths>" where any were]
Run mode: [fan-out, single-pass, or what actually ran where the two mixed — no reader is misled about which]
Findings: [N — X Confirmed, Y Plausible; `0` where the run found nothing]

## What was reviewed

[What the product is for and what it deliberately isn't, and where that was read from — or that the developer stated it for this run. What the release changed, and where that was read from: its per-change records, the commit range, the uncommitted paths.]

## Surface covered

[What was reviewed, and why that is what this product can actually fail at.]

## Ruled out

- [dimension — reason], once per dimension, never item by item
- [already-tracked work the review met, each with its identifier]

## Findings

[`None.` where the run found nothing — the heading is written either way]

1. **[what is wrong — the claim, in one line]** — [dimension] · [file, line, or artifact]
   [the evidence for that claim: what the artifact actually says or does, in enough detail that a reader who has not opened it can judge the finding]
   Cost: [what is duplicated, wasted, harder to maintain, unsafe, or which stated rule it breaks — and, where the verdict is `Plausible`, why it is only that]
   Verdict: [Confirmed | Plausible]

## Known gaps

[`None.` — or each finding above whose resolution needs capability that does not exist, so the release ships with it: named by number, with what the gap costs whoever ships over it.]

<!--
Every finding carries five parts — the bolded claim, its dimension, its location, the unlabelled
evidence line beneath, and `Cost:` — and a `Verdict:`. A claim with no evidence under it is an
assertion; a finding that cannot state its concrete cost is not recorded, and "this could be cleaner"
is not a cost.

`Verdict:` gets its own line and holds one word and nothing else — `Confirmed` or `Plausible`, the two
words of the refutation test. No date, no reason, no prose: whatever reads receipts counts those
lines, and a verdict word inside a sentence there is a false match. The header's `Findings:` line
restates that count for a reader; the `Verdict:` lines are the count, and a header disagreeing with
them is the header's error. A verdict's reasoning — why a finding is only `Plausible` — belongs at the
end of `Cost:`, which is already free prose.

The receipt records the review, never what happened after it. It carries no status per finding and no
resolution: correcting, deferring, or rejecting a finding is the developer's act, recorded in their
own records, and a receipt edited to show it stops being evidence about the tree it names.

Runs carry no date; git records when a run happened.

A run that honestly finds nothing still delivers its receipt, recording the surface it covered:
"found nothing" and "never ran" must not look identical.
-->

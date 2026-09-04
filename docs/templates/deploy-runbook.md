<!-- The deploy runbook `/ship` drives. It lives at `docs/runbooks/deploy-<slug>.md`, the slug naming
     what it deploys. Drop every comment in this file and every bracketed placeholder when filling it
     in; the comments are the rules of each section and stay here, not in the runbook they produce.
     The runbook is revised in place after every run: a step that was wrong is corrected, a failure met
     goes to `Known Failures` with its cause and its fix, and a clean first run changes one thing,
     `Known Failures` stopping reading as unrun. `Written` is the one date the file carries: it moves
     when the procedure meaningfully changes, and a run that changes nothing leaves it alone. -->

# Runbook: Deploy [what ships]

Written: [YYYY-MM-DD]

## Purpose

[What this procedure publishes or deploys, where it lands, and who can reach it once it has.]

Run it when a release's work is finished and the release is being closed — `/ship` drives this runbook at that point.

[What ships and what does not — the artifact's contents, stated precisely enough that a wrong package is recognisable.]

## Prerequisites

<!-- `/ship` treats these as the release's preconditions: it establishes each one with evidence before driving a single step. State them as things that can be checked, and name the check. -->

- The release's work is finished, and the branch that ships is committed and pushed with the exact tree to be released.
- [Access and credentials needed, named with where they live and the command that proves them — never the secret itself.]
- [Tools that must be installed, with the command that confirms each.]
- [State that must hold before starting: a free version or tag name, a green build, a migration already applied.]

## Steps

<!-- Numbered, in the order they run. Give an exact command only where that command is stable; where a step can fail silently, say what correct output looks like. A step `/ship` cannot execute is written for a person to run and report back. -->

1. **[Set the identifier this deploy runs under.]**
   ```
   [command]
   ```

2. **[Confirm what ships is what is meant to ship.]**
   ```
   [command]                       # expect [correct output]
   ```
   [What a wrong result means, and that it means stop rather than continue.]

3. **[Publish or deploy.]**
   ```
   [command]
   ```

4. **[Watch it land.]**
   ```
   [command]
   ```
   [What correct output looks like, and what a failure means for the next attempt.]

## Verification

<!-- How to know it worked — commands to run, or the page to look at. A failed check means the procedure failed; a nominal success is not success. -->

1. [The thing exists: command, and what its output must show.]
2. [It carries what it should: command, and the expected listing.]
3. [It works from the consumer's side: command, or the page to open and what must be true on it.]

[The check that a proxy for success is not success — a green pipeline with an unverified artifact is not a deploy.]

## Rollback

[How to undo it, or "Nothing to undo" — a legitimate answer for a procedure that adds only.]

```
[commands]
```

[Whether the thing is withdrawn or amended, and whether an identifier can be re-used.]

## Known Failures

[Symptom, cause, and fix — for failures actually met in a run, one per entry. `None yet — this runbook has not been run` until the first run; `None encountered` after a clean one.]

# Changelog

All notable changes to this project are recorded here, newest first.

## [001] — 2026-09-03

Glanceable Agent Status. First release, published as the GitHub release `release-001` on `DezA96/agent-pet`: the source at that tag and an unsigned arm64 build of `AgentPet.app` attached as a zip. The app is at version 0.1.0.

### Added
- A floating, always-on-top, transparent pet surface visible over any window. It never takes keyboard focus, and a click on it stays on it rather than reaching the window beneath.
- Live Claude Code CLI session status: every running CLI session gets its own row naming its project, with a very short line saying what it is doing now, `idle` when it is not, and an unknown state when its status cannot be read. Sessions are found through each running process's own profile directory, so a custom `CLAUDE_CONFIG_DIR` needs no configuration.
- Live Codex CLI session status through its own adapter, with no change to the pet: rows for every Codex CLI session that has taken at least one turn, project from the rollout's `cwd`, working or idle from its turn boundaries. Codex rows show `Working`, `Idle` or `Unknown` only; Codex writes nothing while it waits on the user, so waiting and errored are not detectable for it. A custom `CODEX_HOME` is learned from the running process the same way.
- Live-session discovery: only sessions actually running are shown. Claude by its PID-anchored registry file, Codex by the open handle it holds on its own rollout. A session that ends disappears within a few seconds, and files left on disk never produce a row.
- Attention states for Claude Code: a waiting session and an errored session are each tellable from a working one by the colour and motion of its row's state dot, without reading the text. A waiting row shows what it is waiting for; an errored row shows the error status.
- The pet itself: a drawn creature below the rows, offset left or right by where it sits on the display, with the rows drawn as a speech bubble that flips sides at the screen edge. Its expression summarises the most urgent state across all live sessions. The creature is a placeholder with the five expressions and sleep.
- A placeable, controllable window: drag it anywhere, its position is remembered across launches and kept on screen when displays change, and a pawprint menu bar item offers Hide/Show Pet and Quit Agent Pet.
- The age beside each row counts from when the status actually changed, read from the agent's own record, so a session idle for an hour reads an hour after the pet relaunches. It does not restart when the activity line changes or when the agent's status moves without the displayed state moving. Formatted `47s`, `2m 05s`, `1h 03m 47s`.
- Optional configuration at `~/.config/agent-pet/config.json` to watch extra directories; with no file the pet watches each agent's default and whatever its running processes report.
- Nothing leaves the machine: the pet reads agent files and never writes to them, and has no network dependency.

# agent-agnostic-pet

A small always-visible surface that shows what every AI coding-agent session on this
machine is doing right now, so you can stay in another window instead of tabbing back
to the terminal.

Release 001 covers Claude Code CLI and Codex CLI sessions on macOS.

## What it shows

One row per live session: which agent, the project, what that session is currently
doing, and how many seconds ago that was observed.

```
Claude agent-agnostic-pet             1s
Reading backlog.md

Codex agent-agnostic-pet              4s
Inspecting SSH configs and hosts

Claude claude-code-experimental       1s
Idle
```

Two sessions in the same project get a short suffix, because the name a session derives
for itself is not unique. The agent's name is drawn from whatever the adapter reports,
so the surface holds no list of agents.

Three situations replace the list rather than showing rows:

| | |
|---|---|
| `state unknown` | that session's working state could not be read — never guessed as idle or working |
| `sessions unreadable` | discovery itself failed; the reason is written to the log |
| `no agents running` | discovery worked and nothing is running |

## Build and run

Needs Rust and the Xcode Command Line Tools. No Xcode, no package manager.

```sh
./build.sh
open build/AgentPet.app
```

The pet has no dock icon and no menu bar. Quit it with `pkill AgentPet`.

## Configuration

Optional. With no config file the pet watches `~/.claude` and `~/.codex`, and also finds
the profile directory of every running `claude` process — so a session started under a
custom `CLAUDE_CONFIG_DIR` shows up on its own, with no setup.

To watch somewhere else as well, create `~/.config/agent-pet/config.json`:

```json
{ "watchDirectories": ["~/work/.claude"] }
```

Directories that do not exist are skipped.

## How it works

The pet reads files each agent already writes, and writes nothing of its own. Each
agent proves it differently, and each proof stays inside that agent's adapter.

**Claude Code** publishes a registry file per session:

- `<profile>/sessions/<pid>.json` — which sessions exist, their project, and whether
  they are busy or idle.
- `<profile>/projects/<slug>/<sessionId>.jsonl` — read forward from where it last
  stopped, for the activity line.

A session counts as live only when its registry file exists, the process it names is
running, **and** that process's start time matches the one recorded in the file. The
third test is what stops a force-killed session or a recycled PID from leaving a row
behind.

**Codex** records no PID anywhere, so liveness comes from the process itself: a running
`codex` process holds an open handle on its own rollout for as long as the session
lives. A rollout at `~/.codex/sessions/YYYY/MM/DD/rollout-*.jsonl` is a session only
while some live process holds it open **and** its first line says `source: "cli"` and
`thread_source: "user"` — which is also what excludes Codex Desktop threads, subagent
threads, and ChatGPT.app's own `codex` process, none of which hold a CLI rollout.
Rollouts reach 74 MB, so an unseen one is read backward from its end over a bounded
window and tailed forward after that. `~/.codex/state_5.sqlite` is never opened.

A session whose most recent turn boundary cannot be found reads `state unknown` — never
guessed as idle or working. A Codex session that has taken no turn yet has written no
rollout, so it has nothing to show and no row until its first turn.

Nothing is hooked, injected, or configured in any agent, no agent-owned file is ever
written, and nothing observed leaves the machine.

Design notes: [docs/design-docs/001-observation-channel-and-pet-surface.md](docs/design-docs/001-observation-channel-and-pet-surface.md)

## Layout

```
core/    Rust observation core — discovery, liveness, activity text
macos/   Swift + AppKit surface, a non-activating NSPanel
```

The core hands the surface a plain list of live sessions across a single JSON call.
Adding an agent means adding an adapter in `core/`; the surface does not change.

## Tests

```sh
./test.sh                                  # both suites
cargo test --manifest-path core/Cargo.toml # the core alone
```

The Codex tests run against reduced copies of real rollouts in `core/fixtures/codex/`,
one per shape the filter has to tell apart — a CLI session, a subagent thread, and a
Desktop thread — with captured command output scrubbed.

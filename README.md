# agent-agnostic-pet

A small always-visible surface that shows what every AI coding-agent session on this
machine is doing right now, so you can stay in another window instead of tabbing back
to the terminal.

Release 001 covers Claude Code CLI and Codex CLI sessions on macOS.

## What it shows

One row per live session: which agent, the project, what that session is currently
doing, and how many seconds ago that was observed.

```
● Claude agent-agnostic-pet           1s     green
  Reading backlog.md

● Codex agent-agnostic-pet            4s     green
  Inspecting SSH configs and hosts

● Claude claude-code-experimental    36s     orange
  Bash command approval

● Claude margin-release             2m 4s    red
  Error: 529

● Claude scratch-experiments          1s     grey
  Idle
```

Two sessions in the same project get a short suffix, because the name a session derives
for itself is not unique. The agent's name is drawn from whatever the adapter reports,
so the surface holds no list of agents.

Each row opens with a mark that carries its state, so a glance answers "does anything
need me?" without reading a word:

| | |
|---|---|
| green | working — the line beneath says what it is doing |
| grey | idle — finished its turn, costing nothing to leave alone |
| orange | waiting on you, in the agent's own words (`input needed`, `dialog open`) |
| red | stopped on an error it did not recover from, with the status code where there is one |
| grey ring | state could not be read — never guessed |

Unknown is the one that is not a filled dot. Idle and unknown are both grey, and an
unreadable state must never be mistaken for a session that finished cleanly.

**A live session breathes; a settled one holds still.** Working breathes slowly and
shallowly — busy, wanting nothing from you. Waiting and errored breathe faster and deeper,
so the states that need you separate from the one that does not by motion as well as by
colour. Idle and unknown do not move at all. It is a Core Animation opacity pulse, running
in the render server, so it costs the app no per-frame work.

Waiting is deliberately not idle: an idle session finished cleanly, a waiting one is
blocked mid-turn and stays that way until you answer.

Attention states are Claude Code only. Codex publishes nothing about them — see below.

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
the profile each running session reports for itself — a Claude session under a custom
`CLAUDE_CONFIG_DIR`, a Codex session under a custom `CODEX_HOME` — so either shows up on
its own, with no setup. Each adapter names its own command, variable and default; the pet
asks every adapter and watches whatever comes back, so it holds no list of agent
directories and learns no agent's name to do it.

To watch somewhere else as well, create `~/.config/agent-pet/config.json`:

```json
{ "watchDirectories": ["~/work/.claude"] }
```

Directories that do not exist are skipped.

## How it works

The pet reads files each agent already writes, and writes nothing of its own. Each
agent proves it differently, and each proof stays inside that agent's adapter.

**Claude Code** publishes a registry file per session:

- `<profile>/sessions/<pid>.json` — which sessions exist, their project, and their status:
  `busy`, `idle`, `waiting` (with a `waitingFor` reason) or `shell`. Shell mode counts as
  working: nothing is wrong and nothing is wanted from you.
- `<profile>/projects/<slug>/<sessionId>.jsonl` — read forward from where it last
  stopped, for the activity line.

An error is read from the transcript, where the agent writes `isApiErrorMessage` with an
`apiErrorStatus`. It counts only while it is still the newest entry *and* the session is
not busy — a busy session is one that hit an error and carried on, and an older error is
one it already recovered from. Failed `tool_result`s are not session errors; the agent
routinely works around them.

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
window and tailed forward after that; where that window holds no turn boundary — a
session mid-turn has written everything since its turn began, up to 22.8 MB here — it
keeps scanning backward for one rather than reporting an unknown state.
`~/.codex/state_5.sqlite` is never opened.

A session whose most recent turn boundary cannot be found reads `state unknown` — never
guessed as idle or working. A Codex session that has taken no turn yet has written no
rollout, so it has nothing to show and no row until its first turn.

**Codex has no attention states, and this was tested rather than assumed.** Its protocol
defines `exec_approval_request`, `apply_patch_approval_request` and friends, but a real
session driven to a genuine approval prompt wrote *nothing* to its rollout for the whole
fifteen seconds it sat blocked — the file holds no approval or error event of any kind. A
waiting Codex session is byte-for-byte indistinguishable from one running a slow command.
Telling them apart would need a timeout on an unanswered tool call, which would fire on
every long build, so Codex sessions stay `working`, `idle` or `state unknown`. A blocked
one reads `working`, which is incomplete but not false: it is genuinely mid-turn.

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

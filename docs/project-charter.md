# Project Charter: agent-agnostic-pet

## Target User
The developer (solo) — a personal tool for daily use alongside AI coding agents. Others may benefit later but are not the design target.

## Problem
Long-running AI coding-agent sessions give no ambient, glanceable signal of what they are actually doing right now — not merely idle/working/done, but a very short status of the current activity, plus whether they are waiting for input or errored — so the developer keeps tabbing back to check on them instead of staying focused elsewhere.

## Current Alternative
Tabbing back to the terminal to check. OS notifications are available and easy to turn on, but they are discrete events, not a continuous signal; what is missing is a small, always-visible status that can be glanced at while other windows or tabs are in front.

## Core Value
A small, always-visible on-screen pet that mirrors the state of whichever coding agent is running — where "state" is a very short status of what the agent is actually working on, alongside whether it is waiting for input or errored — so a single glance, with any window in front, tells the developer what is happening and whether the agent needs them.

## Charter Statement
For a solo developer running long AI coding-agent sessions who needs a glanceable, always-visible sense of what an agent is actually working on and whether it needs them, this product provides a small on-screen pet that mirrors the state of whichever agent is running.

## Goals
- Agent state — a very short status of what each agent is actually working on, plus waiting-for-input and error conditions — is readable at a glance from anywhere on screen, whatever window is focused.
- Works with any coding agent, not one vendor: supporting a new agent requires no change to the pet itself.
- Multiple concurrent agent sessions remain distinguishable, each with its own clear signal.
- Stays out of the way: never steals focus, blocks content, or demands interaction; it can be ignored entirely.

## Non-Goals
- Controlling or messaging the agent: the pet is read-only — no sending prompts, approving actions, or steering an agent from the pet.
- A log or transcript viewer: it shows a very short status, never a scrollable history or detailed output.
- Virtual-pet game mechanics: no feeding, hunger, XP, evolution, or achievements — the pet exists to convey state.

## Success Measures
- After a week of daily use, the developer no longer switches to the terminal just to see what an agent is doing.
- The pet is launched every day and left running, rather than closed as noise.
- Spot-checked against the real agent, the displayed status matches within a few seconds.
- A second coding agent is supported without changing the pet itself — only through its own integration.

## Important Constraints
- Everything stays local: no agent activity, prompts, or code leaves the machine; no cloud service involved.
- Negligible resource footprint: must not noticeably affect CPU, battery, or the agents themselves.
- Agent-agnosticism is proven from day one with two agents: Claude Code CLI and Codex CLI are both first-class from the start.

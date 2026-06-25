# claude-copet

A desktop pixel-pet companion for Claude Code. *Co* + *pet* — your coding sidekick that lives on your desktop, reacts to what Claude Code is doing, grows over time, and talks back.

## What it is

A lightweight always-on-top desktop pet (built with **Tauri**) that:

- **Reacts in real time** to your Claude Code activity via hooks — busy when tools run, panics on errors, celebrates on completion, dozes when idle.
- **Grows over time** — earns XP from your coding activity, levels up, and evolves through pixel-sprite stages. No hunger, no death; neglect just means it naps.
- **Talks** — speech bubbles driven by handwritten template lines, with optional LLM-generated lines for special moments. The default LLM provider reuses your local Claude Code login (`claude -p`, no API key); a direct Anthropic API key is the alternative. Context sent to the LLM is a de-sensitized summary (event type + level/stage + counts, never raw code, commands, or paths). See [docs/llm.md](docs/llm.md).

## Architecture (high level)

```
Claude Code ──(hooks append JSONL)──> ~/.claude-copet/events.jsonl
                                              │ (file watch)
                                              ▼
                    Tauri (Rust): event → mood state machine + decay
                                  XP / level / evolution  ──> state.json
                                  Speaker: Template | LLM (claude-cli · API key)
                                              │ (emit)
                                              ▼
                    Web frontend (Canvas): pixel sprite animation
                                  transparent · always-on-top · draggable
```

- **Event delivery:** file event log (`events.jsonl`) — hooks are fire-and-forget, never block Claude Code; doubles as the data source for long-term growth.
- **Storage:** plain JSON (`state.json`).
- **LLM:** pluggable `Speaker` interface, fully toggleable (off by default). Default provider reuses the local Claude Code CLI (no API key); Anthropic API key is the alternative. See [docs/llm.md](docs/llm.md).

## Wiring it into Claude Code

The pet reacts to an append-only event log written by Claude Code hooks. See
[docs/hooks.md](docs/hooks.md) for the log path, line format, the fire-and-forget
hook script, and the `settings.json` block to paste in.

## Status

Implemented: Tauri idle-pet shell, the hooks→log→mood→sprite perception pipe,
the mood state machine + template speech, XP/level/evolution + persistence, the
optional LLM voice with settings ([docs/llm.md](docs/llm.md)), and direct-
manipulation interactions (drag with position memory, click-to-pet, right-click
menu → settings / today's stats / quit), and a cross-platform release pipeline
(GitHub Actions + `tauri-action`; see [docs/release.md](docs/release.md)). All
seven planned slices are implemented. See the PRD and `tasks/` for the source of truth.

Cross-platform builds (macOS / Windows / Linux) are produced via GitHub Actions (`tauri-action`) on release.

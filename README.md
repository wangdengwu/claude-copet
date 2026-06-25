# claude-copet

A desktop pixel-pet companion for Claude Code. *Co* + *pet* — your coding sidekick that lives on your desktop, reacts to what Claude Code is doing, grows over time, and talks back.

## What it is

A lightweight always-on-top desktop pet (built with **Tauri**) that:

- **Reacts in real time** to your Claude Code activity via hooks — busy when tools run, panics on errors, celebrates on completion, dozes when idle.
- **Grows over time** — earns XP from your coding activity, levels up, and evolves through pixel-sprite stages. No hunger, no death; neglect just means it naps.
- **Talks** — speech bubbles driven by handwritten template lines, with optional LLM-generated lines (Claude Haiku) for special moments. Context sent to the LLM is a de-sensitized summary (event type + brief gist, never raw code).

## Architecture (high level)

```
Claude Code ──(hooks append JSONL)──> ~/.claude-copet/events.jsonl
                                              │ (file watch)
                                              ▼
                    Tauri (Rust): event → mood state machine + decay
                                  XP / level / evolution  ──> state.json
                                  Speaker: Template | LLM (pluggable)
                                              │ (emit)
                                              ▼
                    Web frontend (Canvas): pixel sprite animation
                                  transparent · always-on-top · draggable
```

- **Event delivery:** file event log (`events.jsonl`) — hooks are fire-and-forget, never block Claude Code; doubles as the data source for long-term growth.
- **Storage:** plain JSON (`state.json`).
- **LLM:** pluggable `Speaker` interface; defaults to Claude Haiku, fully toggleable.

## Status

Design agreed; implementation not yet started. See the PRD for the source of truth.

Cross-platform builds (macOS / Windows / Linux) are produced via GitHub Actions (`tauri-action`) on release.

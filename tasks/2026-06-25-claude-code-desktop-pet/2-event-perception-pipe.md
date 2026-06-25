---
id: 2
slug: event-perception-pipe
prd: docs/prds/2026-06-25-claude-code-desktop-pet.md
state: ready-for-agent
category: enhancement
blocked_by: [1]
---

## What to build
The end-to-end "perception" link that lets the pet feel Claude Code working. Claude
Code hooks append events to a log; the Rust core reads new events and tells the
frontend which mood to show; the sprite changes accordingly.

Concretely: a set of hook snippets register on Claude Code lifecycle events
(SessionStart, UserPromptSubmit, PreToolUse, PostToolUse, Stop, error/notification)
and each appends exactly one JSON line to the event log and exits immediately —
fire-and-forget, never blocking Claude Code. The Rust core watches the log, parses
each new line into a typed event, advances a consumption cursor (so each event is
read exactly once, even across restarts), maps the event to a mood with a simple
immediate mapping, and emits that mood to the frontend, which swaps the sprite.

This slice uses a direct event→mood mapping (no decay/fallback yet — that is slice 3).
Demo: running a tool in Claude Code makes the pet show `work`; an error makes it
`panic`; a completed turn makes it `happy`.

## Key interfaces
- Hook emitter — appends one line `{ ts, type, tool?, session, ... }` to the event log and exits; must not block or fail Claude Code if the pet is not running.
- Event log contract — append-only JSONL at the agreed location; consumers never assume the writer is alive.
- `Event` type — parsed shape of one log line (timestamp, type, optional tool, session id).
- Parser — `(logBytes, fromOffset) → (events, newOffset)`; skips malformed lines without aborting; advances offset past consumed bytes.
- `moodForEvent(event) → mood` — direct mapping used in this slice (SessionStart→wake, UserPromptSubmit→listen, PreToolUse→work, error→panic, Stop→happy).
- Frontend signal — core emits the current mood; frontend feeds it to `animationForMood` from slice 1.

## Acceptance criteria
- [ ] A hook snippet appends a valid JSON line per Claude Code event and returns immediately.
- [ ] With the pet not running, hooks still succeed and events accumulate in the log (nothing lost).
- [ ] The parser turns valid JSONL lines into typed events, skips malformed lines, and advances the offset cursor correctly (unit test, seam 3).
- [ ] Re-reading from a saved offset never re-emits already-consumed events.
- [ ] Live: a PreToolUse event switches the on-screen sprite to `work`; an error switches it to `panic`; Stop switches it to `happy`.
- [ ] Hooks/the log path are documented so a user can wire them into their Claude Code settings.

## Out of scope
- Decay timers, mood preemption rules, idle→sleep fallback, `tired` (slice 3).
- Speech bubbles / any text (slice 3).
- XP / growth aggregation (slice 4) — though this slice produces the consumed-event stream that slice 4 builds on.

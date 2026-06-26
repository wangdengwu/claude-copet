---
id: 2
slug: enriched-hook-session-label
prd: docs/prds/2026-06-26-claude-code-session-hud.md
state: ready-for-agent
category: enhancement
blocked_by: [1]
---

## What to build

Make the HUD know **which session it is showing** and follow the most-recently-
active one. End-to-end: enrich the hook to record the session's working directory
and transcript path, parse those fields, track the active session in the watcher,
and render a session label on the card.

1. **Enriched hook.** The hook script appends two new fields to each event line —
   `cwd` and `transcript_path` — pulled from Claude Code's stdin payload using the
   same `sed`-from-stdin, no-`jq` technique already used for `tool_name` /
   `session_id`. It stays fire-and-forget and always exits 0. Final line shape:
   `{ ts, type, tool, session, cwd, transcript_path }`.

2. **Parse + active-session tracking.** `events::parse` tolerantly captures the two
   new optional fields (missing/empty still fine, malformed lines still skipped).
   The watcher tracks a "current session context" = the session/cwd/transcript_path
   of the most-recently-consumed event, updating it whenever a new event carries a
   (possibly different) session.

3. **`hud` emission + card rendering.** The watcher emits a new **`hud`** event
   carrying at least `{ sessionLabel, sessionId }` on startup and whenever they
   change. The card renders the session label (the cwd basename) on its top row;
   hovering shows the full session id. When a different session emits an event, the
   label switches to follow it.

Re-running Connect upgrades an existing install for free: the install command
string is unchanged, but `install_hooks` rewrites the bundled hook script on every
Connect, so the new `cwd`/`transcript_path` extraction lands automatically.

## Key interfaces

- Hook script — appends `cwd` / `transcript_path` extracted from stdin; output
  remains a single valid JSON line; exit 0 always.
- `events::Event` / `events::parse` — gain optional `cwd` and `transcript_path`
  fields; existing skip-malformed / tolerate-trailing-partial behaviour unchanged.
- `session_label(cwd: &str) -> String` — **new pure fn**: returns the last path
  component of `cwd` (handles `/` and `\` separators, trailing slash, root, empty).
- Watcher active-session state — tracks `{ session, cwd, transcript_path }` from the
  latest event; drives the `hud` emission.
- `hud` event payload — `{ sessionLabel: string, sessionId: string }` for this
  slice (later slices add `model`, `contextPercent`, `activity`, `needsHuman`).
- Frontend — listen for `hud`; render the session-label row + full-id tooltip.
- `hooks_install` — install command string unchanged; confirm existing
  merge/remove/detect tests still pass against the rewritten script.

## Acceptance criteria

- [ ] A real Claude Code event writes a line containing non-empty `cwd` and `transcript_path` (verify against an actual session after Connect).
- [ ] `events::parse` returns the new fields when present and tolerates their absence; malformed-line handling unchanged.
- [ ] `session_label` unit tests cover `/`-paths, `\`-paths, trailing slash, root, and empty input.
- [ ] The watcher emits a `hud` event whose `sessionLabel`/`sessionId` reflect the most-recently-active session and update when a new session emits an event.
- [ ] The card displays the session label (cwd basename); hovering shows the full session id.
- [ ] With two Claude Code sessions in different directories, the card label switches to whichever emitted the latest event.
- [ ] Re-running Connect on an old install rewrites the hook script so new fields start appearing without manual edits.
- [ ] `cargo test` and `pnpm test` pass.

## Out of scope

- Context % and model (no transcript reading yet) — slice 3.
- Current activity and the needs-human alert — slice 4.
- Side-by-side display of multiple sessions (single active session only).

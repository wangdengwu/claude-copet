---
id: 1
slug: remove-growth-hud-shell
prd: docs/prds/2026-06-26-claude-code-session-hud.md
state: ready-for-agent
category: enhancement
blocked_by: []
---

## What to build

Retire the pixel-pet/growth product and stand up the empty **status-HUD card**
that the later slices fill in. After this slice the app runs and shows a small
mood pet (still driven by the existing event→mood pipeline) inside the new
"card · pet-left" container — and nothing growth-related exists anywhere.

Two parts, both end-to-end:

1. **Remove growth + the LLM speaker path.**
   - Delete the growth aggregator and everything that surfaced it: XP / level /
     evolution stage, the persisted `state.json` load/save, the `pet_state` and
     `stage` events emitted to the frontend, the frontend "Today's stats"
     overlay, and the "Today's stats" context-menu item.
   - Delete the watcher's LLM special-moment path (special-moment detection,
     de-sensitised summary builder, cooldown, and the provider clients) and the
     settings/UI that configured it (the "AI-written lines" toggle, provider
     selector, API-key field). The speech-bubble surface itself may remain in
     the codebase but is no longer emitted to by default.
   - The event→mood state machine and the corner pet's sprite rendering stay.

2. **Stand up the HUD card shell.**
   - Replace the old free-floating pet + overlays with a single compact card
     (dark translucent, rounded, frameless, always-on-top, draggable with
     remembered position — reuse the existing drag + window-state behaviour).
   - Layout is "card · pet-left": the existing sprite pet sits on the left of
     the card; the right side is an empty info column that slices 2–4 populate.
   - Keep the right-click menu (Settings, Quit) and the Settings panel's
     **Connect / Disconnect** Claude Code section.

This slice introduces no new status fields yet — it clears the deck and gives
the next slices a card to render into.

## Key interfaces

- Growth module — **deleted**. All call sites in the watcher loop (aggregation,
  stage-change emit, state persistence) and the entry point (state load) removed.
- `watch_event_log()` — current contract drives mood **and** growth + LLM; desired
  contract drives **only** the mood state machine and (still) emits `mood`.
  Remove the `pet_state` / `stage` emissions and the LLM branch.
- Tauri commands — remove growth/LLM-only commands; **keep** `pet_clicked`,
  `quit_app`, `install_hooks`, `uninstall_hooks`, `hooks_status`, `get_settings`,
  `set_settings`. `Settings` shrinks to only what Connect needs (drop
  `llm_enabled` / `provider` / `model` / `api_key` unless still referenced).
- Frontend `main.ts` — remove the `pet_state` / `stage` listeners, the stats
  overlay, and the stats menu item; build the card container around the existing
  `startRenderLoop` pet. `mood` and the drag/click/menu wiring stay.
- `animation.ts` — drop the now-unused `Stage` type.

## Acceptance criteria

- [ ] `growth.rs` and all references to it are gone; project compiles without it.
- [ ] No `pet_state` or `stage` events are emitted or listened for.
- [ ] No XP, level, evolution stage, or daily-stats appears in any UI or persisted file path.
- [ ] The LLM special-moment path and its settings UI are removed; no provider/API-key code remains in the watcher.
- [ ] The app shows a single draggable card with the mood pet on the left; the card remembers its position across restarts.
- [ ] The corner pet still changes with the event→mood pipeline (manual `echo` of a `PreToolUse` line still animates it).
- [ ] Right-click menu has Settings + Quit (no "Today's stats"); Settings still has working Connect / Disconnect.
- [ ] `cargo test` and `pnpm test` pass (growth/LLM tests deleted; mood + hooks_install tests stay green).

## Out of scope

- The enriched hook (`cwd` / `transcript_path`) — slice 2.
- Any new status fields (session label, model, context %, activity, alert) — slices 2–4.
- Changing the event→mood state machine behaviour.
- Ripping out the speech-bubble component itself (just stop emitting to it).

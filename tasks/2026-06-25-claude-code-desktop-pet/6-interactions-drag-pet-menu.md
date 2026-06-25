---
id: 6
slug: interactions-drag-pet-menu
prd: docs/prds/2026-06-25-claude-code-desktop-pet.md
state: ready-for-agent
category: enhancement
blocked_by: [1]
---

## What to build
Make the pet tactile. The user can drag it anywhere on screen and its position is
remembered across launches. Clicking the pet "pets" it — it reacts happily (and may
say a line). Right-clicking opens a context menu.

The menu always offers Quit. Its "Settings" and "Today's stats" entries wire to the
real surfaces from slices 5 and 4 respectively; until those exist, those entries are
hidden (or disabled), so this slice is demoable on its own with drag + pet + quit.

Demo: drag the pet around the desktop, release, relaunch — it reopens where you left
it; click it and it reacts happily; right-click to quit.

## Key interfaces
- Window drag — dragging the sprite moves the frameless window; final position is persisted and restored on next launch.
- Click-to-pet — a click on the sprite triggers the `happy` mood/reaction (reusing the slice-3 mood entry + speech path when present).
- Position persistence — saved window position (kept separate from, or alongside, the pet state store).
- Context menu — `Quit` always present; `Settings` opens the slice-5 settings surface when available; `Today's stats` shows the slice-4 daily stats when available; otherwise those items are hidden/disabled.

## Acceptance criteria
- [ ] The pet can be dragged to any screen position.
- [ ] After quit + relaunch, the pet reappears at its last position.
- [ ] Clicking the pet triggers a happy reaction.
- [ ] Right-click shows a menu; Quit exits the app cleanly.
- [ ] Settings/stats menu entries open their real surfaces when those slices are present, and are hidden/disabled when not — no broken menu items.

## Out of scope
- Building the settings surface (slice 5) or the stats aggregation (slice 4) — this slice only wires menu entries to them.
- Keyboard shortcuts / summon-hide hotkeys (not in PRD).
- Multi-monitor edge-case polish beyond restoring a saved position.

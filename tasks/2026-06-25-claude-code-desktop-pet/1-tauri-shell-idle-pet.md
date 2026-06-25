---
id: 1
slug: tauri-shell-idle-pet
prd: docs/prds/2026-06-25-claude-code-desktop-pet.md
state: done
category: enhancement
blocked_by: []
---

## What to build
The foundation: a Tauri app that puts a living pixel-pet on the desktop. When the
app launches, a transparent, frameless, always-on-top window appears (not in the
taskbar) showing a single idle pixel-sprite animation looping smoothly. This is the
tracer-bullet shell every later slice plugs into — but it must already feel like a
pet sitting on your desktop, not a blank window.

The frontend runs a Canvas render loop that plays a sprite sheet frame-by-frame for
the current mood (only `idle` exists in this slice). The mood→animation lookup is a
pure function so it can be unit-tested in isolation from rendering.

## Key interfaces
- Tauri window config — `decorations:false, transparent:true, always_on_top:true, skip_taskbar:true`. The window background must be genuinely transparent (only the sprite is visible).
- `animationForMood(mood) → animationKey` — pure mapping function; in this slice only `idle` is wired, but the function is the seam later slices extend.
- Render loop — `requestAnimationFrame`-driven; advances sprite frames at a fixed cadence independent of monitor refresh rate.
- Sprite sheet asset contract — a documented convention for how a mood's frames are laid out (frame size, count) so later slices can drop in `work`/`happy`/etc. sheets.

## Acceptance criteria
- [ ] Launching the app shows a transparent, frameless, always-on-top window outside the taskbar.
- [ ] An idle pixel-sprite animation loops smoothly and continuously.
- [ ] Only the sprite is visible — the window background is fully transparent (no chrome, no rectangle).
- [ ] `animationForMood` is a pure function with a unit test asserting `idle → idle` animation key.
- [ ] `tauri dev` runs the app locally on macOS.

## Out of scope
- Any Claude Code event handling, hooks, or the event log (slice 2).
- Mood transitions / decay / non-idle animations (slices 2–3).
- Dragging, clicking, menus (slice 6).
- Persistence of any state (slice 4).

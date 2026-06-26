---
id: 1
slug: hud-popup-windowing
prd: none (ticket — fast lane, no PRD)
state: ready-for-agent
category: bug
blocked_by: []
---

## What to build

Fix the HUD's popups being clipped by the tiny card window, by moving them out
of the WebView: a **native OS context menu** for right-click, and a **separate
settings window** for the settings panel.

> Larger than a typical ticket (touches Tauri window/menu management in Rust +
> a new frontend page). Scope is bounded and the design is decided, so it runs
> as one slice — but expect Rust + frontend changes.

## Current behavior (the bug)

The app window is **320×96** (`tauri.conf.json`), sized to the pet+card. Both
the right-click menu and the settings panel are HTML elements positioned
`position:fixed` inside that WebView. Anything taller/wider than 96×320 — which
both popups are — renders **outside the window bounds and is clipped by the OS**:
the context menu and the settings page are cut off / hidden behind the card.

Today:
- Right-click menu = an HTML `#ctx-menu` div in `src/main.ts` with items
  Refresh usage / Settings / Quit.
- Settings = an HTML `#settings-panel` in `src/settings.ts` (Claude Code
  Connect/Disconnect + the usage-refresh 5/10/15 dropdown), opened via
  `openSettings()` from the context menu.

## Desired behavior

- **Right-click on the card → a native OS context menu** (not an HTML overlay),
  so it's never clipped and renders above the window. Same items and actions:
  - *Refresh usage* → invoke the existing `refresh_usage` command.
  - *Settings* → open the settings window (below).
  - *Quit* → quit the app (existing `quit_app` / window close).
- **Settings opens in a separate, properly-sized window** (e.g. ~320×280, not
  always-on-top, normal chrome or frameless-with-close) that fully shows the
  existing settings UI and is independently movable. It hosts the same controls:
  Connect / Disconnect (hooks) and the usage-refresh interval dropdown, reusing
  the existing commands `get_settings` / `set_settings` / `install_hooks` /
  `uninstall_hooks` / `hooks_status`.
- The in-card HTML context menu and HTML settings panel are removed (their logic
  moves to the native menu and the settings window respectively).
- Running under plain `vite` (no Tauri runtime) must still not crash — guard all
  Tauri/menu/window calls (the existing `invokeOrNull` pattern).

## Key interfaces (by contract, not path)

- Native menu: Tauri v2 menu API (`tauri::menu::{Menu, MenuItem}` + the window
  `on_menu_event` / context-menu/popup mechanism), built in Rust during setup.
  Items map to: `refresh_usage`, an "open settings window" command, `quit_app`.
- Settings window: created via Tauri (a second `WebviewWindow` /
  `WebviewWindowBuilder`, or a second window declared in `tauri.conf.json`) that
  loads a settings HTML/entry. A command like `open_settings_window()` shows/
  focuses it. The page reuses the existing settings commands listed above.
- The HUD card window keeps its 320×96 size and transparency — only the popups
  leave the WebView.

## Acceptance criteria

- [ ] Right-clicking the card shows a native menu fully visible regardless of the
      card's screen position (including near a screen edge) — nothing clipped.
- [ ] Each native menu item performs its action: Refresh usage triggers a
      `/usage` re-fetch (subject to the existing throttle), Settings opens the
      settings window, Quit exits.
- [ ] The settings window shows ALL controls fully (Connect/Disconnect + the
      5/10/15 dropdown), is movable, and persists changes via `set_settings`
      (the interval still takes effect in the watcher without an app restart).
- [ ] Opening Settings twice focuses the existing window rather than spawning a
      second one.
- [ ] The old HTML `#ctx-menu` and `#settings-panel` are gone; no clipped overlay
      remains.
- [ ] `pnpm build` + `cargo build` clean; plain `vite` (no Tauri) does not throw.

## Out of scope

- Redesigning what's IN settings (same controls, just rehoused).
- The usage-limits feature itself (done).
- Changing the card's size or the pet.

## Notes

- Verify capabilities/permissions: Tauri v2 may require enabling menu / window
  creation permissions in `src-tauri/capabilities/*.json`.
- Visual/interaction ACs need an eyeball via `pnpm tauri dev` (native menu +
  second window can't be asserted in unit tests).

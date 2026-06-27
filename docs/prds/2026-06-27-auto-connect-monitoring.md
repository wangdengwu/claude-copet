# Auto-connect monitoring + menu toggle (PRD)

## Problem
claude-copet is a monitoring companion: with no Claude Code hooks installed it
shows nothing useful. Today connection is a manual, easily-missed step buried in
the Settings page (Connect / Disconnect buttons). A fresh user who just launches
the app sees a dead pet and has no signal that they must open Settings and click
Connect. The default state of a monitoring tool should be *connected*.

## Solution
Make connection the default. On launch the app installs its hooks automatically
unless the user has deliberately opted out. Connecting and disconnecting moves to
a single right-click menu item that toggles between **Disconnect** (when
connected) and **Connect** (when opted out). The Settings page keeps a read-only
connection status indicator but no longer carries the buttons.

A user-initiated Disconnect is *sticky*: it survives restarts, so the app never
silently re-connects against the user's wishes. Reconnecting (via the menu)
clears that opt-out.

## User Stories
1. As a new user, I want the pet to start monitoring as soon as I launch it, so that I don't have to discover and perform a manual connect step.
2. As a user, I want to disconnect monitoring from the right-click menu, so that I can stop the hooks without digging into Settings.
3. As a user who disconnected, I want that choice to persist across restarts, so that the app doesn't quietly re-install hooks every time it launches.
4. As a user who disconnected, I want to reconnect from the same right-click menu, so that turning monitoring back on is one click and self-contained.
5. As a user, I want the menu item's label to reflect the current state (Disconnect vs Connect), so that I always know what clicking it will do.
6. As a user, I want a connection status indicator in Settings, so that I can confirm at a glance whether monitoring is active.
7. As a user with pre-existing unrelated Claude Code hooks, I want auto-install to leave them untouched (and back up settings before changes), so that automatic behavior is safe.

## Implementation decisions
- **Persisted opt-out flag.** Add a boolean `hooks_opt_out` (default `false`) to
  the persisted copet `Settings`. It is the single source of truth distinguishing
  "never connected yet" (auto-connect) from "user deliberately disconnected"
  (stay disconnected). Default-false means a fresh install auto-connects.
- **Auto-install decision is a pure function.** Introduce
  `should_auto_install(opt_out, installed) -> bool`, living next to the existing
  hook-merge logic, returning `true` only when `!opt_out && !installed`. Startup
  calls it and, when true, runs the existing idempotent install path (write hook
  script + merge the six hook entries into `~/.claude/settings.json` with a `.bak`
  backup). Install remains idempotent — a no-op when hooks already present.
- **Menu toggle.** The native right-click menu gains one item between "Settings"
  and the final separator. Its label is dynamic: **Disconnect** when connected,
  **Connect** when opted out; the initial label is derived from current state at
  startup. Clicking it:
    - When connected → uninstall the copet hooks, set `hooks_opt_out = true`,
      relabel to "Connect".
    - When opted out → set `hooks_opt_out = false`, run install, relabel to
      "Disconnect".
  The menu-item handle is held in managed app state so the click handler can
  update its text.
- **Settings page becomes status-only.** Remove the Connect / Disconnect buttons
  and their handlers. Keep the existing `hooks_status`-driven status line
  (Connected / Not connected) as a read-only indicator. The menu is now the only
  place to connect or disconnect.
- **Existing seams reused.** `merge_copet_hooks`, `remove_copet_hooks`,
  `copet_hooks_installed`, and the `install_hooks` / `uninstall_hooks` /
  `hooks_status` commands are unchanged in behavior; this work adds the opt-out
  flag, the decision function, the startup call, and the menu wiring around them.

## Testing decisions
- **`Settings` field — `tests/settings.rs` (pure serde seam).** The existing
  round-trip and missing-file-default tests already pin the load/save seam; adding
  `hooks_opt_out` extends `Settings::default()` coverage there. Add/confirm: a
  default-value assertion (`hooks_opt_out == false`) and that round-trip preserves
  a `true` value. Prior art: `round_trip_preserves_settings`,
  `load_from_missing_path_returns_default`.
- **Auto-install decision — `tests/hooks_install.rs` (pure-logic seam).**
  `should_auto_install` is a pure boolean function tested directly with its four
  input combinations (opt_out × installed). This is the highest, cheapest seam for
  the new behavior and needs no Tauri runtime. Prior art: the pure
  merge/remove/installed tests already in this file
  (`merge_is_idempotent`, `installed_true_after_merge`).
- The menu wiring and startup glue are thin orchestration over already-tested
  pure functions and are validated manually (launch → pet connects; menu toggles
  label and connection; disconnect survives relaunch).

## Out of scope
- Any "restart Claude Code to apply" hint or notification — hooks take effect only
  after Claude Code restarts, but this is left silent (already documented).
- Re-architecting hook install/uninstall logic or the event-log pipeline.
- Removing the Settings status line — only the buttons are removed.
- Per-event or partial hook selection; the six-event set is installed as a whole.

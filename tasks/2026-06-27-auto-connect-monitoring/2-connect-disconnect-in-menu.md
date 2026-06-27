---
id: 2
slug: connect-disconnect-in-menu
prd: docs/prds/2026-06-27-auto-connect-monitoring.md
state: done
category: enhancement
blocked_by: [1]
---

## What to build
Move connect/disconnect out of the Settings page and into the native right-click
menu as a single, state-aware toggle item.

The menu gains one item (between "Settings" and the final separator) whose label
reflects the current connection state: **Disconnect** when hooks are installed,
**Connect** when the user is opted out. Clicking it:
- When connected → remove the copet hooks and set `hooks_opt_out = true`, then
  relabel the item to "Connect".
- When opted out → set `hooks_opt_out = false` and install the hooks, then
  relabel the item to "Disconnect".

The initial label is derived at startup from the current hook status. A
user-initiated Disconnect is therefore sticky: because the opt-out flag is
persisted (slice 1), startup will not re-install on the next launch.

In the same slice, remove the Connect / Disconnect buttons (and their handlers)
from the Settings page, keeping only the read-only `hooks_status`-driven status
line. The menu becomes the sole place to connect or disconnect.

## Key interfaces
- Native menu (`MenuBuilder` in `setup()`) — gains one `MenuItem` with a stable
  id; its handle is held in managed app state (alongside the existing
  `NativeCtxMenu`) so the click handler can call `.set_text(...)`.
- `on_menu_event` handler — handles the new item id: branches on current state,
  calls the existing `install_hooks()` / `uninstall_hooks()` commands, persists
  `hooks_opt_out` via the existing settings save path, and updates the item text.
- Existing `install_hooks` / `uninstall_hooks` / `hooks_status` — reused as-is.
  Disconnect must also set `hooks_opt_out=true`; Connect must clear it — keep menu
  and persisted state in sync.
- Settings page (`settings-page.ts`) — remove `#s-connect` / `#s-disconnect`
  buttons and their click handlers; keep the `hooks_status` status line.

## Acceptance criteria
- [ ] Right-click menu shows a toggle item between "Settings" and "Quit".
- [ ] Label reads "Disconnect" when connected, "Connect" when opted out; correct
      on first open based on actual hook status.
- [ ] Clicking "Disconnect" removes copet hooks, sets `hooks_opt_out=true`, and
      relabels to "Connect" (unrelated hooks untouched).
- [ ] Clicking "Connect" clears `hooks_opt_out`, reinstalls hooks, and relabels
      to "Disconnect".
- [ ] After Disconnect, relaunching the app keeps hooks uninstalled (sticky;
      relies on slice 1's startup respecting the flag).
- [ ] Settings page no longer shows Connect/Disconnect buttons; the
      Connected / Not-connected status line remains and reflects reality.

## Out of scope
- Auto-install on startup and the `hooks_opt_out` field/decision function (slice 1).
- Removing or restyling the Settings status line.
- Any restart hint/notification after connecting.

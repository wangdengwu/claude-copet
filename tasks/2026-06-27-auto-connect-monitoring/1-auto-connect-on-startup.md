---
id: 1
slug: auto-connect-on-startup
prd: docs/prds/2026-06-27-auto-connect-monitoring.md
state: done
category: enhancement
blocked_by: []
---

## What to build
On launch, the app installs its Claude Code hooks automatically when they are
missing — unless the user has deliberately opted out. This makes "connected" the
default state for the monitoring pet, with no manual step required for a fresh
install.

The decision is driven by a persisted opt-out flag and a pure decision function:
auto-install only when the user has NOT opted out AND the hooks are not already
present. Install reuses the existing idempotent path (write the hook script,
merge the six hook entries into `~/.claude/settings.json`, with a `.bak` backup),
so it is a no-op when hooks are already there and never disturbs unrelated hooks.

## Key interfaces
- `Settings` (persisted in `~/.claude-copet/settings.json`) — gains a boolean
  `hooks_opt_out`, defaulting to `false`. Must serde-default to `false` when the
  key is absent from an existing file (older installs), so the field is added
  with `#[serde(default)]` semantics like the existing `usage_refresh_minutes`.
- `should_auto_install(opt_out: bool, installed: bool) -> bool` — new pure
  function living alongside the hook-merge logic; returns `true` iff
  `!opt_out && !installed`. No filesystem access.
- Existing `install_hooks()` / `copet_hooks_installed()` — unchanged; startup
  reads current settings + hook status, calls `should_auto_install`, and runs the
  existing install path when true. Install must remain idempotent and back up
  `~/.claude/settings.json` before changes.

## Acceptance criteria
- [ ] `Settings` has `hooks_opt_out: bool` defaulting to `false`; loading a
      settings file that omits the key yields `false` (no error).
- [ ] Round-trip preserves `hooks_opt_out: true`.
- [ ] `should_auto_install` returns `true` only for `(opt_out=false,
      installed=false)` and `false` for the other three combinations.
- [ ] On launch with no copet hooks present and `hooks_opt_out=false`, the hooks
      are installed (verified manually: hooks appear in `~/.claude/settings.json`).
- [ ] On launch with `hooks_opt_out=true`, hooks are NOT installed.
- [ ] On launch with hooks already present, nothing changes (idempotent; no
      duplicate entries, unrelated hooks intact).

## Out of scope
- The right-click menu toggle and removing the Settings buttons (slice 2).
- Any "restart Claude Code to apply" hint/notification.
- Changes to the hook-merge/remove logic itself beyond adding the pure decision
  function.

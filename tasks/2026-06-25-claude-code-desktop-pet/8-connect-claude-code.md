---
id: 8
slug: connect-claude-code
prd: docs/prds/2026-06-25-claude-code-desktop-pet.md
state: done
category: enhancement
blocked_by: [1]
---

## What to build
A one-click **Connect to Claude Code** in the settings panel that installs/removes
the perception hooks automatically — no hand-editing `~/.claude/settings.json`.
Manual editing is error-prone and not product-grade; this makes onboarding a button.

## Design
- **install_hooks**: write the bundled hook script to a stable path
  `~/.claude-copet/claude-copet-hook.sh` (chmod 755 on unix), then merge our six
  hook entries (SessionStart, UserPromptSubmit, PreToolUse[*], PostToolUse[*],
  Stop, Notification) into `~/.claude/settings.json` `hooks` — preserving every
  existing entry, idempotent, with a `.bak` backup written first.
- **uninstall_hooks**: remove only entries whose command references
  `claude-copet-hook.sh`; leave all other hooks intact.
- **hooks_status**: report whether our hooks are installed.
- UI: a "Claude Code" block in the settings panel — status badge (connected / not)
  + Connect / Disconnect buttons; after either, refresh status and tell the user to
  restart Claude Code to apply.

## Key interfaces (pure, unit-tested — no real ~/.claude access in tests)
- `merge_copet_hooks(settings: Value, script_path: &str) -> Value` — idempotent;
  preserves unrelated hooks; adds ours only if absent.
- `remove_copet_hooks(settings: Value) -> Value` — removes only ours.
- `copet_hooks_installed(settings: &Value) -> bool`.

## Acceptance criteria
- [ ] One click installs all six hooks into `~/.claude/settings.json`, preserving any pre-existing hooks (tma1/RTK/etc).
- [ ] Re-running install is idempotent (no duplicate entries).
- [ ] Disconnect removes exactly our entries and nothing else.
- [ ] A backup of the prior settings.json is written before modifying it.
- [ ] Status reflects installed/not-installed; UI tells the user to restart Claude Code.
- [ ] The merge/remove/detect helpers are unit-tested against synthetic JSON (no real home dir touched).

## Out of scope
- Windows `sh` compatibility for the hook command (script is POSIX sh; needs git-bash) — note it, handle later.
- Auto-restarting Claude Code; live "is the log being written" health-check (status = config presence only).

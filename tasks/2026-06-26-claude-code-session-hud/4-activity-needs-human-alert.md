---
id: 4
slug: activity-needs-human-alert
prd: docs/prds/2026-06-26-claude-code-session-hud.md
state: ready-for-agent
category: enhancement
blocked_by: [2]
---

## What to build

Fill the card's bottom row with **what Claude is doing now** and an unmissable
**needs-human alert**.

End-to-end: the watcher derives a current-activity string and a needs-human flag
from the event stream (no transcript needed) and includes them in the `hud` event.
The card's bottom row shows the activity (e.g. "Running Bash", or "Idle"); when
Claude is waiting on permission/input, that row becomes a prominent warning and the
whole card turns amber and pulses. The flag clears automatically when Claude
resumes work.

- **Current activity** — from the already-parsed events + mood: a `PreToolUse`
  with `tool = "Bash"` → "Running Bash"; an idle/quiet mood → "Idle".
- **Needs-human** — a pure flag transition: set on `Notification` and `Stop`
  (Claude is waiting / your turn), cleared on `UserPromptSubmit` and `PreToolUse`
  (work resumed).

## Key interfaces

- `attention_step(flag: bool, event: &Event) -> bool` — **new pure fn**: returns
  `true` on `Notification` / `Stop`, `false` on `UserPromptSubmit` / `PreToolUse`,
  and the unchanged `flag` for other event types.
- Activity derivation — pure mapping from the latest relevant event (`PreToolUse.tool`)
  and/or mood to a short activity string ("Running <tool>", "Idle").
- Watcher — maintain the attention flag across events; add `activity: string` and
  `needsHuman: boolean` to the `hud` payload.
- `hud` payload — gains `activity` and `needsHuman`.
- Frontend `formatHud` — extends the slice-3 mapping: chooses the bottom-row text
  (activity vs. the warning line) and toggles the card-level amber-pulse styling
  when `needsHuman` is true.

## Acceptance criteria

- [ ] `attention_step` tests drive a sequence and assert set/clear: Notification→set, PreToolUse→clear, Stop→set, UserPromptSubmit→clear, unrelated event→unchanged.
- [ ] Activity mapping tests: `PreToolUse` tool name → "Running <tool>"; idle/quiet → "Idle".
- [ ] The card's bottom row shows the current activity and updates as tools run.
- [ ] On a `Notification` (or `Stop`), the card turns amber, pulses, and the bottom row shows the needs-human warning.
- [ ] The alert clears (card returns to normal) on the next `UserPromptSubmit` / `PreToolUse`.
- [ ] `formatHud` vitest covers the activity-vs-warning bottom-row choice and the amber-pulse toggle.
- [ ] `cargo test` and `pnpm test` pass.

## Out of scope

- Context % and model — slice 3.
- Per-tool detail beyond the tool name (no command text, no file paths).
- Sound/notification-center alerts (visual card alert only).

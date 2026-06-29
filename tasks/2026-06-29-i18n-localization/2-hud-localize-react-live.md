---
id: 2
slug: hud-localize-react-live
prd: docs/prds/2026-06-29-i18n-localization.md
state: ready-for-agent
category: enhancement
blocked_by: [1]
---

## What to build
The HUD card speaks the chosen language and updates the instant the user switches,
with no restart.

Introduce a TypeScript i18n module — a `{ en, zh }` message map plus a lookup
function `t(locale, key)` — that is the single place TS chrome strings live. This
module is created here and reused by the settings window (slice 3).

Make the pure HUD view-model builder locale-aware: it currently bakes in a Chinese
needs-human warning and an English `"Idle"` fallback. Both become message-table
lookups chosen by a `locale` parameter. The English needs-human string is
`"⚠ Waiting for input / approval"`; the Chinese one is the existing
`"⚠ 等你输入 / 授权"`. Data-derived fields (label, model, percent, color band,
countdowns, usage lines) are unchanged and identical across locales.

Wire the pet window: read the current locale on load via `get_settings`, pass it
into the builder when rendering, and listen for the `"locale"` event (emitted by
slice 1). On that event, re-render the card from the retained latest snapshot using
the new locale — the needs-human warning and the Idle label flip immediately.

## Key interfaces
- TS `i18n` module (new) — `t(locale, key)` returns the localized string; `en` and
  `zh` maps must contain the same keys (no missing-translation gaps). Keys needed
  now: needs-human warning, idle activity.
- `formatHud()` — current contract: `formatHud(state)` returns a view with
  `activityText` already resolved (Chinese warning / "Idle"). Desired: takes the
  active locale (added param or option) and resolves `activityText`'s chrome
  branches via the i18n table; the warning/Idle text differs by locale, everything
  else identical.
- Pet window render path — currently calls `formatHud(payload)` in the `"hud"`
  listener and retains the snapshot in the existing last-state variable. Desired:
  thread the current locale through render, refresh it from the `"locale"` event,
  and re-render the retained snapshot on locale change.
- `"locale"` event — consumed here (produced by slice 1).

## Acceptance criteria
- [ ] The TS `i18n` module's `en` and `zh` maps expose the same key set; `t()`
      returns the correct string per locale (unit test).
- [ ] `formatHud` under `en` vs `zh`: the needs-human text and the Idle fallback
      differ as specified; percent, color band, countdown, model, and label are
      byte-identical across locales (extends the existing HUD builder tests).
- [ ] On load the HUD renders in the persisted locale (English by default).
- [ ] Switching language from the menu flips the needs-human warning and the Idle
      label live, with no restart and no new snapshot required.
- [ ] When running outside Tauri (no `get_settings`/event), the HUD still renders
      in English rather than crashing (existing graceful-degrade behavior kept).

## Out of scope
- Settings-window localization (slice 3) — though it reuses the i18n module created
  here.
- The locale setting, menu, and `"locale"` event emission (slice 1).
- Translating data strings or the `"—"` unavailable placeholder.

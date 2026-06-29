---
id: 3
slug: settings-window-localize-react-live
prd: docs/prds/2026-06-29-i18n-localization.md
state: ready-for-agent
category: enhancement
blocked_by: [1, 2]
---

## What to build
The settings window speaks the chosen language and updates live when the user
switches, completing the "all chrome localized" goal.

Replace the hardcoded English labels in the standalone settings window with lookups
into the shared TS i18n module (created in slice 2). The localized strings are the
static chrome: the panel title, the section titles (Claude Code, Usage Refresh),
the connected / not-connected status text, the Interval label, and the interval
option labels (the "min" wording — the numeric values stay as data).

Read the current locale on load via `get_settings` and render the labels in that
locale. Listen for the `"locale"` event (from slice 1) and re-render the static
labels in place when it fires, so a language change made from the menu is reflected
in an open settings window immediately. Live setting values (hook status, selected
interval) must be preserved across a locale re-render — switching language must not
reset the selected interval or lose the connection status.

## Key interfaces
- TS `i18n` module — reused from slice 2; add any settings-only keys (panel title,
  section titles, connected/not-connected, interval label, "min" unit) to both the
  `en` and `zh` maps.
- Settings page render — currently builds its DOM from an inline English template
  and refreshes hook status / interval via `get_settings`. Desired: label text
  comes from `t(locale, key)`; a locale read on load picks the initial language; a
  `"locale"` listener re-applies labels without discarding current control state.
- `"locale"` event — consumed here (produced by slice 1).

## Acceptance criteria
- [ ] All static settings-window labels render via the i18n module; `en` and `zh`
      maps contain every key used (covered by the slice-2 same-key-set test once the
      new keys are added).
- [ ] On open, the settings window renders in the persisted locale (English by
      default).
- [ ] Switching language from the menu while the settings window is open flips its
      labels live, with no reopen and no restart.
- [ ] A locale re-render preserves the displayed hook status and the selected
      Interval value (no reset to defaults).
- [ ] Running outside Tauri still renders the window in English without crashing.

## Out of scope
- HUD localization and the shared i18n module's creation (slice 2).
- The locale setting, menu, and `"locale"` event emission (slice 1).
- Translating the numeric interval values or any data strings.

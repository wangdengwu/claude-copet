---
id: 1
slug: locale-setting-localized-menu
prd: docs/prds/2026-06-29-i18n-localization.md
state: ready-for-agent
category: enhancement
blocked_by: []
---

## What to build
The foundation and the menu surface of localization, end to end. The user's
language choice becomes a persisted setting, the native right-click menu is fully
localized, and switching language from the menu takes effect live.

Add a locale to persisted settings: an enum with two variants (English, Chinese),
serialized as `"en"` / `"zh"`, defaulting to English. Loading must tolerate a
settings file that has no locale key (older installs) or an unrecognized value —
both fall back to English, consistent with the struct's existing
ignore-unknown-keys behavior.

Localize every native context-menu label (Refresh usage, Settings, the new
Language entry, the connect/disconnect toggle, Quit) through a Rust string table
keyed by `(locale, key)`. Add a **Language ▸** submenu containing two check-style
items — `English` and `中文` — with a check on the currently-active locale.

When the user picks a language: persist the new locale to settings, rebuild the
entire native menu (every label depends on locale) and replace the stored menu
handle so the next right-click shows translated labels with the checkmark moved,
and emit a `"locale"` event carrying the new locale to all webviews (no webview
listens yet — that is slices 2 and 3 — but the event must fire now).

The existing connect/disconnect toggle behavior (sticky opt-out, label reflects
live install state) must keep working, now with a locale-aware label.

## Key interfaces
- `Locale` (new Rust enum) — `{ En, Zh }`, serde as `"en"`/`"zh"`; unknown/missing
  deserializes to `En`.
- `Settings` — gains a `locale: Locale` field, default `En`; round-trips through
  the existing `get_settings` / `set_settings` commands and `load_from`/`save_to`.
- Rust `i18n` module — `menu_label(locale, key) -> &'static str` (or equivalent)
  covering: refresh, settings, language, english-name, chinese-name, quit. Every
  key resolves to a non-empty string in both locales.
- `connection_menu_label()` — current contract takes `installed: bool` and returns
  a static EN string; desired contract takes `(locale, installed)` and returns the
  localized Connect/Disconnect label.
- Native menu build (in app setup) — currently builds a flat menu once and stores
  it in `NativeCtxMenu`; desired: build with localized labels + a Language submenu,
  and be rebuildable so a language switch can replace the stored menu.
- `"locale"` Tauri event — new; payload is the active locale (`"en"`/`"zh"`),
  emitted to all windows on switch.

## Known data variants
- `settings.json` with **no `locale` key** (every install before this change) →
  load as `En`.
- `settings.json` with an **unrecognized locale value** (e.g. `"fr"`, `""`, wrong
  type) → load as `En`, do not error.
- Existing unrelated keys (`usage_refresh_minutes`, `hooks_opt_out`) must survive a
  locale write untouched.

## Acceptance criteria
- [ ] `Settings` serde round-trips `locale`; a file missing `locale` loads as `En`;
      an unrecognized locale value loads as `En` (unit tests alongside the existing
      settings round-trip tests).
- [ ] The Rust `i18n` table resolves every menu key to a non-empty string in both
      `En` and `Zh` (unit test).
- [ ] `connection_menu_label(locale, installed)` returns the correct localized
      label for all four (locale × installed) combinations (unit test).
- [ ] Right-clicking shows a **Language ▸** submenu with `English` and `中文`, a
      checkmark on the active locale.
- [ ] Selecting a language persists it, rebuilds the menu so labels are translated
      and the checkmark moves, and the choice survives an app restart.
- [ ] Switching language emits a `"locale"` event with the new locale.
- [ ] The connect/disconnect toggle still installs/uninstalls hooks, keeps the
      sticky opt-out, and now shows a locale-aware label.

## Out of scope
- Translating the HUD card or settings window (slices 2 and 3).
- Any locale beyond English/Chinese; OS-locale auto-detection.
- Translating data strings (model, %, countdowns, activity text).

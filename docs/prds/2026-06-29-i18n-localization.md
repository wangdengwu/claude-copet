# EN/ZH Localization (PRD)

## Problem
The app's user-facing chrome is hardcoded and inconsistent: the right-click menu
and settings window are English, while the HUD card already hardcodes a Chinese
needs-human warning (`"⚠ 等你输入 / 授权"`). A non-English (or English-only) user
has no way to pick a language, and the mixed text looks unfinished. There is no
single place that decides what language the UI speaks.

## Solution
A user can switch the entire UI chrome between **English** and **中文** from the
right-click menu, via a **Language ▸** submenu that check-marks the active
language. The default is English. The switch applies **live, everywhere at once**
— the native menu rebuilds with translated labels, and the HUD card and settings
window re-render immediately, with no restart. The choice persists across
restarts. Data strings (model name, usage %, countdowns, the activity text that
comes from Claude) are left untranslated.

## User Stories
1. As a user, I want the UI to default to English on a fresh install, so that the
   experience is predictable without configuration.
2. As a Chinese-speaking user, I want to switch the UI to 中文 from the right-click
   menu, so that I can read the menu, settings, and HUD in my language.
3. As a user, I want a **Language ▸** submenu listing English and 中文 with a
   checkmark on the active one, so that I can see and change the current language
   at a glance.
4. As a user, when I pick a language I want the right-click menu labels to change
   immediately (Refresh usage, Settings, Language, Quit, Connect/Disconnect), so
   that the menu I'm using reflects my choice right away.
5. As a user, when I switch language I want the HUD card to re-render instantly
   (the needs-human warning and the "Idle" label), so that I don't have to restart
   or reopen anything.
6. As a user, when I switch language I want the settings window labels to update
   instantly if it's open, so that all surfaces stay consistent.
7. As a user, I want my language choice to survive a restart, so that I set it once.
8. As a user, I want data like the model name, usage percentages, and countdowns
   to stay as-is regardless of language, so that the meaning of those values is
   never altered by localization.
9. As a user with a settings file from an older build (no locale, or an
   unrecognized value), I want the app to fall back to English rather than break,
   so that upgrades are safe.

## Implementation decisions

**Locale as single source of truth (Rust `Settings`).**
Add a `locale` field to the `Settings` struct, modeled as an enum `Locale { En, Zh }`
serialized as `"en"` / `"zh"`, defaulting to `En`. Loading tolerates a missing or
unrecognized value by falling back to `En`, consistent with the struct's existing
tolerant-load behavior (unknown keys ignored). Locale flows through the existing
`get_settings` / `set_settings` commands and persists to `settings.json`. Rust is
the authority; the native menu and both webviews read locale from it.

**Two string tables, one per runtime (no shared-file machinery — YAGNI).**
- Rust `i18n` module: a `(Locale, key) → &'static str` lookup for the native menu
  labels (Refresh usage, Settings, Language, English, 中文, Quit, and the
  connect/disconnect toggle). The existing `connection_menu_label` is extended to
  take a `Locale` argument.
- TS `i18n` module: a `{ en, zh }` message map plus a `t(locale, key)` lookup,
  consumed by the HUD view-model builder and the settings page. The currently
  hardcoded Chinese needs-human string becomes the `zh` entry; the `en` entry is a
  new English string (`"⚠ Waiting for input / approval"`). The `"Idle"` fallback
  is localized the same way.

**Native menu — live rebuild.**
A `Language ▸` submenu holds two check-style items (English / 中文), the active one
checked. On selection: persist the new locale, rebuild the entire native context
menu (every label is locale-dependent) and replace the stored menu handle, then
emit a `"locale"` event to all webviews.

**Live propagation to webviews.**
The HUD view-model builder gains a `locale` parameter and stays a pure function.
The pet window keeps the last `hud` snapshot and, on a `"locale"` event, re-runs
the builder with the new locale and re-renders. The settings page re-renders its
static labels on the same event. On load, each webview reads the current locale
via `get_settings`.

**Scope of translation.** Only UI chrome. Data strings (model, %, countdowns, the
Claude-supplied activity text, the `"—"` unavailable placeholder) are passed
through unchanged.

## Testing decisions

Seams, highest-level first, kept minimal:

- **Rust `Settings` serde round-trip** (prior art: `src-tauri/tests/settings.rs`).
  Verify `locale` round-trips; a settings file with no `locale` loads as `En`; an
  unrecognized locale value loads as `En`.
- **Rust `i18n` table + `connection_menu_label`** (unit tests in the new module /
  alongside `hooks_install`). Every menu key resolves to a non-empty string in
  both locales; `connection_menu_label` returns the correct label for all four
  (locale × installed) combinations.
- **TS HUD view-model builder** (prior art: `src/hud.test.ts`). With `locale = en`
  vs `zh`, the needs-human text and the `Idle` fallback differ as expected; data
  fields (percent, countdown, model) are identical across locales.
- **TS `i18n` lookup** (new unit test). `t(locale, key)` returns the right string
  per locale; keys exist in both maps (no missing-translation gaps).
- **Manual / human-verify.** Menu rebuild, checkmark state, and live HUD + settings
  refresh on switch are event/window wiring across processes — not unit-testable;
  verified by running the app and toggling language.

## Out of scope
- Any third language beyond English and 中文.
- The speech bubble (emits no text in the HUD product; wiring left untouched).
- Translating data strings: model name, usage percentages, countdown timers, the
  activity text supplied by Claude, and the `"—"` unavailable placeholder.
- OS-locale auto-detection on first run (default is always English).
- Localizing logs, error messages, or any developer-facing output.

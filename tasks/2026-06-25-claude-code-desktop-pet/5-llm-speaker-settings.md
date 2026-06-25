---
id: 5
slug: llm-speaker-settings
prd: docs/prds/2026-06-25-claude-code-desktop-pet.md
state: done
category: enhancement
blocked_by: [3]
---

## What to build
Give the pet real personality at special moments via an optional LLM voice, plus the
settings to control it. The `Speaker` trait from slice 3 gains a second
implementation, `LlmSpeaker`, that generates a contextual line (default model: Claude
Haiku) at special moments only — a hard-won success, a level-up, a reunion after
absence — gated by a cooldown so the LLM is never spammed. Common moments still use
`TemplateSpeaker`; with the LLM toggled off the pet behaves exactly as in slice 3.

Privacy is a hard constraint: only a de-sensitized summary is ever sent — event type
+ pet state + a short gist — never raw code, commands, or file paths.

A settings surface lets the user toggle the LLM on/off, choose the provider, and
enter their API key. The key is stored locally and never committed (the repo already
gitignores `.env*`).

Confirm the exact Claude model id and pricing via the `claude-api` skill during
implementation rather than hard-coding from memory.

## Key interfaces
- `LlmSpeaker` implementing `Speaker.speak(context) → line` — calls the configured LLM with a de-sensitized prompt; honors a cooldown; falls back to a template line on error/timeout/disabled.
- Special-moment trigger policy — which moods/events qualify (e.g. success-after-struggle, level-up, reunion) and the cooldown window.
- De-sensitized context builder — `event/state → summary`; asserts no raw code/commands/paths are included.
- Settings — persisted config for: LLM enabled flag, provider selection, API key. Read by the speaker selection at runtime; editable from the UI.
- Provider selection — choosing the active `Speaker` from config (Template-only vs Template + LLM).

## Acceptance criteria
- [ ] With the LLM enabled and a valid key, a qualifying special moment shows an LLM-generated line.
- [ ] A cooldown prevents more than the configured rate of LLM calls.
- [ ] With the LLM disabled, the pet shows only template lines and makes no network calls (behaves like slice 3).
- [ ] The context builder is unit-tested to contain only event type + pet state + gist — never raw code, commands, or paths.
- [ ] `LlmSpeaker` is exercised in tests through the `Speaker` trait with a mock — no real API calls in the test suite (seam 4).
- [ ] Settings UI can toggle the LLM, pick a provider, and store an API key locally; the key is never written to a committed file.
- [ ] On LLM error/timeout the pet falls back to a template line (never goes silent, never blocks).

## Out of scope
- A hosted/proxy LLM backend — the call goes direct from the app with the user's key (out of PRD scope).
- Deep telemetry context (token/cost) — not part of the summary.
- The right-click menu entry that opens settings (slice 6 wires the entry; the settings surface itself is built here).

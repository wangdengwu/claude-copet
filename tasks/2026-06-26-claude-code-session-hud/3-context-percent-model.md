---
id: 3
slug: context-percent-model
prd: docs/prds/2026-06-26-claude-code-session-hud.md
state: ready-for-agent
category: enhancement
blocked_by: [2]
---

## What to build

Show the active session's **context-used %** (as a colour-banded progress bar)
and **current model** on the card, derived by reading the tail of that session's
transcript.

End-to-end: when an event carries a `transcript_path`, the watcher reads only the
**tail** of that file (a bounded read — never load a huge transcript fully), finds
the latest assistant message's token usage and model, computes a percentage against
the model's context window, and includes `model` + `contextPercent` in the `hud`
event. The card renders a model badge and a progress bar whose fill width and
colour band (green → amber → red) come from a pure `formatHud` mapping. If the
transcript is missing or unreadable, the bar/percent degrade gracefully to "—"
while the rest of the card keeps working.

## Key interfaces

- `latest_usage_and_model(tail_bytes: &[u8]) -> Option<{ usage, model }>` — **new
  pure fn**: parse JSONL tail lines, skip malformed, return the last assistant
  message that carries a `usage` block (with its `model`). `None` if none found.
- `context_window(model_id: &str) -> u64` — **new pure fn**: mapping table; default
  200_000; `…[1m]` variants → 1_000_000; unknown ids fall back to the default.
- `context_percent(usage, model_id: &str) -> f64` — **new pure fn**:
  `(input_tokens + cache_read_input_tokens + cache_creation_input_tokens) /
  context_window(model_id)`, expressed as a percentage, clamped to `[0, 100]`.
- `model_friendly_name(model_id: &str) -> String` — **new pure fn**:
  "claude-opus-4-8" → "Opus 4.8"; unknown ids degrade to the raw id.
- Watcher — for the active session, read the bounded tail of `transcript_path`,
  derive model + context %, and add them to the `hud` payload. Missing/unreadable
  transcript → omit/flag so the frontend shows "—".
- `hud` payload — gains `model: string | null` and `contextPercent: number | null`.
- Frontend `formatHud(hudState) -> view model` — **new pure mapping**: produces the
  context-bar fill width, the colour band by threshold, and the model-badge text;
  handles the null/"—" case.

## Acceptance criteria

- [ ] `latest_usage_and_model` tests: well-formed tail, tail with interleaved malformed lines, no assistant message (→ None), multiple assistant messages (→ picks the last).
- [ ] `context_window` tests: known models, `[1m]` variants, unknown-id fallback.
- [ ] `context_percent` tests: representative usages, clamping at 0 and 100, sums all three token components.
- [ ] `model_friendly_name` tests: known ids map to friendly names; unknown id returns the raw id.
- [ ] The transcript is read only as a bounded tail (does not read the whole file).
- [ ] The card shows the friendly model name and a progress bar that fills and changes colour (green/amber/red) as % rises.
- [ ] When `transcript_path` is missing/unreadable, the bar/percent show "—" and the rest of the card (label, later activity/alert) still works.
- [ ] `formatHud` vitest covers fill width + colour band at threshold boundaries and the null case.
- [ ] `cargo test` and `pnpm test` pass.

## Out of scope

- Current activity and the needs-human alert — slice 4 (may share the `formatHud`/card but its fields land there).
- Token cost in currency; historical/trend charts.
- Recomputing context % without a transcript (graceful "—" is the defined behaviour).

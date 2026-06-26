---
id: 1
slug: parse-context-refactor-percent
prd: docs/prds/2026-06-26-real-context-window.md
state: ready-for-agent
category: enhancement
blocked_by: []
---

## What to build

The pure-function layer for accurate context-percentage calculation.

1. **New `parse_context_output`** — parses the stdout of `claude -p --resume "/context"` into a struct carrying the model alias and window size. `None` if either required line is missing or unparseable.

2. **Refactor `context_percent`** — change its signature from `(usage, model_id)` to `(usage, window_size: u64)`. The caller now provides the window explicitly; the function does the arithmetic and clamping only — no internal window derivation, no heuristic. The old "if usage > 200k, upgrade to 1M" logic moves to a separate L3 fallback path (next slice), not in this function.

3. **Keep `context_window`** unchanged as a standalone helper — still used by the L3 fallback path.

4. **Model comparison helper** — a function that takes two model strings (e.g. the transcript's model id and the previously-seen one) and returns whether they represent the same model. Exact string equality is sufficient: transcript model ids are canonical and stable across turns for the same model.

End-to-end: after this slice, the pure functions for parsing /context output and computing an accurate percentage with an explicit window are in place and tested. The watcher is NOT changed yet — that's slice 2.

## Key interfaces

- `parse_context_output(stdout: &str) -> Option<ContextInfo>` — new pure fn. ContextInfo carries `model_alias: String` and `window_size: u64`. Extracts from lines like `**Model:** opus[1m]` and `**Tokens:** 170k / 1m (17%)`. Handles `k`/`m`/`K`/`M` suffixes in token counts. None if Model or Tokens line missing, or if window_size parses to 0.
- `context_window(model_id: &str) -> u64` — unchanged contract; still used by L3 fallback.
- `context_percent(usage: &Usage, window_size: u64) -> f64` — signature change: `model_id` parameter replaced by `window_size`. Arithmetic + clamping only. Returns 0.0 if window_size is 0.
- `model_changed(previous: Option<&str>, current: &str) -> bool` — new pure fn. True when `previous` is Some and differs from `current` (exact string compare). False when `previous` is None (first observation) or equal.

## Known data variants

The `/context` stdout shape is a markdown fragment — no JSON structure:

```
## Context Usage

**Model:** deepseek-v4-pro[1m]  
**Tokens:** 170k / 1m (17%)
```

Variants the parser must handle:
- Model: `opus[1m]`, `sonnet`, `haiku-4-5`, `deepseek-v4-pro[1m]`, `claude-opus-4-8` — any alias with optional `[1m]`
- Window: `200k` (200,000), `1m` (1,000,000), `1M` (1,000,000), `500k` (500,000), bare numbers like `200000`
- Used: `164.9k` (164,900), `1.2m` (1,200,000), bare numbers like `215248`
- Parser is lenient: extra whitespace, leading/trailing text on the line is ignored as long as the `Model:`/`Tokens:` prefix is found

Edge cases for `model_changed`:
- First observation (previous=None) → false (not a change, just initialisation)
- Same canonical id seen again → false
- Different canonical id → true (the model actually changed)

## Acceptance criteria

- [ ] `parse_context_output` parses a real `/context` output (the exact text from the PRD's experiment) → correct model_alias and window_size.
- [ ] `parse_context_output` handles `k`/`K` suffix (`×1,000`), `m`/`M` suffix (`×1,000,000`), and bare numbers in both the used and window fields.
- [ ] `parse_context_output` returns None when Model line is missing, Tokens line is missing, or window_size would be 0.
- [ ] `context_percent` with explicit window: `(usage=100k, window=1M)` → 10.0; `(usage=200k, window=200k)` → 100.0; `(usage=250k, window=200k)` → 100.0 (clamped); `(usage=0, window=0)` → 0.0.
- [ ] `model_changed`: `(None, "claude-opus-4-8")` → false; `(Some("claude-opus-4-8"), "claude-opus-4-8")` → false; `(Some("claude-opus-4-8"), "claude-sonnet-4-6")` → true.
- [ ] Existing `context_window` and `latest_usage_and_model` tests stay green.
- [ ] `cargo test` passes.

## Out of scope

- The `/context` CLI subprocess call — slice 2.
- ContextClient trait, cache, trigger logic — slice 2.
- The "upgrade to 1M if usage > 200k" heuristic is removed from `context_percent` but NOT yet re-added to a fallback path — slice 2 handles the L3 fallback.
- Frontend changes (the HUD card already handles `model` and `contextPercent` fields correctly).

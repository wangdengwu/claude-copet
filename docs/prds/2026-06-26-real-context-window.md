# Real Context Window (PRD)

## Problem

The HUD calculates `context_percent` by dividing observed usage by a guessed
window size. `context_window()` only reads the transcript's `model` field, which
is always a canonical id like `claude-opus-4-8` — never carrying a `[1m]`
marker. So the window defaults to 200k. A heuristic "if usage > 200k, upgrade to
1M" was added, but 1M sessions with usage still under 200k read 2–5× too high
(e.g. a real 10% shows as ~50%).

The user wants the **real context window** for the active session, not a guess.

## Solution

`claude -p --resume <session-id> "/context"` outputs the authoritative model
alias and window size non-interactively:

```
Model: deepseek-v4-pro[1m]
Tokens: 170k / 1m (17%)
```

Parse this to obtain the real window, cache it, and use it as the denominator
for `context_percent`. The numerator (current usage) still comes from the cheap
transcript-tail read that runs every tick — only the window is cached from
`/context`.

`/context` is triggered only on state transitions (no timers): startup, a
session switch, or when the transcript model id diverges from the cached alias.

## User Stories

1. As a developer on a 1M-context session, I want the context bar to show the
   real percentage against 1M, so that I have an accurate headroom reading at
   a glance.
2. As a developer switching between sessions with different models, I want the
   HUD to re-derive the window automatically, so that each session's bar is
   accurate without me doing anything.
3. As a developer who runs `/model` mid-session, I want the HUD to notice the
   model mismatch and refresh its window cache, so that the percentage stays
   correct after the switch.
4. As a developer on a standard 200k session, I want the context bar to keep
   working exactly as before, so that the feature is a pure improvement with no
   regression.

## Implementation decisions

### Three-tier information chain

| Tier | Source | Trigger | Purpose |
|---|---|---|---|
| L1 | `claude -p --resume <sid> "/context"` | Startup, session switch, model mismatch | Window size (authoritative) |
| L2 | Transcript tail (512 KB, fast) | Every 250ms tick | Current usage (numerator) |
| L3 | Heuristic fallback (L1 never succeeded) | L1 unavailable | Degraded window guess |

The window from L1 and the usage from L2 are combined: `used / cached_window * 100`.

### `/context` invocation

- Runs in a separate thread (does not block the 250ms watcher loop).
- Call-sites: (1) first event arrival after watcher startup, (2) active session
  id changes, (3) `latest_usage_and_model.model` differs from the last
  `/context`-derived model alias (after normalisation).
- No cooldown, no timer — purely state-driven.
- Output parsed by a new pure function that extracts `model_alias` and
  `window_size` from the stdout text (regex on `Model:` and `Tokens:` lines).
- On failure: retry once after 2s; if still failing, fall through to L3.

### Transcript usage feed (L2, existing)

- `latest_usage_and_model()` stays as-is — still reads the tail, skips
  malformed, returns the last assistant usage.
- `context_percent()` is simplified: the denominator is the cached L1 window
  when available. The "if usage > window, upgrade to 1M" heuristic is removed
  from the L2 path and only kept in L3 (the never-had-L1 fallback).

### Cache structure

```
CachedContext {
    model_alias: String,   // as returned by /context, e.g. "opus[1m]"
    window_size: u64,      // e.g. 1_000_000
    // no timestamp — state-driven invalidation only
}
```

Set to `None` at watcher start. Populated on first successful `/context`.
Replaced on session switch or on model mismatch.

### Model mismatch detection

When the transcript yields a `model` (canonical id, e.g. `claude-opus-4-8`) and
the cache holds a `model_alias` (e.g. `opus[1m]`), normalise both sides —
strip `claude-` prefix, strip `[1m]` suffix, strip dated suffix — and compare.
Mismatch → trigger a new `/context` call. (The `/model` command at runtime
updates the alias; the transcript's `model` field reflects whichever model
actually ran.)

### Model display

- `model_friendly_name()` stays unchanged — it maps canonical ids to display
  names. The `/context`-derived alias is used only for window determination;
  the transcript's `model` field still feeds the display pipeline.

### `/context` stdout parser

Pure function, unit-tested:

```
fn parse_context_output(stdout: &str) -> Option<ContextInfo>
```

ContextInfo carries `model_alias` (String) and `window_size` (u64). Parses the
`Model:` line (e.g. `deepseek-v4-pro[1m]`) and the `Tokens:` line (e.g.
`170k / 1m (17%)`). `None` if either line is missing or unparseable.

### watcher loop changes

- On startup: mark `context_needs_refresh = true`.
- On session switch: `context_needs_refresh = true`.
- On transcript read yielding a `model` that doesn't match the cached alias:
  `context_needs_refresh = true`.
- When `context_needs_refresh` and no call is in-flight: spawn a thread with
  `claude -p --resume <sid> "/context"`, set in-flight flag.
- Thread exits → parse stdout → update cache → clear in-flight +
  `context_needs_refresh`. On error: retry counter; if retry exhausted, set
  L3 fallback.

### ContextClient trait (subprocess isolation)

A trait abstracts `claude -p --resume` so the trigger logic is testable:

```
trait ContextClient: Send {
    fn fetch_context(&self, session_id: &str) -> Result<String, ()>;
}
```

Production impl uses `std::process::Command`. Test impl returns canned stdout.

## Testing decisions

| Seam | Type | Tests |
|---|---|---|
| `parse_context_output(stdout)` | Pure fn | Normal output with `[1m]`, standard 200k model, missing Model line, missing Tokens line, garbled input → None |
| `ContextClient` trait | Mockable interface | Trigger logic: session switch triggers call, model mismatch triggers call, in-flight prevents duplicate call, retry-once-then-L3 |
| `context_percent` (L2 path) | Pure fn | Existing tests kept; heuristic-removal regression confirmed |
| `model_friendly_name` | Pure fn | Existing tests unchanged |

The CLI subprocess itself is not tested in CI (same policy as the old
`AnthropicClient` / `ClaudeCliClient`).

Prior art in the codebase: the `LlmClient` trait (now removed in slice 1) used
the same pattern to isolate a subprocess behind a testable interface.

## Out of scope

- Reading `~/.claude/settings.json` for the model field (alias format differs
  across providers, not authoritative for a specific session).
- Displaying the window size on the card (the bar + %, model badge are enough).
- Per-session attention scoping (documented v1 limitation).
- Sound or notification-center alerts for context pressure.

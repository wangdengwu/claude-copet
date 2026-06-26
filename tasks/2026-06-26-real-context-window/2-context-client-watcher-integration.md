---
id: 2
slug: context-client-watcher-integration
prd: docs/prds/2026-06-26-real-context-window.md
state: ready-for-agent
category: enhancement
blocked_by: [1]
---

## What to build

Wire the real context window into the watcher. After this slice the HUD shows an accurate percentage: the denominator comes from a cached `/context` call, the numerator from the cheap transcript-tail read.

1. **ContextClient trait** — abstracts the `/context` call so trigger logic is testable. Production impl spawns `claude -p --resume <session-id> "/context"` as a subprocess. Test impl returns canned stdout.

2. **Cache + trigger logic in the watcher** — three triggers, no timers:
   - **Startup**: first event arrival → spawn `/context`
   - **Session switch**: active session id changes → spawn `/context`; also clear the cached transcript model so a mismatch isn't falsely detected on the first read of the new session
   - **Model mismatch**: `latest_usage_and_model.model` from transcript differs from the last-seen transcript model (string equality via slice 1's `model_changed`) → spawn `/context`

3. **L3 fallback** — if `/context` has never succeeded (e.g. CLI not installed), use the existing `context_window(model_id)` for the denominator. The "upgrade to 1M if usage > 200k" heuristic lives ONLY in this fallback path.

4. **Threading** — the `/context` subprocess runs on a separate thread (4.6 seconds, never blocks the 250ms watcher loop). A flag prevents duplicate in-flight calls. On completion: parse stdout → update cache. On failure: retry once after 2s; if the retry also fails, activate L3 fallback and stop retrying until the next trigger.

End-to-end: fire `pnpm tauri dev`, the card shows a percentage computed against the real window from `/context`. Switch sessions or change model mid-session → the HUD re-derives within seconds. If `claude` CLI is absent, the old heuristic still works.

## Key interfaces

- `ContextClient` trait — pure abstraction with one method: `fetch_context(&self, session_id: &str) -> Result<String, ()>`. Production impl: `ClaudeCliContextClient` wrapping `std::process::Command`. Test impl: canned string.
- `CachedContext` struct — `{ model_alias: String, window_size: u64, last_transcript_model: String }`. Stored in the watcher as `Option<CachedContext>` (None = never fetched).
- Watcher state additions — `context_in_flight: bool`, `context_retry_count: u8`, `context_fallback_active: bool` (true when L1 has never succeeded).
- `context_percent` call site in watcher — changed from `context_percent(&um.usage, &um.model)` to `context_percent(&um.usage, cached_window)`, where `cached_window` is `cached.window_size` when cache is Some, else `context_window(&um.model)` (L3).

## Known data variants

The `claude` CLI may not be installed. In that case `Command::new("claude")` fails at spawn — treat as an immediate error (no retry needed for "not found").

When the CLI is present but the session id doesn't exist (e.g. the file was deleted): the process exits non-zero. This triggers the retry-once path before L3.

The `/context` stdout format is covered by slice 1's parser. The production `ContextClient` impl just runs the command and returns the raw stdout string — it does not parse.

## Acceptance criteria

- [ ] Mock ContextClient: startup trigger spawns a call; session switch spawns a call; model stays same → no call; model changes → call spawned; in-flight prevents duplicate; failure retries once then activates L3.
- [ ] L3 fallback: when cache is None (never succeeded), `context_percent` falls through to `context_window(model_id)` with the "upgrade if usage > window" heuristic.
- [ ] The `/context` subprocess runs on a background thread and does not block the watcher's 250ms tick.
- [ ] Cache is cleared on session switch (so the new session re-derives its own window).
- [ ] After a successful `/context` parse, the HUD shows `contextPercent` computed with the real window size as denominator.
- [ ] `cargo test` passes (including new mock-based trigger tests + all existing tests from slice 1).
- [ ] `pnpm tauri dev` — the card shows the correct percentage for a real 1M session (no longer ~100% when it should be ~20%).

## Out of scope

- Reading `~/.claude/settings.json` for the model field.
- Displaying the raw window size or model alias on the card (the existing model badge and % bar are sufficient).
- Per-session attention scoping (documented v1 limitation).
- Re-parsing `/context` output format (done in slice 1).

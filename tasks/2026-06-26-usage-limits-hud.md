---
id: 1
slug: usage-limits-hud
prd: none (ticket — fast lane, no PRD)
state: ready-for-agent
category: feature
blocked_by: []
---

## What to build

Show Claude Code's subscription rate-limit usage on the HUD card: the **5-hour
session window** and the **7-day week window**, each as a percentage plus its
reset time. The data is fetched by shelling out to the Claude CLI — the same
mechanism the card already uses for `/context`.

Confirmed feasible: `claude -p --output-format text "/usage"` runs headless and
prints (among other lines) exactly:

```
Current session: 31% used · resets Jun 26 at 11:59pm (Asia/Shanghai)
Current week (all models): 77% used · resets Jun 30 at 3pm (Asia/Shanghai)
```

Unlike `/context`, `/usage` is **global** (account-wide), NOT per-session — one
fetch serves every session; no `--resume` and no per-session cwd are needed.

Scope, end to end:

1. **Pure parser** that turns `/usage` stdout into a struct with the two
   percentages and the two reset strings.
2. **A usage client** (CLI abstraction, mirrors `ContextClient`) and its
   production impl that runs the command above.
3. **Watcher integration**: fetch once on startup, then poll on a configurable
   interval (default 5 min), plus an on-demand manual refresh that is
   rate-limited (rapid clicks do not re-run the CLI).
4. **Settings**: a persisted refresh-interval choice — 5 / 10 / 15 minutes.
5. **Frontend**: two compact lines under the context % — `5h 31% · 11:59pm`
   and `7d 77% · Jun 30` — a manual refresh button next to them, and an
   interval dropdown (5/10/15) in the settings UI.

## Current behavior

The HUD card shows session label, model, context %, and activity. There is no
indication of how much of the 5-hour or weekly subscription limit is used, nor
when those windows reset. The watcher (`watch_event_log`) already fetches
`/context` on a background thread via `ContextClient` / `ClaudeCliContextClient`
and emits a `HudSnapshot` to the frontend.

## Desired behavior

- On app startup the watcher fetches `/usage` once and the card shows both
  windows' percentages and reset times within a few seconds.
- Thereafter usage is refreshed automatically every N minutes, where N is the
  persisted setting (default 5; selectable 5 / 10 / 15).
- A refresh button on the card triggers an immediate re-fetch, but is throttled:
  if an actual fetch happened within the last throttle window, the click is a
  no-op (no CLI invocation). The throttle is enforced server-side so UI cannot
  bypass it.
- A failed/garbled `/usage` fetch leaves the last good values on the card (no
  flicker to blank), same "only overwrite on success" discipline the `/context`
  path already uses.
- The two windows render as compact text showing TIME REMAINING until reset
  (not the absolute clock time / date): the 5h window as hours+minutes, the 7d
  window as days+hours — e.g. `5h 31% · 还剩 2h 15m` / `7d 77% · 还剩 3d 8h`.
  The countdown ticks down live. When the reset phrase can't be parsed into an
  instant, the `· 还剩 …` suffix is dropped (show just `5h 31%`).
- **Non-Claude / non-subscription sessions degrade gracefully.** When the
  current setup has no such limits (e.g. DeepSeek or any third-party model, or
  an API-key / non-subscription configuration), `/usage` does NOT print the
  session/week lines. In that case the card shows **nothing** for usage — the
  whole usage block (both rows + the refresh button) is hidden, not rendered as
  `—`, `0%`, "N/A", or a broken half-row. The rest of the card is unaffected.
  When a later fetch does yield the lines (e.g. the user switches back to a
  Claude model), the block reappears.

## Key interfaces (by contract, not path)

- `parse_usage_output(stdout: &str) -> Option<UsageLimits>` — new pure fn.
  `UsageLimits` carries `session_percent: u8`, `session_reset: String`,
  `week_percent: u8`, `week_reset: String`.
  - `session_*` comes from the line beginning `Current session:`.
  - `week_*` comes from the line beginning `Current week` (the `(all models)`
    qualifier may or may not be present — match on the `Current week` prefix).
  - Percent is the integer before `%` in `NN% used`.
  - Reset string is the text after `resets ` up to end of line, trimmed (keep
    the human phrase, e.g. `Jun 26 at 11:59pm (Asia/Shanghai)`; the frontend
    decides how compactly to render it).
  - Returns `None` if either required line is missing or its percent does not
    parse. Lenient about surrounding whitespace and any other lines in the
    output (the `/usage` text also contains "contributing to your limits"
    breakdown lines, which must be ignored).
- `UsageClient` trait (mirrors `ContextClient`): `fn fetch_usage(&self) ->
  Result<String, ()>`. Production impl runs
  `claude -p --output-format text "/usage"` and returns stdout; `Err(())` on
  non-zero exit or spawn failure. Keep it injectable so the watcher stays
  testable without the real CLI.
- A Tauri command `refresh_usage()` (frontend-invocable) that requests an
  immediate usage re-fetch, subject to the server-side throttle.
- `HudSnapshot` gains an optional usage payload (serialized for the frontend),
  e.g. a nested object `usage: { sessionPercent, sessionReset, weekPercent,
  weekReset } | null`. **Null** until the first successful fetch AND whenever the
  current setup reports no limits (parse → `None`). The frontend treats `null`
  as "hide the usage block entirely" — null is the single, model-agnostic signal
  for "this setup has no Claude usage limits to show". Do not invent a separate
  "unsupported" flag; absence of data IS the signal.
- `Settings` gains `usage_refresh_minutes` (default 5; persisted; the existing
  `get_settings` / `set_settings` commands round-trip it). Only 5, 10, 15 are
  offered by the UI; an out-of-range persisted value falls back to the default.

## Known data / behavior variants

- The week line may read `Current week (all models):` or just `Current week:` —
  match on the `Current week` prefix, not the parenthetical.
- Percentages are integers in the observed output (`31%`, `77%`). Treat a
  decimal defensively (parse the integer part) but integer is the expected case.
- Reset phrasing varies (`resets Jun 26 at 11:59pm (Asia/Shanghai)`,
  `resets Jun 30 at 3pm (...)`). Do not try to parse it into a timestamp — carry
  it as an opaque display string.
- `/usage` output also contains many other lines (request counts, top skills,
  etc.). The parser must ignore everything except the two target lines.
- Each `/usage` invocation costs one CLI request against the account. Honor the
  interval and throttle so the feature does not meaningfully inflate usage.
- **Two failure modes must be told apart:**
  - *Transient* — the CLI fails to spawn / non-zero exit / timeout
    (`fetch_usage` → `Err(())`). Treat as "no new info": keep the last good
    payload on the card, do not blank it.
  - *Definitively no limits* — the CLI succeeds (`Ok(stdout)`) but the output
    has no session/week lines (`parse_usage_output` → `None`). This is the
    non-Claude / API-key case: set the snapshot usage payload to `null` so the
    UI hides the block.

## Compatibility — non-Claude / non-subscription setups

Only a Claude subscription has the 5-hour and weekly windows. With a third-party
model (DeepSeek, etc.) or an API-key configuration, `/usage` prints a different
message (e.g. about a custom API key) without the two lines, so
`parse_usage_output` returns `None`. Requirements:

- Never panic / unwrap on the unexpected output — the parser is total and
  returns `None`, the watcher carries on, the rest of the HUD is unaffected.
- On a successful-but-no-limits fetch, emit `usage: null` → the frontend hides
  the entire usage block (no `—`/`0%`/"N/A"/empty bar). The card looks
  intentionally clean, as if the feature simply isn't there for this setup.
- **Back-off to avoid pointless CLI calls:** after a successful fetch that yields
  no limit lines, slow auto-polling for the no-limits case — e.g. after 2
  consecutive no-data results, stop the interval poll and only re-check on a
  manual refresh or when the active session's transcript model changes
  (the watcher already tracks model changes for `/context`). Reset to normal
  polling once limits reappear. Goal: a DeepSeek-only user should not spawn
  `claude -p "/usage"` every 5 minutes forever.

## Refresh / throttle rules (precise)

- Startup: one fetch as soon as the watcher starts.
- Interval: re-fetch when `now - last_successful_fetch >= usage_refresh_minutes`.
  Reading the interval each tick means a settings change takes effect without
  restart.
- Manual: `refresh_usage()` forces a fetch on the next tick UNLESS an actual
  fetch (manual or interval) occurred within the throttle window
  (suggested minimum gap: 30s). A throttled manual request is silently dropped.
- Only one `/usage` fetch in flight at a time (reuse the in-flight-guard pattern
  from the `/context` path).

## Acceptance criteria

- [ ] `parse_usage_output` parses the real two-line sample → `session_percent=31`,
      `session_reset` contains `11:59pm`, `week_percent=77`, `week_reset`
      contains `Jun 30`.
- [ ] `parse_usage_output` matches the week line whether it says
      `Current week (all models):` or `Current week:`.
- [ ] `parse_usage_output` returns `None` when the session line is missing, when
      the week line is missing, or when a percent is non-numeric.
- [ ] `parse_usage_output` ignores the surrounding `/usage` breakdown lines and
      still extracts the two windows.
- [ ] `Settings` round-trips `usage_refresh_minutes`; a missing/old settings file
      loads the default (5); an out-of-range value falls back to 5.
- [ ] The throttle drops a manual refresh that lands within the min-gap of a
      prior fetch (unit-testable via the pure throttle decision, e.g. a fn that
      takes last-fetch instant + now + min-gap → should-fetch bool).
- [ ] With a stub `UsageClient`, a successful fetch surfaces both windows in the
      emitted snapshot; a failing fetch (`Err`) leaves the previous values intact.
- [ ] `parse_usage_output` returns `None` (no panic) on a non-Claude / API-key
      `/usage` output that lacks the session/week lines — e.g. a sample like
      "You are using a custom API key …" with no limit lines.
- [ ] A successful-but-no-limits fetch sets the snapshot usage payload to `null`
      (distinct from the transient-failure case, which preserves last values).
- [ ] Frontend hides the whole usage block (both rows + refresh button) when the
      usage payload is `null`; it does NOT render `—`, `0%`, "N/A", or an empty
      bar, and the rest of the card layout is unchanged.
- [ ] Back-off holds: after 2 consecutive no-limits fetches the interval poll
      stops issuing `/usage`; a manual refresh or a transcript model change can
      still re-trigger; normal polling resumes once limits reappear.
- [ ] Frontend renders two compact lines with reset times under the context %,
      shows a refresh button, and offers a 5/10/15 interval dropdown that
      persists via `set_settings`.
- [ ] `cargo test` green; existing `/context`, mood, settings tests unaffected.

## Out of scope

- Per-session usage attribution or the "what's contributing to your limits"
  breakdown — only the two top-line windows.
- (Superseded) The reset phrase IS now parsed into an instant to drive a live
  countdown (5h → h+m, 7d → d+h). The phrase's timezone parenthetical is assumed
  equal to the machine-local tz; an unparseable phrase drops the countdown
  suffix rather than showing an absolute date/time.
- Historical charts, notifications/alerts when nearing a limit, or any
  cross-device aggregation.
- Changing the `/context` fetch path.

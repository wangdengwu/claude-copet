# Claude Code Session HUD (PRD)

> Supersedes the product direction in `docs/prds/2026-06-25-claude-code-desktop-pet.md`.
> That PRD's pixel-pet/growth product is being retired; the perception pipe, mood
> machine, settings, and release pipeline it built are reused here.

## Problem

The current product is a pixel pet that reacts to Claude Code activity and grows
over time. In daily use it's **decorative, not useful** — it tells you a mood, but
not the things a developer actually wants at a glance while Claude Code runs in
another window:

- How full is the context window right now? (Am I about to need a compact / fresh session?)
- Does Claude need me right now — a permission prompt or my input — or can I keep doing something else?
- Which model is this session on?
- What is Claude doing this second?

Today none of these are visible. The pet also can't tell you **which session** it's
reflecting when several Claude Code windows are open.

## Solution

Repurpose claude-copet into a compact, always-on-top **status HUD** for the
currently-active Claude Code session. A small mood pet stays in the corner of the
card as an accent (driven by the existing mood machine), but the card's job is now
practical, always-visible status:

- **Session label** — the working directory's name (cwd basename), so you know which session you're looking at. Full session id on hover.
- **Current model** — friendly name (e.g. "Opus 4.8").
- **Context used %** — a coloured progress bar (green → amber → red) with a number.
- **Current activity** — what Claude is doing now (e.g. "Running Bash", "Idle").
- **Needs-human alert** — when Claude is waiting on permission/input, the activity line becomes a prominent warning and the whole card turns amber and pulses.

The HUD reflects the **most-recently-active session**: whichever session produced
the latest event owns the card; when focus shifts (a different session emits an
event), the label/model/context switch with it.

The growth system (XP / level / evolution / daily-stats / `state.json`) is
**removed entirely** — code and persistence deleted, not merely hidden.

Layout: **card with the pet on the left, info stacked on the right.**

```
╭────────────────────────────────╮
│ ╭────╮ claude-copet · Opus 4.8  │   session label (cwd) · model
│ │ 🐤 │ ▓▓▓▓▓▓▓░░░ 62%           │   context-used bar (colour by %)
│ ╰────╯ ⚙ Running Bash    ● live │   current activity · live dot
╰────────────────────────────────╯

needs-human state — whole card amber + pulse:
╭────────────────────────────────╮
│ ╭────╮ claude-copet · Opus 4.8  │
│ │ 🐤 │ ▓▓▓▓▓▓▓░░░ 62%           │
│ ╰────╯ ⚠ 等你输入 / 授权          │
╰────────────────────────────────╯
```

## User Stories

1. As a developer, I want a small always-on-top card showing my active Claude Code session's status, so that I can monitor it while working in another window.
2. As a developer, I want to see the context-used percentage of the active session, so that I know when I'm approaching the limit and should compact or start fresh.
3. As a developer, I want the context bar to change colour as it fills (green → amber → red), so that I can judge headroom at a glance without reading the number.
4. As a developer, I want a clear alert when Claude is waiting on my permission or input, so that I don't leave it idle while I'm focused elsewhere.
5. As a developer, I want the alert to be unmissable (card turns amber and pulses), so that I notice it from peripheral vision.
6. As a developer, I want to see which model the active session is using, so that I can confirm I'm on the model I intended.
7. As a developer, I want to see what Claude is doing right now (which tool, or idle), so that I have a live sense of progress.
8. As a developer, I want a session label (the project / working-directory name), so that I know which of my open sessions the HUD is reflecting.
9. As a developer running several sessions, I want the HUD to follow the most-recently-active session and switch its label when focus moves, so that the card always reflects "the one that just did something".
10. As a developer, I want to see the full session id on hover, so that I can disambiguate two sessions in the same directory if needed.
11. As a developer, I want a small mood pet to remain as a friendly accent reflecting busy / waiting / error / idle, so that the card still has personality without being the point.
12. As a developer, I want the card to be draggable and remember its position, so that I can place it out of the way (reusing existing window-state behaviour).
13. As a developer, I want one-click "Connect to Claude Code" to install the (newly enriched) hooks, so that setup stays effortless after the data-pipeline change.
14. As a developer, I want the hook to stay fire-and-forget and never block or fail Claude Code, so that installing the HUD carries zero risk to my sessions.
15. As a developer who already installed the old hooks, I want re-running Connect to upgrade them to the enriched version, so that context-% and model start working without manual edits.
16. As a developer, I want no XP/level/growth UI anywhere, so that the product is purely the practical status I asked for.

## Implementation decisions

### Enriched event log (hook → `events.jsonl`)

- The hook continues to append one JSON line per event, fire-and-forget, always exit 0, no `jq` dependency (same `sed`-from-stdin technique already in `claude-copet-hook.sh`).
- **New fields per line:** `cwd` and `transcript_path` (extracted from the hook's stdin payload alongside the existing `tool_name` / `session_id`). Final line shape: `{ ts, type, tool, session, cwd, transcript_path }`.
- The event `parse` seam gains `cwd` and `transcript_path` as optional fields (malformed/missing still skipped/tolerated, exactly as today).

### Session-context derivation (Rust core)

A new pure module derives the HUD's data from the latest event + that event's transcript:

- **Active session** = the session/cwd/transcript_path of the most-recently-consumed event. The watcher tracks "current session context" and updates it whenever a new event carries a (possibly different) session.
- **Context % + model** — read only the **tail** of `transcript_path` (last N KB, bounded read — never load a huge transcript fully), parse JSONL lines, and take the latest assistant message's `usage` and `model`.
  - `latest_usage_and_model(tail_bytes) -> Option<{ usage, model }>` — pure; skips malformed lines; scans for the last assistant message bearing a `usage` block.
  - `context_percent(usage, model_id) -> f64` — pure: `(input_tokens + cache_read_input_tokens + cache_creation_input_tokens) / window(model_id)`, clamped to `[0,100]`.
  - `context_window(model_id) -> u64` — pure mapping table (default 200_000; `…[1m]` variants → 1_000_000). Unknown ids fall back to a sane default.
  - `model_friendly_name(model_id) -> String` — pure mapping ("claude-opus-4-8" → "Opus 4.8"); unknown ids degrade to the raw id.
- **Needs-human attention** — a pure flag transition driven by the existing event stream:
  - `attention_step(flag, event) -> flag`: set on `Notification` and `Stop`; cleared on `UserPromptSubmit` and `PreToolUse`. (Notification = permission/input wait; Stop = turn finished / your turn.)
- **Current activity** — derived from events already parsed: `PreToolUse.tool` → "Running <tool>"; combined with the mood machine's busy/idle to render "Idle" when nothing is running.
- **Session label** — `session_label(cwd) -> String`: the last path component of `cwd`. Pure.

### Emission contract (Rust → frontend)

- Replace the growth-oriented `pet_state` / `stage` events with a single **`hud`** event carrying the full snapshot: `{ sessionLabel, sessionId, model, contextPercent, activity, needsHuman }`. Emitted on startup and whenever any field changes.
- **`mood`** continues to be emitted (drives the corner pet) — unchanged contract.
- **`speech`** retained but **off by default** for this product; the bubble is no longer central. (Kept as an optional surface; not required by any HUD story.)

### Frontend (HUD card)

- Render the "card · pet-left" layout: corner pet (existing sprite/render loop) + three info rows.
- `formatHud(hudState) -> view model` — pure mapping that decides the context-bar fill width, the **colour band** (green/amber/red thresholds), the activity text, and the needs-human styling switch. This is the primary frontend seam.
- Needs-human state toggles a card-level CSS class (amber + pulse) and swaps the activity row for the warning line.
- Drag + position memory reuse the existing `startDragging()` + `tauri-plugin-window-state` behaviour.

### Hook install / Connect

- `install_hooks` / `hooks_install` (merge/remove/detect) updated to install the **enriched** hook. Re-running Connect upgrades an old install in place (same idempotent merge, `.bak` backup preserved).
- Settings panel: keep **Connect/Disconnect**; remove the now-irrelevant AI-written-lines toggle/provider/key UI.

### Removals

- Delete `growth.rs` and all call sites (aggregator, XP/level/stage, `state.json` load/save, `pet_state`/`stage` emissions, daily-stats overlay in the frontend, "Today's stats" menu item).
- Delete the LLM speaker path wired into the watcher (`is_special_moment` / `build_summary` / cooldown / provider clients) and its settings, since AI lines are out of this product. `docs/llm.md` retired.

## Testing decisions

Same philosophy as the existing codebase — pure functions are the seams, verified by behaviour, no IO/clock/Tauri in tests. Prior art: `events::parse`, `mood::step`, `growth::aggregate` (being removed), `hooks_install` unit tests, and the frontend `animationForMood` vitest.

1. **`latest_usage_and_model(tail_bytes)`** — feed sample transcript-tail bytes (well-formed, with intervening malformed lines, with no assistant message, multiple assistant messages → picks the last) and assert the returned usage + model. Mirrors `events::parse`'s "skip malformed, tolerate partial" tests.
2. **`context_percent(usage, model)` + `context_window` + `model_friendly_name`** — table-driven: known models, the `[1m]` variants, unknown-id fallbacks, and clamping at 0 / 100. Pure arithmetic, like `growth::level_for_xp`.
3. **`attention_step(flag, event)`** — drive a sequence of events and assert the flag set/clear transitions (Notification→set, PreToolUse→clear, Stop→set, UserPromptSubmit→clear). Mirrors `mood::step` transition tests.
4. **`session_label(cwd)`** — basenames across `/`-paths, Windows `\`-paths, trailing slash, root, empty.
5. **`events::parse` extension** — existing tests extended to assert `cwd` / `transcript_path` are captured when present and absent-tolerant when missing.
6. **`hooks_install` merge/remove/detect** — existing tests updated for the enriched hook command; assert idempotent re-install upgrades an old entry and detect still recognises a full install.
7. **Frontend `formatHud(hudState)`** — vitest: context-bar width + colour band at threshold boundaries, activity text mapping, needs-human styling switch. Mirrors the existing `animationForMood` pure-mapping test.
8. **Mood seam** — unchanged; existing `mood::step` tests stay green (still drives the corner pet).

Verified by eye (no automated tests, same as before): transparent always-on-top card rendering, the amber-pulse alert, drag/position feel, and live context-% updating against a real Claude Code session.

## Out of scope

- **Multi-session side-by-side display.** v1 follows a single active session only.
- **Token cost / spend in currency.** Context-% only; no $ figures, no TMA1 integration.
- **Historical charts / trends / daily stats.** All accumulated/growth reporting is gone.
- **Per-tool detail beyond the tool name** (no command text, no file paths — keeps the de-sensitisation principle).
- **Windows hook shell portability** — unchanged from prior limitation (POSIX `sh`, needs git-bash); not addressed here.
- **App signing / notarisation** — unchanged; still unsigned (see `docs/release.md`).
- **Reading context-% without a transcript** — if `transcript_path` is missing or unreadable, the HUD shows model/activity/attention but context-% degrades gracefully to "—".

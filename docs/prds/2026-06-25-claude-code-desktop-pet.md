# Claude Code Desktop Pixel-Pet (PRD)

## Problem

Working with Claude Code is a long, often solitary loop — you fire off prompts, watch tools run, hit errors, wait. There's no ambient companion that makes the session feel alive or gives you a lightweight, glanceable sense of "what's happening / how's it going." The user wants a desktop pet that lives alongside Claude Code: reacts to the work in real time, grows as a reward for sustained activity, and occasionally talks — turning a tool session into something with a bit of warmth and personality.

## Solution

A lightweight, always-on-top desktop pixel-pet (built with Tauri) that:

- **Reacts in real time** to Claude Code activity. Hooks append events to a log; the pet shifts mood accordingly — busy while tools run, panicked on errors, happy on completion, dozing when idle.
- **Grows over time** without pressure. Activity earns XP; the pet levels up and evolves through pixel-sprite stages. There is no hunger meter that kills it — neglect simply means it naps. No guilt.
- **Talks** via speech bubbles. Common moments use handwritten template lines (instant, offline, free). Special moments (a hard-won success, a level-up, a reunion after absence) can use an LLM-generated line for personality — by default via the local Claude Code CLI (`claude -p`, reusing the user's existing login, no API key), or via a direct Anthropic API key (Claude Haiku). Only a de-sensitized summary (event type + brief gist) is ever sent to the LLM — never raw code, commands, or paths. The LLM is a pluggable, toggleable provider; the pet works fully without it. (See [Amendments](#amendments).)

The pet is a floating, draggable, transparent window. Click it to pet it; right-click for settings, today's stats, and quit.

## User Stories

1. As a Claude Code user, I want a pet floating on my desktop, so that my coding session feels less solitary.
2. As a user, I want the pet to look busy while Claude Code is running a tool, so that I get an ambient sense that work is in progress.
3. As a user, I want the pet to react with alarm when an error occurs, so that I notice something went wrong without reading logs.
4. As a user, I want the pet to celebrate when a turn completes successfully, so that finishing feels rewarding.
5. As a user, I want the pet to doze/sleep when nothing has happened for a while, so that idle time is visually obvious and calm.
6. As a user, I want the pet to wake and greet me when a new session starts, so that it feels present and responsive.
7. As a user, I want the pet to earn experience and level up from my coding activity, so that consistent use feels rewarding.
8. As a user, I want the pet to evolve through visible pixel stages (egg → juvenile → adult → …), so that long-term progress is tangible.
9. As a user, I want the pet to NOT die or punish me if I'm away for days, so that it stays a low-pressure companion.
10. As a user, I want the pet to say short template lines reacting to events, so that it has a voice even offline.
11. As a user, I want richer LLM-generated lines at special moments, so that the pet feels like it has real personality.
12. As a privacy-conscious user, I want only a de-sensitized summary sent to the LLM, so that my code and commands never leave my machine in full.
13. As a user, I want to toggle the LLM off (or swap providers / set my API key) in settings, so that I control cost and privacy.
14. As a user, I want to drag the pet anywhere and have its position remembered, so that it stays where I put it.
15. As a user, I want to click the pet to "pet" it and get a happy reaction, so that the interaction is tactile.
16. As a user, I want a right-click menu with settings, today's stats, and quit, so that I can control the app.
17. As a user, I want the pet's window to be transparent, frameless, always-on-top, and out of the taskbar, so that it behaves like a real desktop pet.
18. As a maintainer, I want hooks to be fire-and-forget and never block Claude Code, so that the pet never slows down my actual work.
19. As a maintainer, I want the pet to recover its state after restart, so that level/progress persist.
20. As a maintainer, I want cross-platform installers (macOS / Windows / Linux) built automatically on release, so that distribution is hands-off.

## Implementation decisions

This is the high-level design. Five units, one clear data flow.

**Architecture & data flow**
```
Claude Code hooks ──append JSONL──> events.jsonl
                                         │ (file watch)
                                         ▼
   Rust core: Parser → State machine (mood + decay) → emit signals to frontend
                       Growth aggregator (XP/level/stage) ──persist──> state.json
                       Speaker (Template | LLM) ──> line ──> frontend
                                         │
                                         ▼
   Web frontend (Canvas): pixel-sprite animation by mood; speech bubble;
                          transparent / always-on-top / draggable window
```

**Unit 1 — Hook event emitter.** Shell snippets registered on Claude Code hooks (SessionStart, UserPromptSubmit, PreToolUse, PostToolUse, Stop, error/notification). Each appends one JSON line and exits immediately — fire-and-forget, never blocks Claude Code. Contract: appends to `events.jsonl`; line shape `{ ts, type, tool?, session, ... }`. Survives the pet not running (events are not lost; consumed later via offset cursor).

**Unit 2 — Event parser.** Reads new lines from the event log; parses each JSONL line into a typed `Event`. Malformed lines are skipped, not fatal. Tracks a byte/line offset cursor so each event is consumed exactly once (also persisted, to avoid double-counting XP across restarts).

**Unit 3 — Mood state machine (the soul).** Pure transition: `(current_mood, event, elapsed) → (new_mood, signals)`. Events preempt the current mood and reset that mood's decay timer; on timer expiry the mood falls back to `idle`; sustained `idle` → `sleep`. Moods: `wake, listen, work, panic, happy, idle, sleep, tired`. Mood is ephemeral (derived from the event stream), not persisted — after restart it starts at `idle`.

**Unit 4 — Growth aggregator.** Aggregates consumed events into per-day stats (sessions, tool_calls, turns, errors, active_min) and accumulates XP → level → evolution stage. No death/penalty for inactivity. Persisted to `state.json`. Contract: `(prev_pet, new_events) → updated_pet`; idempotent given the offset cursor.

**Unit 5 — Speaker (pluggable).** Interface `Speaker.speak(context) → line`. `TemplateSpeaker` (default, always-on): picks a handwritten line per mood/event. `LlmSpeaker` (optional, off by default): generates a contextual line at special moments only, behind a cooldown, via a pluggable `LlmClient`. Two providers ship: `claude-cli` (**the default** — spawns the local Claude Code CLI `claude -p`, reusing the user's login, no API key) and `anthropic` (direct Messages API with the user's key, model `claude-haiku-4-5`). Context is a de-sensitized summary (event type + pet state + short gist), never raw code/paths/session-id/tool-string. Toggle, provider choice, and API key live in settings (`~/.claude-copet/settings.json`). Model id/pricing confirmed via the `claude-api` skill (`claude-haiku-4-5`, $1/$5 per MTok). See [docs/llm.md](../llm.md).

**Frontend.** Canvas + `requestAnimationFrame` loop plays the sprite sheet for the current mood; evolution stage swaps the sprite set. Speech bubble renders above the sprite and fades after a few seconds. A pure `mood → animation` mapping function isolates the testable logic from rendering. Tauri window: `decorations:false, transparent:true, always_on_top:true, skip_taskbar:true`; window position persisted. Interactions: drag to move, click to pet (→ happy), right-click menu (settings / today's stats / quit).

**Storage.** Plain JSON at `state.json` (pet: birth_date, level, xp, stage, unlocked; daily_stats; cursor). No SQLite — data volume is tiny (YAGNI).

**Persisted state shape (from the agreed design):**
```jsonc
{
  "pet":  { "birth_date": "2026-06-25", "level": 3, "xp": 1240,
            "stage": "juvenile", "unlocked": ["hat_pixel", "anim_dance"] },
  "daily_stats": { "2026-06-25": { "sessions": 4, "tool_calls": 87,
                   "turns": 22, "errors": 3, "active_min": 95 } },
  "cursor": { "events_offset": 10432 }
}
```

**Release.** GitHub Actions with `tauri-apps/tauri-action` builds macOS / Windows / Linux artifacts on tag/release. Local dev uses `tauri dev`.

## Testing decisions

No prior art in this repo (greenfield). Seams confirmed with the user; logic is concentrated in pure Rust functions so the highest-value tests are cheap and deterministic.

- **Seam 1 — Mood state machine (primary).** Feed synthetic event sequences with controlled elapsed time; assert mood transitions and decay fall-back (event preempts → mood; timer expiry → idle; long idle → sleep). This is the single most important seam — it verifies the pet's externally observable behavior (what mood it shows when) without touching rendering.
- **Seam 2 — Growth aggregator.** Given event batches + a prior pet/cursor, assert XP accrual, level/stage transitions, and that re-consuming the same offset never double-counts (idempotency).
- **Seam 3 — Event parser.** Valid JSONL lines parse to typed events; malformed lines are skipped without aborting the stream; offset advances correctly.
- **Seam 4 — TemplateSpeaker.** With a fixed RNG seed, assert deterministic line selection per mood/event. `LlmSpeaker` is exercised through the `Speaker` trait with a mock — no real API calls in tests.
- **Seam 5 — Frontend `mood → animation` mapping.** Pure function: assert each mood maps to the expected animation/sprite key.
- **Integration.** Write synthetic lines into a temp `events.jsonl`; assert the core engine emits the expected frontend signals end-to-end (parser → state machine → emitted signal).

Verified by eye (no automated tests): transparent always-on-top window, sprite animation rendering, drag/click feel.

## Out of scope

- Deep telemetry (token/cost curves, OpenTelemetry/TMA1 integration) — later enhancement; growth uses the hook event log only.
- Multiple evolution branches, accessory/cosmetic shop, multiple pets, social features.
- Hunger-to-death mechanics or any inactivity penalty (explicitly rejected).
- A hosted/proxy LLM backend run by us. The LLM call is always client-side: either through the user's local Claude Code CLI (default) or direct to Anthropic with the user's own key.
- Polished cross-platform UX parity beyond what `tauri-action` produces out of the box.

## Amendments

- **2026-06-25 (slice 5) — LLM provider default.** Original design defaulted the
  optional LLM voice to a direct Anthropic API call (Claude Haiku, user's key).
  Changed the **default** provider to `claude-cli`, which reuses the user's local
  Claude Code login via `claude -p` (no API key to configure). Rationale: the
  pet's audience already runs Claude Code, so this works out of the box. The
  Anthropic-API provider remains available as the alternative. Both sit behind one
  `LlmClient` seam; the privacy contract (de-sensitized summary only) is unchanged.

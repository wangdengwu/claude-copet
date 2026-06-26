# Handoff — claude-copet

> ## Session handoff — 2026-06-26 (current state)
>
> **All 8 slices implemented, merged to `main`, and pushed.** Repo:
> https://github.com/wangdengwu/claude-copet (public). `main` is clean; **cargo 52
> tests + vitest green**; `cargo build` + `pnpm build` clean.
>
> ### What's built (slices 1–8)
> 1. Tauri idle-pet shell (frameless/transparent/always-on-top) + `animationForMood`
> 2. Perception pipe: Claude Code hooks → `~/.claude-copet/events.jsonl` → Rust watcher → mood → sprite
> 3. Mood state machine + decay + `TemplateSpeaker`
> 4. Growth: XP / level / evolution stage + persistence (`~/.claude-copet/state.json`)
> 5. Optional AI-written lines: `LlmClient` seam — **default `claude-cli`** (reuses
>    local Claude Code login, no key) / `anthropic` (API key); de-sensitized summary; cooldown
> 6. Interactions: drag + position memory (`tauri-plugin-window-state`), click-to-pet (Happy), right-click menu
> 7. Release pipeline: `.github/workflows/release.yml` (tauri-action, `v*` tags → draft Release, 3 OSes)
> 8. One-click **Connect to Claude Code** (auto-install hooks; `hooks_install.rs` pure merge/remove/detect)
>
> ### Key files
> - Rust core: `src-tauri/src/lib.rs` (watcher loop + Tauri commands + builder), `speaker.rs`
>   (`Speaker`/`LlmClient`/`ClaudeCliClient`/`AnthropicClient`/`build_summary`/`is_special_moment`),
>   `growth.rs`, `mood.rs`, `events.rs`, `settings.rs`, `hooks_install.rs`
> - Frontend: `src/main.ts` (render loop + listeners + drag/click/menu), `src/settings.ts`
>   (panel: AI-written-lines toggle, provider, key, **Claude Code Connect/Disconnect**), `render.ts`, `sprites.ts`, `bubble.ts`
> - Capabilities: `src-tauri/capabilities/default.json` (needs `core:window:allow-start-dragging` + `allow-close`)
> - Docs: `docs/llm.md`, `docs/hooks.md`, `docs/release.md`; PRD at `docs/prds/…` (see its **Amendments**: LLM default changed to claude-cli); tasks in `tasks/2026-06-25-claude-code-desktop-pet/` (1–8 all `state: done`)
>
> ### Still pending (human / not code)
> - **Visual acceptance** via `pnpm tauri dev`: drag, click→Happy, right-click menu, settings ✕/Esc/outside-close, one-click Connect.
> - **End-to-end release**: push a `v0.1.0` tag and confirm Actions produces installers for all 3 OSes.
> - **Known limits**: Windows hook command uses POSIX `sh` (needs git-bash; slice-8 out of scope); apps are unsigned (see `docs/release.md`).
>
> ### Conventions / how to run
> - Run: `pnpm install` then `pnpm tauri dev`. Tests: `cargo test` in `src-tauri/`, `pnpm test` for vitest.
> - Workflow: weee:dev two-role TDD via subagents; one branch per slice, `--no-ff` merge, delete branch. `.weee/` is gitignored (local ledger at `.weee/progress.md`).
> - Settings + state live under `~/.claude-copet/` (outside the repo); API keys never committed.
>
> ---

## Original handoff (planning → implementation)

Planning is done (discuss → PRD → tasks). This doc is the bridge into implementation.
Read this, then pick up slice 1.

## What this is

A desktop pixel-pet companion for Claude Code (*Co* + *pet*). It floats on the desktop,
reacts in real time to Claude Code activity, grows over time, and talks. Full intent is
in the PRD; the design below is the short version.

## Source of truth

- **PRD:** `docs/prds/2026-06-25-claude-code-desktop-pet.md`
- **Tasks:** `tasks/2026-06-25-claude-code-desktop-pet/` — 7 vertical slices, one file each, ordered by `blocked_by`.
- **Git conventions:** `CONTRIBUTING.md` (commit as `dengwu.wang <me@dengwu.wang>` — already pinned in repo-local git config).

## Locked design decisions

- **Stack:** Tauri (Rust backend + Web frontend). Frontend draws pixel sprites on Canvas.
- **Claude Code integration:** file event log. Hooks append one JSON line to `events.jsonl` (fire-and-forget, never block Claude Code); the Rust core watches + consumes it via an offset cursor. The same log feeds both real-time mood and long-term growth.
- **Gameplay:** reaction-first (real-time mood) + lightweight growth (XP/level/evolution). **No death, no inactivity penalty** — neglect just means the pet naps.
- **Art:** pixel-sprite sheets, frame animation; evolution stage swaps the sprite set.
- **Speech:** hybrid. `TemplateSpeaker` (always-on, handwritten lines) + optional `LlmSpeaker` (default Claude Haiku) at special moments, behind a cooldown. Only a **de-sensitized summary** (event type + pet state + gist) is sent to the LLM — never raw code/commands/paths. Toggle/provider/key live in settings.
- **Storage:** plain JSON (`state.json`). No SQLite (YAGNI).
- **Release:** GitHub Actions + `tauri-apps/tauri-action` builds macOS/Windows/Linux on release.

## Architecture

```
Claude Code hooks ──append JSONL──> events.jsonl
                                         │ (file watch)
                                         ▼
   Rust core: Parser → Mood state machine (decay) → emit mood to frontend
                       Growth aggregator (XP/level/stage) ──persist──> state.json
                       Speaker (Template | LLM) ──> line ──> frontend
                                         │
                                         ▼
   Web frontend (Canvas): sprite animation by mood; speech bubble;
                          transparent · always-on-top · draggable window
```

## Test seams (keep these as pure functions)

1. **Mood state machine** `step(mood, event, elapsed) → (mood, signals)` — primary seam.
2. **Growth aggregator** `aggregate(prevPet, newEvents) → updatedPet` — idempotent via offset cursor.
3. **Event parser** — JSONL line → typed event; skip malformed lines; advance offset.
4. **TemplateSpeaker** — deterministic line under a fixed RNG seed; `LlmSpeaker` mocked through the `Speaker` trait (no real API in tests).
5. **Frontend `animationForMood(mood) → key`** — pure mapping.

Verified by eye (no automated tests): transparent always-on-top window, sprite rendering, drag/click feel.

## Build order (dependency graph)

```
1 (Tauri shell + idle pet)
├─ 2 (perception pipe) ─┬─ 3 (state machine + template speech) ── 5 (LLM speaker + settings)
│                       └─ 4 (growth / persistence)         ← 3 and 4 parallel
├─ 6 (drag / pet / menu)                                     ← needs only 1
└─ 7 (GitHub Actions release)                                ← needs only 1
```

- Critical path: **1 → 2 → 3 → 5**.
- After 1: slices 2, 6, 7 can run in parallel. After 2: slices 3, 4 can run in parallel.

## Start here

```
/weee:dev tasks/2026-06-25-claude-code-desktop-pet/1-tauri-shell-idle-pet.md
```

Slice 1 is the foundation everything else plugs into: transparent/frameless/always-on-top
Tauri window + Canvas frame loop + a single looping idle sprite + the pure
`animationForMood` function (unit-tested) + `tauri dev` runs locally.

## Gotchas / reminders

- When you reach slice 5 (LLM), confirm the exact Claude model id + pricing via the `claude-api` skill before hard-coding — don't go from memory.
- Hooks must never block or fail Claude Code, even when the pet app isn't running.
- The offset cursor is what makes event consumption exactly-once across restarts — don't double-count XP.
- Mood is ephemeral (derived from the event stream), never persisted; growth state is persisted.

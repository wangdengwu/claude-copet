---
id: 4
slug: growth-xp-evolution-persistence
prd: docs/prds/2026-06-25-claude-code-desktop-pet.md
state: done
category: enhancement
blocked_by: [2]
---

## What to build
The養成 layer: the pet grows from sustained coding activity and the progress sticks.
Consumed events are aggregated into per-day stats and accumulated into XP; XP drives
level, and level drives an evolution stage that swaps the pet's sprite set. Progress
persists to disk and survives restart. There is no death and no penalty for
inactivity — being away just means the pet naps.

This builds on the consumed-event stream + offset cursor from slice 2 (not on the
decay state machine), so it can proceed in parallel with slice 3. Demo: activity
raises XP and triggers a level-up and an eventual stage evolution (egg → juvenile →
adult …); quitting and relaunching keeps level/XP/stage; leaving for days causes no
loss.

## Key interfaces
- `aggregate(prevPet, newEvents) → updatedPet` — pure: rolls events into the matching day's stats (sessions, tool_calls, turns, errors, active_min) and accrues XP → level → stage. Idempotent given the offset cursor (re-consuming the same offset never double-counts). Seam 2.
- Persisted state shape (agreed):
  ```jsonc
  {
    "pet":  { "birth_date": "2026-06-25", "level": 3, "xp": 1240,
              "stage": "juvenile", "unlocked": [] },
    "daily_stats": { "2026-06-25": { "sessions": 4, "tool_calls": 87,
                     "turns": 22, "errors": 3, "active_min": 95 } },
    "cursor": { "events_offset": 10432 }
  }
  ```
- Store — load/save plain JSON state; no SQLite (YAGNI). The cursor lives here so consumption is exactly-once across restarts.
- Stage → sprite-set selection — evolution stage chooses which sprite sheets `animationForMood` draws from.

## Acceptance criteria
- [ ] `aggregate` unit tests (seam 2): XP accrues from events; crossing thresholds raises level and stage; re-running with the same offset does not double-count.
- [ ] State persists to JSON and is reloaded on launch (level/XP/stage/cursor restored).
- [ ] Inactivity for an extended period causes no XP loss, no stage regression, no death.
- [ ] An evolution stage change visibly swaps the pet's sprite set.
- [ ] Birth date is recorded once on first run and preserved thereafter.

## Out of scope
- Mood/decay behavior (slice 3) — growth reads the event stream, not the mood.
- Speech (slices 3 and 5).
- Multiple evolution branches, accessory/cosmetic shop (out of PRD scope).
- Deep telemetry / token/cost curves (out of PRD scope).

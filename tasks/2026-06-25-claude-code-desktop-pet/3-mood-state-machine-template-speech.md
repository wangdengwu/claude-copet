---
id: 3
slug: mood-state-machine-template-speech
prd: docs/prds/2026-06-25-claude-code-desktop-pet.md
state: ready-for-agent
category: enhancement
blocked_by: [2]
---

## What to build
The pet's soul: a real mood state machine with decay, plus a voice via handwritten
template lines. This replaces the slice-2 direct event→mood mapping with proper
temporal behavior, and adds a speech bubble.

Behavior: an incoming event preempts the current mood and resets that mood's decay
timer. When a mood's timer expires with no new event, the mood falls back to `idle`.
Sustained `idle` transitions to `sleep`. A long/intense session can show `tired`.
On every mood entry (and on notable events) the pet may speak: the `TemplateSpeaker`
picks a handwritten line for that mood/event and the frontend shows it in a pixel
speech bubble above the sprite that fades after a few seconds. Mood is ephemeral
(derived from the event stream), never persisted; after restart the pet starts at
`idle`.

Demo: trigger events and watch moods preempt and decay back to idle, then to sleep
when quiet; each reaction shows a short spoken line.

## Key interfaces
- `step(currentMood, event, elapsed) → (newMood, signals)` — pure transition function. Events preempt + reset decay; timer expiry → `idle`; sustained `idle` → `sleep`. Moods: `wake, listen, work, panic, happy, idle, sleep, tired`. (Prototype-shaped contract, kept deliberately pure for testing — seam 1.)
- `Speaker` trait — `speak(context) → line`. This slice ships `TemplateSpeaker` (always-on): given mood/event, returns a handwritten line (deterministic under a fixed RNG seed). The trait is the extension point slice 5 plugs `LlmSpeaker` into.
- Speech bubble — frontend renders the returned line above the sprite and fades it after a few seconds.

## Acceptance criteria
- [ ] `step` unit tests (seam 1): event preempts current mood; mood decays to `idle` after its timer; sustained `idle` becomes `sleep`; `tired` reachable from long/intense activity.
- [ ] Mood is not persisted — after restart the pet begins at `idle`.
- [ ] `TemplateSpeaker` returns a deterministic line per mood/event under a fixed seed (seam 4 unit test).
- [ ] Live: events drive mood changes that visibly decay back to idle then sleep when quiet, each with a template speech bubble that fades.
- [ ] Each defined mood has at least one handwritten line and its own sprite animation wired through `animationForMood`.

## Out of scope
- LLM-generated speech, settings, API keys, cooldown (slice 5).
- XP / level / evolution (slice 4).
- Drag/click/menu interactions (slice 6).

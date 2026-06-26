//! The pet's soul: a pure mood state machine with decay (seam 1). Events preempt
//! the current mood and reset its decay timer; with no event, a mood decays to
//! `idle` once its TTL elapses; sustained `idle` becomes `sleep`; long/intense
//! activity surfaces `tired`. No IO/clock here — wall time enters as `delta`, so
//! the whole thing is deterministic and unit-testable. Mood is never persisted.

use std::time::Duration;

use crate::events::{mood_for_event, Event, Mood};

/// The evolving mood state. `elapsed` is time in the current mood; `active_streak`
/// is cumulative non-idle time, which drives `tired` and resets on reaching idle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoodState {
    pub mood: Mood,
    pub elapsed: Duration,
    pub active_streak: Duration,
}

impl MoodState {
    /// Every run starts here — mood is ephemeral, so a restart begins at idle.
    pub fn initial() -> Self {
        Self {
            mood: Mood::Idle,
            elapsed: Duration::ZERO,
            active_streak: Duration::ZERO,
        }
    }
}

impl Default for MoodState {
    fn default() -> Self {
        Self::initial()
    }
}

/// What a transition tells the rest of the system. `mood_changed` is true exactly
/// when this step ENTERS a new mood — the trigger for the speaker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Signals {
    pub mood_changed: bool,
}

// Decay-to-idle TTL per active mood.
const WAKE_TTL: Duration = Duration::from_secs(3);
const LISTEN_TTL: Duration = Duration::from_secs(6);
const WORK_TTL: Duration = Duration::from_secs(5);
const PANIC_TTL: Duration = Duration::from_secs(4);
const HAPPY_TTL: Duration = Duration::from_secs(4);
const TIRED_TTL: Duration = Duration::from_secs(6);
const IDLE_TO_SLEEP: Duration = Duration::from_secs(20);
const TIRED_ACTIVE_THRESHOLD: Duration = Duration::from_secs(60);

/// TTL after which an active mood decays; `None` for idle/sleep (handled separately).
fn decay_ttl(mood: Mood) -> Option<Duration> {
    match mood {
        Mood::Wake => Some(WAKE_TTL),
        Mood::Listen => Some(LISTEN_TTL),
        Mood::Work => Some(WORK_TTL),
        Mood::Panic => Some(PANIC_TTL),
        Mood::Happy => Some(HAPPY_TTL),
        Mood::Tired => Some(TIRED_TTL),
        Mood::Idle | Mood::Sleep => None,
    }
}

fn is_active(mood: Mood) -> bool {
    !matches!(mood, Mood::Idle | Mood::Sleep)
}

/// Pure transition. `event = Some` means an event arrived this step; `None` is a
/// pure time tick. `delta` is wall time since the previous step.
pub fn step(state: &MoodState, event: Option<&Event>, delta: Duration) -> (MoodState, Signals) {
    // Cumulative non-idle time grows whenever we were in an active mood.
    let streak_increment = if is_active(state.mood) {
        delta
    } else {
        Duration::ZERO
    };
    let active_streak = state.active_streak + streak_increment;

    if let Some(ev) = event {
        return match mood_for_event(ev) {
            // A recognized event preempts the current mood and resets its decay timer.
            Some(new_mood) => (
                MoodState {
                    mood: new_mood,
                    elapsed: Duration::ZERO,
                    active_streak,
                },
                Signals {
                    mood_changed: new_mood != state.mood,
                },
            ),
            // Activity that maps to no mood (e.g. PostToolUse) still refreshes the
            // decay timer so the pet stays "busy", but does not change mood.
            None => (
                MoodState {
                    mood: state.mood,
                    elapsed: Duration::ZERO,
                    active_streak,
                },
                Signals {
                    mood_changed: false,
                },
            ),
        };
    }

    // No event → a time tick. Grow elapsed and apply decay rules.
    let elapsed = state.elapsed + delta;

    match state.mood {
        Mood::Idle => {
            if elapsed >= IDLE_TO_SLEEP {
                (
                    MoodState {
                        mood: Mood::Sleep,
                        elapsed: Duration::ZERO,
                        active_streak: Duration::ZERO,
                    },
                    Signals { mood_changed: true },
                )
            } else {
                (
                    MoodState {
                        mood: Mood::Idle,
                        elapsed,
                        active_streak: Duration::ZERO,
                    },
                    Signals {
                        mood_changed: false,
                    },
                )
            }
        }
        Mood::Sleep => (
            MoodState {
                mood: Mood::Sleep,
                elapsed,
                active_streak: Duration::ZERO,
            },
            Signals {
                mood_changed: false,
            },
        ),
        active_mood => {
            let ttl = decay_ttl(active_mood).expect("active mood has a TTL");
            if elapsed >= ttl {
                if active_mood != Mood::Tired && active_streak >= TIRED_ACTIVE_THRESHOLD {
                    // Long/intense stretch — surface tiredness instead of going idle.
                    (
                        MoodState {
                            mood: Mood::Tired,
                            elapsed: Duration::ZERO,
                            active_streak,
                        },
                        Signals { mood_changed: true },
                    )
                } else {
                    // Quiet for long enough — fall back to idle; the streak ends.
                    (
                        MoodState {
                            mood: Mood::Idle,
                            elapsed: Duration::ZERO,
                            active_streak: Duration::ZERO,
                        },
                        Signals { mood_changed: true },
                    )
                }
            } else {
                (
                    MoodState {
                        mood: active_mood,
                        elapsed,
                        active_streak,
                    },
                    Signals {
                        mood_changed: false,
                    },
                )
            }
        }
    }
}

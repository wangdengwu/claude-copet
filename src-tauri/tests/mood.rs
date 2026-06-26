use claude_copet_lib::events::{Event, Mood};
use claude_copet_lib::mood::{step, MoodState, Signals};
use std::time::Duration;

fn make_event(event_type: &str) -> Event {
    Event {
        ts: None,
        event_type: event_type.to_string(),
        tool: None,
        session: None,
        cwd: None,
        transcript_path: None,
    }
}

#[test]
fn initial_is_idle() {
    let s = MoodState::initial();
    assert_eq!(s.mood, Mood::Idle);
    assert_eq!(s.elapsed, Duration::ZERO);
    assert_eq!(s.active_streak, Duration::ZERO);
}

#[test]
fn event_preempts_idle() {
    let s = MoodState::initial();
    let ev = make_event("PreToolUse");
    let (next, sig) = step(&s, Some(&ev), Duration::from_secs(1));
    assert_eq!(next.mood, Mood::Work);
    assert!(sig.mood_changed);
    assert_eq!(next.elapsed, Duration::ZERO);
}

#[test]
fn decay_to_idle_after_ttl() {
    // Start in Work mood with zero streak (below 60s threshold)
    let s = MoodState {
        mood: Mood::Work,
        elapsed: Duration::ZERO,
        active_streak: Duration::from_secs(10),
    };
    // Work TTL = 5s; delta >= 5s should decay to Idle
    let (next, sig) = step(&s, None, Duration::from_secs(5));
    assert_eq!(next.mood, Mood::Idle);
    assert!(sig.mood_changed);
}

#[test]
fn no_premature_decay() {
    let s = MoodState {
        mood: Mood::Work,
        elapsed: Duration::ZERO,
        active_streak: Duration::from_secs(10),
    };
    // delta < 5s — should stay Work, no change
    let (next, sig) = step(&s, None, Duration::from_secs(4));
    assert_eq!(next.mood, Mood::Work);
    assert!(!sig.mood_changed);
}

#[test]
fn idle_to_sleep() {
    let s = MoodState::initial(); // Idle, elapsed=0
    // delta >= 20s — should enter Sleep
    let (next, sig) = step(&s, None, Duration::from_secs(20));
    assert_eq!(next.mood, Mood::Sleep);
    assert!(sig.mood_changed);
}

#[test]
fn tired_reachable() {
    // Build active_streak >= 60s by feeding PreToolUse events and ticks.
    // Each PreToolUse event while active accumulates delta into active_streak.
    let mut state = MoodState::initial();

    // Transition to Work via event, accumulate streak over multiple steps.
    // Step 1: enter Work from Idle (no streak yet from Idle)
    let ev_pre = make_event("PreToolUse");
    let (s1, _) = step(&state, Some(&ev_pre), Duration::from_secs(1));
    assert_eq!(s1.mood, Mood::Work);
    state = s1;

    // Steps 2-13: keep refreshing with PreToolUse events, each adding delta to streak.
    // 12 steps * 5s = 60s active_streak accumulated
    for _ in 0..12 {
        let ev = make_event("PreToolUse");
        let (next, _) = step(&state, Some(&ev), Duration::from_secs(5));
        state = next;
    }

    // Now active_streak should be >= 60s. A decay tick (delta >= Work TTL=5s) should go to Tired.
    let (final_state, sig) = step(&state, None, Duration::from_secs(5));
    assert_eq!(final_state.mood, Mood::Tired, "expected Tired after large active_streak, got {:?}", final_state.mood);
    assert!(sig.mood_changed);
}

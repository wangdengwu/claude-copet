// Integration tests for the `growth` module (Task 4, XP/Level/Stage/Persistence).
// These tests import the public API that does NOT exist yet => compile fails (RED state).
// Do NOT add any stub implementation here.

use claude_copet_lib::events::Event;
use claude_copet_lib::growth::{aggregate, stage_for_level, xp_for_event, PetState, Stage};

// ── Behavior 1: XP accrual from events ──

#[test]
fn xp_accrual_matches_spec() {
    // Fresh state: level-1 egg, zero everything, birth date 2026-06-25.
    let mut state = PetState::initial("2026-06-25".to_string());

    // One of each event type, matching the spec.
    let events = vec![
        Event {
            ts: None,
            event_type: "SessionStart".to_string(),
            tool: None,
            session: None,
        },
        Event {
            ts: None,
            event_type: "UserPromptSubmit".to_string(),
            tool: None,
            session: None,
        },
        Event {
            ts: None,
            event_type: "PreToolUse".to_string(),
            tool: Some("Bash".to_string()),
            session: None,
        },
        Event {
            ts: None,
            event_type: "Stop".to_string(),
            tool: None,
            session: None,
        },
        Event {
            ts: None,
            event_type: "error".to_string(),
            tool: None,
            session: None,
        },
        Event {
            ts: None,
            event_type: "Notification".to_string(),
            tool: None,
            session: None,
        },
        Event {
            ts: None,
            event_type: "PostToolUse".to_string(),
            tool: Some("Bash".to_string()),
            session: None,
        },
        Event {
            ts: None,
            event_type: "SomeUnmappedEvent".to_string(),
            tool: None,
            session: None,
        },
    ];

    // Expected XP: 5 + 10 + 15 + 10 + 20 + 20 + 0 + 0 = 80
    aggregate(&mut state, &events, 0, "2026-06-25");

    assert_eq!(
        state.pet.xp, 80,
        "total XP after one of each event type should be 80"
    );

    // 80 XP is below the level-2 threshold (100), so pet should still be level 1.
    assert_eq!(
        state.pet.level, 1,
        "80 XP should keep the pet at level 1 (threshold for level 2 is 100)"
    );
}

// ── Behavior 2: Level thresholds ──

#[test]
fn level_thresholds() {
    let mut state = PetState::initial("2026-06-25".to_string());

    // Closure for concise event construction.
    let event = |et: &str| Event {
        ts: None,
        event_type: et.to_string(),
        tool: None,
        session: None,
    };

    // Verify xp_for_event returns correct values per the spec.
    assert_eq!(
        xp_for_event(&event("error")),
        20,
        "error events should be worth 20 XP"
    );
    assert_eq!(
        xp_for_event(&event("PreToolUse")),
        15,
        "PreToolUse events should be worth 15 XP"
    );
    assert_eq!(
        xp_for_event(&event("Stop")),
        10,
        "Stop events should be worth 10 XP"
    );

    // ── Threshold 1: exactly 100 XP → level 2 ──
    // 5 error events × 20 XP each = 100 XP.
    let e100: Vec<Event> = (0..5).map(|_| event("error")).collect();
    aggregate(&mut state, &e100, 0, "2026-06-25");
    assert_eq!(
        state.pet.xp, 100,
        "5 error events should give exactly 100 XP"
    );
    assert_eq!(
        state.pet.level, 2,
        "100 XP is the threshold for level 2"
    );

    // ── Threshold 2: 250 XP total → level 3 ──
    // 10 PreToolUse events × 15 XP each = 150 XP (cumulative 250).
    let e150: Vec<Event> = (0..10).map(|_| event("PreToolUse")).collect();
    aggregate(&mut state, &e150, 0, "2026-06-25");
    assert_eq!(
        state.pet.xp, 250,
        "10 PreToolUse events should bring total to exactly 250 XP"
    );
    assert_eq!(
        state.pet.level, 3,
        "250 XP is the threshold for level 3"
    );

    // ── Threshold 3: 500 XP total → level 4 ──
    // 12 error events (240) + 1 Stop event (10) = 250 XP (cumulative 500).
    let mut e250: Vec<Event> = (0..12).map(|_| event("error")).collect();
    e250.push(event("Stop"));
    aggregate(&mut state, &e250, 0, "2026-06-25");
    assert_eq!(
        state.pet.xp, 500,
        "12 error + 1 Stop events should bring total to exactly 500 XP"
    );
    assert_eq!(
        state.pet.level, 4,
        "500 XP is the threshold for level 4"
    );
}

// ── Behavior 3: Stage evolution ──

#[test]
fn stage_evolution() {
    let mut state = PetState::initial("2026-06-25".to_string());

    let event = |et: &str| Event {
        ts: None,
        event_type: et.to_string(),
        tool: None,
        session: None,
    };

    // Fresh state: level 1, egg.
    assert_eq!(state.pet.level, 1, "fresh state starts at level 1");
    assert_eq!(state.pet.stage, Stage::Egg, "level 1 maps to egg");
    assert_eq!(
        stage_for_level(1),
        Stage::Egg,
        "stage_for_level(1) must return Egg"
    );

    // ── Cross egg -> juvenile boundary (level 3, 250 XP) ──
    // 13 error events x 20 XP = 260 XP -> level 3 -> juvenile.
    let e260: Vec<Event> = (0..13).map(|_| event("error")).collect();
    aggregate(&mut state, &e260, 0, "2026-06-25");
    assert_eq!(state.pet.xp, 260);
    assert_eq!(state.pet.level, 3, "260 XP should be level 3");
    assert_eq!(
        state.pet.stage,
        Stage::Juvenile,
        "level 3 must map to juvenile"
    );
    assert_eq!(
        stage_for_level(3),
        Stage::Juvenile,
        "stage_for_level(3) must return Juvenile"
    );

    // ── Still juvenile at level 5 (800 XP) ──
    // 27 more errors x 20 XP = 540 XP (cumulative 800).
    let e540: Vec<Event> = (0..27).map(|_| event("error")).collect();
    aggregate(&mut state, &e540, 0, "2026-06-25");
    assert_eq!(state.pet.xp, 800);
    assert_eq!(state.pet.level, 5, "800 XP should be level 5");
    assert_eq!(
        state.pet.stage,
        Stage::Juvenile,
        "level 5 must still be juvenile"
    );
    assert_eq!(
        stage_for_level(5),
        Stage::Juvenile,
        "stage_for_level(5) must return Juvenile"
    );

    // ── Cross juvenile -> adult boundary (level 6, 1200 XP) ──
    // 20 more errors x 20 XP = 400 XP (cumulative 1200).
    let e400: Vec<Event> = (0..20).map(|_| event("error")).collect();
    aggregate(&mut state, &e400, 0, "2026-06-25");
    assert_eq!(state.pet.xp, 1200);
    assert_eq!(state.pet.level, 6, "1200 XP should be level 6");
    assert_eq!(
        state.pet.stage,
        Stage::Adult,
        "level 6 must map to adult"
    );
    assert_eq!(
        stage_for_level(6),
        Stage::Adult,
        "stage_for_level(6) must return Adult"
    );

    // ── Cross adult -> elder boundary (level 10, 4000 XP) ──
    // 140 more errors x 20 XP = 2800 XP (cumulative 4000).
    let e2800: Vec<Event> = (0..140).map(|_| event("error")).collect();
    aggregate(&mut state, &e2800, 0, "2026-06-25");
    assert_eq!(state.pet.xp, 4000);
    assert_eq!(state.pet.level, 10, "4000 XP should be level 10");
    assert_eq!(
        state.pet.stage,
        Stage::Elder,
        "level 10 must map to elder"
    );
    assert_eq!(
        stage_for_level(10),
        Stage::Elder,
        "stage_for_level(10) must return Elder"
    );
}

// ── Behavior 4: Daily stats across days ──

#[test]
fn daily_stats_across_days() {
    let mut state = PetState::initial("2026-06-25".to_string());

    // Day 1: SessionStart + PreToolUse on "2026-06-25"
    let day1 = vec![
        Event {
            ts: None,
            event_type: "SessionStart".to_string(),
            tool: None,
            session: None,
        },
        Event {
            ts: None,
            event_type: "PreToolUse".to_string(),
            tool: Some("Bash".to_string()),
            session: None,
        },
    ];
    aggregate(&mut state, &day1, 0, "2026-06-25");

    // Day 2: error + UserPromptSubmit on "2026-06-26"
    let day2 = vec![
        Event {
            ts: None,
            event_type: "error".to_string(),
            tool: None,
            session: None,
        },
        Event {
            ts: None,
            event_type: "UserPromptSubmit".to_string(),
            tool: None,
            session: None,
        },
    ];
    aggregate(&mut state, &day2, 0, "2026-06-26");

    assert_eq!(state.daily_stats["2026-06-25"].sessions, 1);
    assert_eq!(state.daily_stats["2026-06-25"].tool_calls, 1);
    assert_eq!(state.daily_stats["2026-06-26"].errors, 1);
    assert_eq!(state.daily_stats["2026-06-26"].turns, 1);
}

// ── Behavior 5: active_min accumulation ──

#[test]
fn active_min_accumulation() {
    let mut state = PetState::initial("2026-06-25".to_string());

    let events = vec![Event {
        ts: None,
        event_type: "SessionStart".to_string(),
        tool: None,
        session: None,
    }];

    // 125 seconds = 2 min + 5 sec => floored to 2 active_min.
    aggregate(&mut state, &events, 125, "2026-06-25");

    assert_eq!(state.daily_stats["2026-06-25"].active_min, 2);
}

// ── Behavior 6: Birth date preserved ──

#[test]
fn birth_date_preserved() {
    let mut state = PetState::initial("2026-06-25".to_string());

    let event = |et: &str| Event {
        ts: None,
        event_type: et.to_string(),
        tool: None,
        session: None,
    };

    // Run aggregate many times across different days.
    aggregate(&mut state, &[event("SessionStart")], 0, "2026-06-25");
    aggregate(&mut state, &[event("PreToolUse")], 60, "2026-06-25");
    aggregate(&mut state, &[event("error")], 120, "2026-06-26");
    aggregate(&mut state, &[event("UserPromptSubmit")], 0, "2026-06-27");
    aggregate(&mut state, &[event("Stop")], 0, "2026-06-28");

    assert_eq!(
        state.pet.birth_date, "2026-06-25",
        "birth_date must never change after initial creation"
    );
}

// ── Behavior 7: PostToolUse and unknown events give 0 XP ──

#[test]
fn zero_xp_events() {
    let post_tool_use = Event {
        ts: None,
        event_type: "PostToolUse".to_string(),
        tool: Some("Bash".to_string()),
        session: None,
    };

    let unknown = Event {
        ts: None,
        event_type: "SomeUnmappedEvent".to_string(),
        tool: None,
        session: None,
    };

    assert_eq!(xp_for_event(&post_tool_use), 0, "PostToolUse must give 0 XP");
    assert_eq!(
        xp_for_event(&unknown),
        0,
        "unknown event types must give 0 XP"
    );
}

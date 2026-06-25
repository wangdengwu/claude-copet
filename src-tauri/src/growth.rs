use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::events::Event;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Stage {
    Egg,
    Juvenile,
    Adult,
    Elder,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DailyStats {
    pub sessions: u64,
    pub tool_calls: u64,
    pub turns: u64,
    pub errors: u64,
    pub active_min: u64,
}

impl DailyStats {
    pub fn zero() -> Self {
        DailyStats {
            sessions: 0,
            tool_calls: 0,
            turns: 0,
            errors: 0,
            active_min: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Pet {
    pub birth_date: String,
    pub level: u32,
    pub xp: u64,
    pub stage: Stage,
    pub unlocked: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Cursor {
    pub events_offset: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PetState {
    pub pet: Pet,
    pub daily_stats: BTreeMap<String, DailyStats>,
    pub cursor: Cursor,
}

impl PetState {
    /// First-run state: level-1 egg, today's birth_date, zero everything.
    pub fn initial(birth_date: String) -> Self {
        PetState {
            pet: Pet {
                birth_date,
                level: 1,
                xp: 0,
                stage: Stage::Egg,
                unlocked: Vec::new(),
            },
            daily_stats: BTreeMap::new(),
            cursor: Cursor { events_offset: 0 },
        }
    }
}

/// XP for each event type per the spec:
/// SessionStart=5, UserPromptSubmit=10, PreToolUse=15, Stop=10,
/// error=20, Notification=20, PostToolUse=0, unknown=0.
pub fn xp_for_event(event: &Event) -> u64 {
    match event.event_type.as_str() {
        "SessionStart" => 5,
        "UserPromptSubmit" => 10,
        "PreToolUse" => 15,
        "Stop" => 10,
        "error" => 20,
        "Notification" => 20,
        "PostToolUse" => 0,
        _ => 0,
    }
}

/// Level thresholds: 0→1, 100→2, 250→3, 500→4, 800→5, 1200→6,
/// 1700→7, 2300→8, 3000→9, 4000→10; then each +1000 XP = +1 level.
pub fn level_for_xp(xp: u64) -> u32 {
    if xp >= 4000 {
        return 10 + ((xp - 4000) / 1000) as u32;
    }
    if xp >= 3000 {
        return 9;
    }
    if xp >= 2300 {
        return 8;
    }
    if xp >= 1700 {
        return 7;
    }
    if xp >= 1200 {
        return 6;
    }
    if xp >= 800 {
        return 5;
    }
    if xp >= 500 {
        return 4;
    }
    if xp >= 250 {
        return 3;
    }
    if xp >= 100 {
        return 2;
    }
    1
}

/// Map a level to its evolution stage:
/// egg(1-2), juvenile(3-5), adult(6-9), elder(10+).
pub fn stage_for_level(level: u32) -> Stage {
    match level {
        1..=2 => Stage::Egg,
        3..=5 => Stage::Juvenile,
        6..=9 => Stage::Adult,
        _ => Stage::Elder,
    }
}

/// Core pure transition: adds XP per event, updates level/stage, and
/// accumulates daily stats for `today`. The `delta_active_s` is floored
/// to whole minutes for active_min.
pub fn aggregate(state: &mut PetState, events: &[Event], delta_active_s: u64, today: &str) {
    // Accrue XP from each event.
    let xp_gained: u64 = events.iter().map(xp_for_event).sum();
    state.pet.xp += xp_gained;

    // Update level and stage.
    state.pet.level = level_for_xp(state.pet.xp);
    state.pet.stage = stage_for_level(state.pet.level);

    // Update daily stats for today.
    let stats = state
        .daily_stats
        .entry(today.to_string())
        .or_insert_with(DailyStats::zero);

    for event in events {
        match event.event_type.as_str() {
            "SessionStart" => stats.sessions += 1,
            "UserPromptSubmit" => stats.turns += 1,
            "PreToolUse" => stats.tool_calls += 1,
            "error" | "Notification" => stats.errors += 1,
            _ => {}
        }
    }

    stats.active_min += delta_active_s / 60;
}

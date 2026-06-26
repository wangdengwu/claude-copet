//! Slice 4: needs-human flag transitions + current-activity label (pure).

use claude_copet_lib::events::{Event, Mood};
use claude_copet_lib::session::{activity_label, attention_step};

fn ev(event_type: &str, tool: Option<&str>) -> Event {
    Event {
        ts: None,
        event_type: event_type.to_string(),
        tool: tool.map(|s| s.to_string()),
        session: None,
        cwd: None,
        transcript_path: None,
    }
}

// ─────────────────────────── attention_step ──────────────────────────────────

#[test]
fn attention_set_and_cleared_across_a_sequence() {
    let mut flag = false;

    flag = attention_step(flag, &ev("Notification", None));
    assert!(flag, "Notification must set the needs-human flag");

    flag = attention_step(flag, &ev("PreToolUse", Some("Bash")));
    assert!(!flag, "PreToolUse (work resumed) must clear the flag");

    flag = attention_step(flag, &ev("Stop", None));
    assert!(flag, "Stop (your turn) must set the flag");

    flag = attention_step(flag, &ev("UserPromptSubmit", None));
    assert!(!flag, "UserPromptSubmit (you replied) must clear the flag");
}

#[test]
fn unrelated_event_leaves_flag_unchanged() {
    assert!(
        attention_step(true, &ev("PostToolUse", Some("Bash"))),
        "set flag stays set"
    );
    assert!(
        !attention_step(false, &ev("PostToolUse", Some("Bash"))),
        "clear flag stays clear"
    );
    assert!(
        attention_step(true, &ev("SessionStart", None)),
        "SessionStart doesn't clear"
    );
}

// ─────────────────────────── activity_label ──────────────────────────────────

#[test]
fn running_tool_label() {
    assert_eq!(activity_label(Mood::Work, Some("Bash")), "Running Bash");
    assert_eq!(activity_label(Mood::Work, Some("Edit")), "Running Edit");
}

#[test]
fn idle_or_quiet_moods_are_idle() {
    assert_eq!(activity_label(Mood::Idle, None), "Idle");
    assert_eq!(
        activity_label(Mood::Idle, Some("Bash")),
        "Idle",
        "idle wins over a stale tool"
    );
    assert_eq!(activity_label(Mood::Sleep, Some("Bash")), "Idle");
}

#[test]
fn working_without_a_known_tool_is_non_empty() {
    let label = activity_label(Mood::Work, None);
    assert!(
        !label.is_empty(),
        "an active mood must still produce a label"
    );
}

// Integration tests for the `events` module (Task 2, seam 3 + mood mapping).
// These tests import the public API that does NOT exist yet → compile fails (RED state).
// Do NOT add any stub implementation here.

use claude_copet_lib::events::{parse, mood_for_event, Mood, Event};

// ── Behavior 1: Valid JSONL lines parse to typed Event structs; offset == bytes.len() ──

#[test]
fn valid_jsonl_parses_to_events_and_advances_offset() {
    // Two complete newline-terminated lines.
    let line1: &[u8] = b"{\"type\":\"SessionStart\",\"ts\":\"2024-01-01T00:00:00Z\",\"session\":\"s1\"}\n";
    let line2: &[u8] = b"{\"type\":\"PreToolUse\",\"tool\":\"Bash\",\"session\":\"s1\"}\n";

    let mut buf = Vec::new();
    buf.extend_from_slice(line1);
    buf.extend_from_slice(line2);

    let expected_offset = line1.len() + line2.len(); // == buf.len()

    let (events, offset) = parse(&buf, 0);

    assert_eq!(events.len(), 2, "expected 2 events from 2 valid lines");

    assert_eq!(events[0].event_type, "SessionStart");
    assert_eq!(events[0].ts.as_deref(), Some("2024-01-01T00:00:00Z"));
    assert_eq!(events[0].session.as_deref(), Some("s1"));
    assert_eq!(events[0].tool, None);

    assert_eq!(events[1].event_type, "PreToolUse");
    assert_eq!(events[1].tool.as_deref(), Some("Bash"));
    assert_eq!(events[1].session.as_deref(), Some("s1"));

    assert_eq!(offset, expected_offset, "offset must equal buf.len() after consuming all complete lines");
}

// ── Behavior 2: Malformed lines are skipped (not fatal); offset still advances past them ──

#[test]
fn malformed_lines_skipped_not_fatal_offset_advances() {
    // valid \n / not-json \n / valid \n
    let line_valid_1: &[u8] = b"{\"type\":\"SessionStart\",\"session\":\"s2\"}\n";
    let line_malformed: &[u8] = b"not json\n";
    let line_valid_2: &[u8]   = b"{\"type\":\"Stop\",\"session\":\"s2\"}\n";

    let mut buf = Vec::new();
    buf.extend_from_slice(line_valid_1);
    buf.extend_from_slice(line_malformed);
    buf.extend_from_slice(line_valid_2);

    let expected_offset = line_valid_1.len() + line_malformed.len() + line_valid_2.len();

    let (events, offset) = parse(&buf, 0);

    assert_eq!(events.len(), 2, "malformed line must be skipped, yielding 2 valid events");
    assert_eq!(events[0].event_type, "SessionStart");
    assert_eq!(events[1].event_type, "Stop");
    assert_eq!(offset, expected_offset, "offset must advance past malformed line too");
}

// ── Behavior 3: Idempotency from saved offset; unterminated trailing line not consumed ──

#[test]
fn resume_from_offset_is_idempotent_and_unterminated_line_held() {
    // One complete line, then a partial line with no trailing '\n'.
    let complete_line: &[u8]     = b"{\"type\":\"UserPromptSubmit\",\"session\":\"s3\"}\n";
    let unterminated_line: &[u8] = b"{\"type\":\"Stop\""; // no '\n' — intentionally truncated

    let mut buf = Vec::new();
    buf.extend_from_slice(complete_line);
    buf.extend_from_slice(unterminated_line);

    // First parse from offset 0 — should emit the complete line, hold the unterminated one.
    let (events1, offset1) = parse(&buf, 0);

    assert_eq!(events1.len(), 1, "only the complete line should be emitted on first parse");
    assert_eq!(events1[0].event_type, "UserPromptSubmit");
    // The unterminated line is not consumed: offset1 must point at the start of it.
    assert_eq!(offset1, complete_line.len(),
        "offset after first parse must stop before the unterminated trailing line");
    assert!(offset1 < buf.len(), "unterminated line must NOT be consumed");

    // Second parse from the returned offset — same buffer, same state — must yield nothing new.
    let (events2, offset2) = parse(&buf, offset1);

    assert_eq!(events2.len(), 0, "re-parsing from saved offset must yield no new events");
    assert_eq!(offset2, offset1, "offset must be unchanged when no new complete line is available");
}

// ── Behavior 4: mood_for_event mapping (all 5 authoritative mappings + None cases) ──

#[test]
fn mood_for_event_mapping_matches_spec() {
    let make = |event_type: &str, tool: Option<&str>| Event {
        ts: None,
        event_type: event_type.to_string(),
        tool: tool.map(|s| s.to_string()),
        session: None,
    };

    // SessionStart → Wake
    assert_eq!(mood_for_event(&make("SessionStart", None)), Some(Mood::Wake));

    // UserPromptSubmit → Listen
    assert_eq!(mood_for_event(&make("UserPromptSubmit", None)), Some(Mood::Listen));

    // PreToolUse → Work
    assert_eq!(mood_for_event(&make("PreToolUse", Some("Bash"))), Some(Mood::Work));

    // error → Panic
    assert_eq!(mood_for_event(&make("error", None)), Some(Mood::Panic));

    // Notification → Panic  (Claude Code attention/error signal)
    assert_eq!(mood_for_event(&make("Notification", None)), Some(Mood::Panic));

    // Stop → Happy
    assert_eq!(mood_for_event(&make("Stop", None)), Some(Mood::Happy));

    // PostToolUse → None  (explicitly not mood-changing)
    assert_eq!(mood_for_event(&make("PostToolUse", Some("Bash"))), None);

    // Unknown type → None
    assert_eq!(mood_for_event(&make("SomeUnknownEvent", None)), None);
}

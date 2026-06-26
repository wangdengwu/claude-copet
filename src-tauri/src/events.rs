//! Event-log perception: parse appended JSONL lines into typed events and map
//! each event to a mood. Pure functions only — no file IO, threads, or tauri here
//! (the watcher in `lib.rs` owns those). This is seam 3 + the direct mood mapping.

use serde::{Deserialize, Serialize};

/// One parsed line of the event log. Log line shape: `{ ts, type, tool?, session, ... }`.
/// `type` is a Rust keyword, so it is renamed. Only `type` is required; a line missing
/// it (or that is not valid JSON) is treated as malformed and skipped by [`parse`].
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Event {
    #[serde(default)]
    pub ts: Option<String>,
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(default)]
    pub tool: Option<String>,
    #[serde(default)]
    pub session: Option<String>,
    /// The session's working directory (cwd basename becomes the HUD label).
    #[serde(default)]
    pub cwd: Option<String>,
    /// Path to the session's transcript JSONL (read for context % + model).
    #[serde(default)]
    pub transcript_path: Option<String>,
}

/// Mood the frontend can show. Serializes to the lowercase string the TS `Mood`
/// type uses, so emitting it to the frontend yields e.g. `"work"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Mood {
    Wake,
    Listen,
    Work,
    Panic,
    Happy,
    Idle,
    Sleep,
    Tired,
}

/// Parse complete newline-terminated lines from `bytes` starting at byte index
/// `from_offset`. Returns the parsed events (malformed / JSON-invalid lines are
/// skipped, never fatal) and the new absolute byte offset, advanced past every
/// COMPLETE line consumed (including skipped malformed ones).
///
/// A trailing line with no final `\n` is NOT consumed: the returned offset points
/// at its start, so it is re-read once fully written. Idempotent — calling again
/// with the returned offset over the same bytes yields no already-consumed events.
pub fn parse(bytes: &[u8], from_offset: usize) -> (Vec<Event>, usize) {
    let mut events = Vec::new();
    let mut offset = from_offset.min(bytes.len());

    let mut i = offset;
    while i < bytes.len() {
        if bytes[i] == b'\n' {
            let line = &bytes[offset..i]; // line content, excluding the '\n'
            if let Ok(event) = serde_json::from_slice::<Event>(line) {
                events.push(event);
            }
            // Advance past this complete line regardless of parse outcome, so a
            // malformed line is consumed once and never blocks the stream.
            offset = i + 1;
        }
        i += 1;
    }

    (events, offset)
}

/// Direct event→mood mapping for this slice (no decay/fallback — that is slice 3).
/// Returns `None` for events that should not change the displayed mood
/// (`PostToolUse`, unknown types).
pub fn mood_for_event(event: &Event) -> Option<Mood> {
    match event.event_type.as_str() {
        "SessionStart" => Some(Mood::Wake),
        "UserPromptSubmit" => Some(Mood::Listen),
        "PreToolUse" => Some(Mood::Work),
        // Claude Code has no dedicated error hook; its Notification hook is the
        // concrete attention/error signal, so both map to panic.
        "error" | "Notification" => Some(Mood::Panic),
        "Stop" => Some(Mood::Happy),
        _ => None,
    }
}

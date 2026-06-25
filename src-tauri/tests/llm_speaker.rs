//! Failing tests for seams 1–4 of slice 5 (LLM speaker).
//!
//! Required public API (implementer must expose these):
//!
//!   // in claude_copet_lib::speaker
//!   pub trait LlmClient: Send + Sync {
//!       fn complete(&self, system: &str, prompt: &str) -> Result<String, ()>;
//!   }
//!
//!   pub struct LlmSpeaker<C: LlmClient> { /* ... */ }
//!   impl<C: LlmClient> LlmSpeaker<C> {
//!       pub fn new(client: C, cooldown_secs: u64, fallback_seed: u64) -> Self;
//!       pub fn with_enabled(client: C, enabled: bool, cooldown_secs: u64, fallback_seed: u64) -> Self;
//!   }
//!   impl<C: LlmClient> Speaker for LlmSpeaker<C> { ... }
//!
//!   pub fn build_summary(event: &Event, state: &PetState, mood: Mood) -> String;
//!
//!   // in claude_copet_lib::speaker (or a sub-module re-exported from it)
//!   pub fn is_special_moment(
//!       event: &Event,
//!       state: &PetState,
//!       mood: Mood,
//!       prev_mood: Option<Mood>,
//!       stage_changed: bool,
//!   ) -> bool;
//!
//!   // SpeakContext gains optional event+state so LlmSpeaker can use them:
//!   pub struct SpeakContext {
//!       pub mood: Mood,
//!       pub event: Option<Event>,
//!       pub state: Option<PetState>,
//!       pub prev_mood: Option<Mood>,
//!       pub stage_changed: bool,
//!   }

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use claude_copet_lib::events::{Event, Mood};
use claude_copet_lib::growth::{DailyStats, Pet, PetState, Stage, Cursor};
use claude_copet_lib::speaker::{
    LlmClient, LlmSpeaker, SpeakContext, Speaker, build_summary, is_special_moment,
};

// ─────────────────────────── helpers ────────────────────────────────────────

fn make_event(event_type: &str) -> Event {
    Event {
        ts: None,
        event_type: event_type.to_string(),
        tool: None,
        session: None,
    }
}

fn make_event_with_sentinels() -> Event {
    Event {
        ts: None,
        event_type: "PreToolUse".to_string(),
        // Distinctive sentinels that MUST NOT appear in the summary.
        tool: Some("SENTINEL_TOOL_XYZ_SECRET".to_string()),
        session: Some("SENTINEL_SESSION_ABC_SECRET".to_string()),
    }
}

fn make_state(level: u32, stage: Stage) -> PetState {
    PetState {
        pet: Pet {
            birth_date: "2026-01-01".to_string(),
            level,
            xp: 0,
            stage,
            unlocked: vec![],
        },
        daily_stats: {
            let mut m = std::collections::BTreeMap::new();
            m.insert("2026-01-01".to_string(), DailyStats {
                sessions: 3,
                tool_calls: 10,
                turns: 5,
                errors: 1,
                active_min: 20,
            });
            m
        },
        cursor: Cursor { events_offset: 0 },
    }
}

// ─────────────────────────── mock LlmClient ─────────────────────────────────

/// A controllable mock. `calls` counts every invocation of `complete`.
struct MockClient {
    calls: Arc<AtomicUsize>,
    /// None → return Err(()); Some(s) → return Ok(s).
    response: Option<String>,
}

impl MockClient {
    fn succeeds(response: &str) -> (Self, Arc<AtomicUsize>) {
        let calls = Arc::new(AtomicUsize::new(0));
        let client = MockClient {
            calls: Arc::clone(&calls),
            response: Some(response.to_string()),
        };
        (client, calls)
    }

    fn fails() -> (Self, Arc<AtomicUsize>) {
        let calls = Arc::new(AtomicUsize::new(0));
        let client = MockClient { calls: Arc::clone(&calls), response: None };
        (client, calls)
    }
}

impl LlmClient for MockClient {
    fn complete(&self, _system: &str, _prompt: &str) -> Result<String, ()> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        match &self.response {
            Some(s) => Ok(s.clone()),
            None => Err(()),
        }
    }
}

// ─────────────────────────── seam 1 — de-sensitized context builder ─────────

/// The summary MUST contain the event type.
#[test]
fn summary_contains_event_type() {
    let event = make_event("SessionStart");
    let state = make_state(3, Stage::Juvenile);
    let summary = build_summary(&event, &state, Mood::Wake);
    assert!(
        summary.contains("SessionStart"),
        "summary must contain the event type; got: {summary:?}"
    );
}

/// The summary MUST contain the pet level.
#[test]
fn summary_contains_level() {
    let event = make_event("Stop");
    let state = make_state(5, Stage::Juvenile);
    let summary = build_summary(&event, &state, Mood::Happy);
    let has_level = summary.contains("5") || summary.to_lowercase().contains("level");
    assert!(
        has_level,
        "summary must reference the pet level; got: {summary:?}"
    );
}

/// The summary MUST reference the stage.
#[test]
fn summary_contains_stage() {
    let event = make_event("Stop");
    let state = make_state(7, Stage::Adult);
    let summary = build_summary(&event, &state, Mood::Happy);
    // Accept either the enum debug name or the serde lowercase form.
    let lower = summary.to_lowercase();
    assert!(
        lower.contains("adult"),
        "summary must contain the stage; got: {summary:?}"
    );
}

/// The session sentinel MUST NOT appear in the summary (privacy).
#[test]
fn summary_excludes_session_sentinel() {
    let event = make_event_with_sentinels();
    let state = make_state(2, Stage::Egg);
    let summary = build_summary(&event, &state, Mood::Work);
    assert!(
        !summary.contains("SENTINEL_SESSION_ABC_SECRET"),
        "summary must NOT contain the raw session id; got: {summary:?}"
    );
}

/// The tool sentinel MUST NOT appear in the summary (privacy).
#[test]
fn summary_excludes_tool_sentinel() {
    let event = make_event_with_sentinels();
    let state = make_state(2, Stage::Egg);
    let summary = build_summary(&event, &state, Mood::Work);
    assert!(
        !summary.contains("SENTINEL_TOOL_XYZ_SECRET"),
        "summary must NOT contain the raw tool name; got: {summary:?}"
    );
}

/// The summary is deterministic (same inputs → same output).
#[test]
fn summary_is_deterministic() {
    let event = make_event("UserPromptSubmit");
    let state = make_state(4, Stage::Juvenile);
    let a = build_summary(&event, &state, Mood::Listen);
    let b = build_summary(&event, &state, Mood::Listen);
    assert_eq!(a, b, "build_summary must be deterministic");
}

// ─────────────────────────── seam 2 — cooldown ──────────────────────────────

/// Two consecutive special-moment speak() calls within the cooldown window →
/// the client must be called EXACTLY ONCE (second call returns template fallback
/// without hitting the client again).
#[test]
fn cooldown_prevents_second_client_call() {
    let (client, calls) = MockClient::succeeds("LLM line");
    // cooldown_secs = 3600 — very long; second call is always inside the window.
    let mut speaker = LlmSpeaker::new(client, 3600, 42);

    let state = make_state(4, Stage::Juvenile);
    let ctx = SpeakContext {
        mood: Mood::Happy,
        event: Some(make_event("Stop")),
        state: Some(state.clone()),
        prev_mood: Some(Mood::Panic),
        stage_changed: false,
    };

    let first = speaker.speak(&ctx);
    assert!(first.is_some(), "first speak must return Some");
    assert_eq!(calls.load(Ordering::SeqCst), 1, "first speak should call the client once");

    // Second call: still within cooldown → client must NOT be called again.
    let second = speaker.speak(&ctx);
    assert!(second.is_some(), "second speak must return Some (template fallback)");
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "second speak within cooldown must NOT call the client again"
    );
}

// ─────────────────────────── seam 3 — LlmSpeaker via Speaker trait ──────────

/// Enabled + client succeeds + special moment → returns the LLM line.
#[test]
fn enabled_client_ok_returns_llm_line() {
    let (client, _calls) = MockClient::succeeds("You leveled up!");
    let mut speaker = LlmSpeaker::new(client, 0, 42); // cooldown_secs=0 → always ready

    let state = make_state(4, Stage::Juvenile);
    let ctx = SpeakContext {
        mood: Mood::Happy,
        event: Some(make_event("Stop")),
        state: Some(state),
        prev_mood: Some(Mood::Panic), // happy-after-panic → special
        stage_changed: false,
    };

    let line = speaker.speak(&ctx).expect("should return Some");
    assert_eq!(line, "You leveled up!", "expected the LLM-provided line");
}

/// Enabled + client errors → falls back to a non-empty template line (never silent).
#[test]
fn client_error_falls_back_to_template() {
    let (client, calls) = MockClient::fails();
    let mut speaker = LlmSpeaker::new(client, 0, 42);

    let state = make_state(4, Stage::Juvenile);
    let ctx = SpeakContext {
        mood: Mood::Happy,
        event: Some(make_event("Stop")),
        state: Some(state),
        prev_mood: Some(Mood::Panic),
        stage_changed: false,
    };

    let line = speaker.speak(&ctx).expect("must return Some even on LLM error");
    assert!(!line.is_empty(), "template fallback must not be empty");
    // The client was attempted exactly once before falling back.
    assert_eq!(calls.load(Ordering::SeqCst), 1, "client should have been tried once");
}

/// LlmSpeaker disabled → zero client calls, still returns a template line.
#[test]
fn disabled_makes_zero_client_calls() {
    let (client, calls) = MockClient::succeeds("should never appear");
    let mut speaker = LlmSpeaker::with_enabled(client, false, 0, 42);

    let state = make_state(4, Stage::Juvenile);
    let ctx = SpeakContext {
        mood: Mood::Happy,
        event: Some(make_event("Stop")),
        state: Some(state),
        prev_mood: Some(Mood::Panic),
        stage_changed: false,
    };

    let line = speaker.speak(&ctx);
    assert!(line.is_some(), "disabled speaker must still return a template line");
    assert_eq!(
        calls.load(Ordering::SeqCst),
        0,
        "disabled speaker must make zero client calls"
    );
}

/// Non-special moment (ordinary Work event, no stage change, no panic→happy) →
/// client is NOT called even when enabled; template line is returned.
#[test]
fn non_special_moment_uses_template_only() {
    let (client, calls) = MockClient::succeeds("should not appear");
    let mut speaker = LlmSpeaker::new(client, 0, 42);

    let state = make_state(2, Stage::Egg);
    let ctx = SpeakContext {
        mood: Mood::Work,
        event: Some(make_event("PreToolUse")),
        state: Some(state),
        prev_mood: Some(Mood::Listen), // ordinary mood transition
        stage_changed: false,
    };

    let line = speaker.speak(&ctx).expect("should return Some");
    assert!(!line.is_empty());
    assert_eq!(
        calls.load(Ordering::SeqCst),
        0,
        "non-special moment must not call the LLM client"
    );
}

// ─────────────────────────── seam 4 — special-moment policy ─────────────────

/// A stage change qualifies as a special moment.
#[test]
fn stage_change_is_special_moment() {
    let event = make_event("Stop");
    let state = make_state(3, Stage::Juvenile); // just evolved
    let is_special = is_special_moment(&event, &state, Mood::Happy, Some(Mood::Happy), true);
    assert!(is_special, "stage change must qualify as a special moment");
}

/// Happy entered right after Panic (hard-won success) qualifies.
#[test]
fn happy_after_panic_is_special_moment() {
    let event = make_event("Stop");
    let state = make_state(2, Stage::Egg);
    let is_special = is_special_moment(&event, &state, Mood::Happy, Some(Mood::Panic), false);
    assert!(is_special, "Happy-after-Panic must qualify as a special moment");
}

/// An ordinary Work event with no stage change and no special mood transition
/// does NOT qualify.
#[test]
fn ordinary_event_is_not_special_moment() {
    let event = make_event("PreToolUse");
    let state = make_state(2, Stage::Egg);
    let is_special = is_special_moment(&event, &state, Mood::Work, Some(Mood::Listen), false);
    assert!(!is_special, "ordinary PreToolUse must NOT qualify as a special moment");
}

/// Idle→Happy is not special on its own (not from Panic).
#[test]
fn happy_after_idle_is_not_special_moment() {
    let event = make_event("Stop");
    let state = make_state(2, Stage::Egg);
    let is_special = is_special_moment(&event, &state, Mood::Happy, Some(Mood::Idle), false);
    assert!(!is_special, "Happy-after-Idle must NOT qualify as a special moment");
}

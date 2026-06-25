//! The pet's voice (seam 4). `TemplateSpeaker` picks a handwritten line for a mood
//! using a seeded RNG, so selection is deterministic given the seed. The `Speaker`
//! trait is the extension point slice 5 plugs `LlmSpeaker` into.

use std::time::{Duration, Instant};

use crate::events::{Event, Mood};
use crate::growth::PetState;

/// What the speaker is reacting to. Enriched in slice 5 with de-sensitised
/// event + state fields used by `LlmSpeaker`.
pub struct SpeakContext {
    pub mood: Mood,
    /// The triggering event, if available (slice 5+).
    pub event: Option<Event>,
    /// Current pet state snapshot (slice 5+).
    pub state: Option<PetState>,
    /// Mood immediately before the current one (slice 5+).
    pub prev_mood: Option<Mood>,
    /// True if a stage change just happened this tick (slice 5+).
    pub stage_changed: bool,
}

// Allow `SpeakContext { mood, ..Default::default() }` so old tests that only
// set `mood` can still compile without modification.
impl Default for SpeakContext {
    fn default() -> Self {
        SpeakContext {
            mood: Mood::Idle,
            event: None,
            state: None,
            prev_mood: None,
            stage_changed: false,
        }
    }
}

pub trait Speaker {
    /// Return a line to show, or `None` to stay silent.
    fn speak(&mut self, ctx: &SpeakContext) -> Option<String>;
}

/// Handwritten lines per mood — ASCII only (the pixel bubble has no emoji font).
/// Every mood has at least one line.
fn lines(mood: Mood) -> &'static [&'static str] {
    match mood {
        Mood::Wake => &["*yawn*... morning!", "I'm awake!", "Ready when you are."],
        Mood::Listen => &["Hmm, let's see...", "I'm listening.", "What's the plan?"],
        Mood::Work => &["On it!", "Crunching...", "Working hard!", "beep boop"],
        Mood::Panic => &["Uh oh!", "Something broke!", "Eep, an error!"],
        Mood::Happy => &["Done!", "Nice work!", "We did it!"],
        Mood::Idle => &["...", "*hums*", "Just chillin'."],
        Mood::Sleep => &["Zzz...", "*snore*", "...zzz"],
        Mood::Tired => &["So tired...", "*big yawn*", "Need a break..."],
    }
}

/// Deterministic line picker: same seed + same call sequence ⇒ identical lines.
pub struct TemplateSpeaker {
    rng: u64,
}

impl TemplateSpeaker {
    pub fn new(seed: u64) -> Self {
        // Avoid a zero state (xorshift fixed point); any nonzero constant works.
        Self {
            rng: seed ^ 0x9E37_79B9_7F4A_7C15,
        }
    }

    /// xorshift64* — small, dependency-free, good enough for picking a line.
    fn next(&mut self) -> u64 {
        let mut x = self.rng;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.rng = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
}

impl Speaker for TemplateSpeaker {
    fn speak(&mut self, ctx: &SpeakContext) -> Option<String> {
        let choices = lines(ctx.mood);
        if choices.is_empty() {
            return None;
        }
        let idx = (self.next() % choices.len() as u64) as usize;
        Some(choices[idx].to_string())
    }
}

// ─────────────────────────── LLM seam ────────────────────────────────────────

/// Network seam for the LLM. Tests inject a mock; production uses
/// `AnthropicClient`.
pub trait LlmClient: Send + Sync {
    fn complete(&self, system: &str, prompt: &str) -> Result<String, ()>;
}

/// `LlmSpeaker` wraps an `LlmClient` with a cooldown and a `TemplateSpeaker`
/// fallback. Special moments get an LLM line; everything else uses the template.
pub struct LlmSpeaker<C: LlmClient> {
    client: C,
    enabled: bool,
    cooldown: Duration,
    last_llm_call: Option<Instant>,
    fallback: TemplateSpeaker,
}

impl<C: LlmClient> LlmSpeaker<C> {
    pub fn new(client: C, cooldown_secs: u64, fallback_seed: u64) -> Self {
        Self::with_enabled(client, true, cooldown_secs, fallback_seed)
    }

    pub fn with_enabled(client: C, enabled: bool, cooldown_secs: u64, fallback_seed: u64) -> Self {
        LlmSpeaker {
            client,
            enabled,
            cooldown: Duration::from_secs(cooldown_secs),
            last_llm_call: None,
            fallback: TemplateSpeaker::new(fallback_seed),
        }
    }

    /// True when the cooldown has elapsed (or has never been set).
    fn cooldown_elapsed(&self) -> bool {
        match self.last_llm_call {
            None => true,
            Some(t) => t.elapsed() >= self.cooldown,
        }
    }
}

impl<C: LlmClient> Speaker for LlmSpeaker<C> {
    fn speak(&mut self, ctx: &SpeakContext) -> Option<String> {
        // Determine whether this is a special moment that warrants an LLM call.
        let is_special = ctx
            .event
            .as_ref()
            .zip(ctx.state.as_ref())
            .map(|(ev, st)| is_special_moment(ev, st, ctx.mood, ctx.prev_mood, ctx.stage_changed))
            .unwrap_or(false);

        // Use LLM only when: enabled + special moment + cooldown elapsed.
        if self.enabled && is_special && self.cooldown_elapsed() {
            let summary = ctx
                .event
                .as_ref()
                .zip(ctx.state.as_ref())
                .map(|(ev, st)| build_summary(ev, st, ctx.mood))
                .unwrap_or_default();

            let system = "You are a small pixel desktop pet companion for a developer. \
                Respond with exactly one short, friendly, encouraging line (max 12 words). \
                No hashtags, no special characters.";

            match self.client.complete(system, &summary) {
                Ok(text) if !text.is_empty() => {
                    self.last_llm_call = Some(Instant::now());
                    return Some(text);
                }
                _ => {
                    // Client error or empty — fall through to template.
                }
            }
        }

        // Always produce a fallback template line so the pet is never silent.
        self.fallback.speak(ctx)
    }
}

// ─────────────────────────── context builder ─────────────────────────────────

/// Build a de-sensitised summary safe to send to the LLM.
/// PRIVACY: includes ONLY event type, mood, pet level, pet stage, and today's
/// aggregate DailyStats counts. MUST NOT include `event.session` or `event.tool`.
pub fn build_summary(event: &Event, state: &PetState, mood: Mood) -> String {
    // Pick today's stats if available (the most recent date key), else zeros.
    let today_stats = state
        .daily_stats
        .values()
        .last() // BTreeMap is sorted; last = most recent date.
        .cloned()
        .unwrap_or_else(crate::growth::DailyStats::zero);

    format!(
        "event={event_type} mood={mood:?} level={level} stage={stage:?} \
         sessions={sessions} tool_calls={tool_calls} turns={turns} errors={errors}",
        event_type = event.event_type,
        mood = mood,
        level = state.pet.level,
        stage = state.pet.stage,
        sessions = today_stats.sessions,
        tool_calls = today_stats.tool_calls,
        turns = today_stats.turns,
        errors = today_stats.errors,
    )
}

// ─────────────────────────── special-moment policy ───────────────────────────

/// Returns `true` when the current moment is worth an LLM-generated line.
/// Qualifies on:
/// - A stage/evolution change (`stage_changed`).
/// - `Happy` entered right after `Panic` (hard-won success).
/// - Reunion: first event after `SessionStart` (not implemented yet; kept false
///   here until slice 6 wires idle-gap detection).
///
/// Ordinary events return `false`.
pub fn is_special_moment(
    _event: &Event,
    _state: &PetState,
    mood: Mood,
    prev_mood: Option<Mood>,
    stage_changed: bool,
) -> bool {
    if stage_changed {
        return true;
    }
    if mood == Mood::Happy && prev_mood == Some(Mood::Panic) {
        return true;
    }
    false
}

// ─────────────────────────── real HTTP client ────────────────────────────────

/// Production `LlmClient` that calls the Anthropic Messages API (blocking).
/// Not exercised by tests (no network); kept small and non-panicking.
pub struct AnthropicClient {
    pub api_key: String,
    pub model: String,
}

impl LlmClient for AnthropicClient {
    fn complete(&self, system: &str, prompt: &str) -> Result<String, ()> {
        let body = serde_json::json!({
            "model": self.model,
            "max_tokens": 60,
            "system": system,
            "messages": [{"role": "user", "content": prompt}]
        });

        let response = ureq::post("https://api.anthropic.com/v1/messages")
            .set("x-api-key", &self.api_key)
            .set("anthropic-version", "2023-06-01")
            .set("content-type", "application/json")
            .timeout(std::time::Duration::from_secs(10))
            .send_json(&body)
            .map_err(|_| ())?;

        // Parse the response: look for the first content block of type "text".
        let json: serde_json::Value = response.into_json().map_err(|_| ())?;
        let text = json
            .get("content")
            .and_then(|c| c.as_array())
            .and_then(|arr| {
                arr.iter().find(|block| {
                    block.get("type").and_then(|t| t.as_str()) == Some("text")
                })
            })
            .and_then(|block| block.get("text"))
            .and_then(|t| t.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .ok_or(())?;

        Ok(text)
    }
}

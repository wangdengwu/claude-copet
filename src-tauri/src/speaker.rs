//! The pet's voice (seam 4). `TemplateSpeaker` picks a handwritten line for a mood
//! using a seeded RNG, so selection is deterministic given the seed. The `Speaker`
//! trait is the extension point slice 5 plugs `LlmSpeaker` into.

use crate::events::Mood;

/// What the speaker is reacting to. Kept minimal for this slice (mood only);
/// slice 5 enriches it with a de-sensitized event summary.
pub struct SpeakContext {
    pub mood: Mood,
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

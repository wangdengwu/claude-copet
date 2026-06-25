use claude_copet_lib::events::Mood;
use claude_copet_lib::speaker::{SpeakContext, Speaker, TemplateSpeaker};

const ALL_MOODS: [Mood; 8] = [
    Mood::Wake,
    Mood::Listen,
    Mood::Work,
    Mood::Panic,
    Mood::Happy,
    Mood::Idle,
    Mood::Sleep,
    Mood::Tired,
];

#[test]
fn every_mood_returns_some_nonempty() {
    let mut speaker = TemplateSpeaker::new(42);
    for mood in ALL_MOODS {
        let ctx = SpeakContext { mood };
        let result = speaker.speak(&ctx);
        assert!(result.is_some(), "speak() returned None for mood {:?}", mood);
        let line = result.unwrap();
        assert!(!line.is_empty(), "speak() returned empty string for mood {:?}", mood);
    }
}

#[test]
fn deterministic_under_fixed_seed() {
    const SEED: u64 = 12345;

    // Drive two speakers with the same seed through the same call sequence.
    let mut speaker_a = TemplateSpeaker::new(SEED);
    let mut speaker_b = TemplateSpeaker::new(SEED);

    let moods_sequence = [
        Mood::Wake,
        Mood::Work,
        Mood::Panic,
        Mood::Happy,
        Mood::Listen,
        Mood::Idle,
        Mood::Sleep,
        Mood::Tired,
        Mood::Work,
        Mood::Work,
    ];

    let outputs_a: Vec<String> = moods_sequence
        .iter()
        .filter_map(|&mood| speaker_a.speak(&SpeakContext { mood }))
        .collect();

    let outputs_b: Vec<String> = moods_sequence
        .iter()
        .filter_map(|&mood| speaker_b.speak(&SpeakContext { mood }))
        .collect();

    assert_eq!(
        outputs_a, outputs_b,
        "same seed must produce identical output sequences"
    );

    // Verify we got output for all moods
    assert_eq!(outputs_a.len(), moods_sequence.len());
}

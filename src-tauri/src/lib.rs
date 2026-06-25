// The pet shell. Window chrome/transparency/always-on-top live in
// tauri.conf.json so later slices can extend behavior without touching this.

pub mod events;
pub mod mood;
pub mod speaker;

use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use speaker::Speaker;
use tauri::Emitter;

/// The event-log location, shared by the Claude Code hooks and this watcher.
/// `$HOME/.claude-copet/events.jsonl` (`%USERPROFILE%` on Windows).
fn event_log_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
    Some(PathBuf::from(home).join(".claude-copet").join("events.jsonl"))
}

/// Emit the new mood to the frontend and, on mood entry, a spoken template line.
fn announce(app: &tauri::AppHandle, speaker: &mut speaker::TemplateSpeaker, mood: events::Mood) {
    let _ = app.emit("mood", mood);
    if let Some(line) = speaker.speak(&speaker::SpeakContext { mood }) {
        let _ = app.emit("speech", line);
    }
}

/// Drive the mood state machine from the event log. New events preempt the mood
/// and reset its decay timer; quiet polls are pure time ticks that let the mood
/// decay back to idle and then sleep. Mood is ephemeral — we always start at idle,
/// reading the log from its current end so pre-existing events don't replay.
fn watch_event_log(app: tauri::AppHandle) {
    let Some(path) = event_log_path() else {
        return;
    };

    let mut offset = fs::metadata(&path).map(|m| m.len() as usize).unwrap_or(0);
    let mut state = mood::MoodState::initial();
    // Seed line selection from the wall clock for run-to-run variety. The constant
    // fallback only applies if the clock predates the Unix epoch (not a real case);
    // speech would then be deterministic, which is harmless for a pet.
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0xC0FF_EE00);
    let mut speaker = speaker::TemplateSpeaker::new(seed);
    let mut last = Instant::now();

    loop {
        std::thread::sleep(Duration::from_millis(250));
        let now = Instant::now();
        let delta = now.duration_since(last);
        last = now;

        let mut new_events: Vec<events::Event> = Vec::new();
        if let Ok(bytes) = fs::read(&path) {
            if bytes.len() < offset {
                offset = 0; // file truncated/rotated — restart from the beginning
            }
            if bytes.len() > offset {
                let (evs, new_offset) = events::parse(&bytes, offset);
                offset = new_offset;
                new_events = evs;
            }
        }

        if new_events.is_empty() {
            // No events this tick: advance time so the current mood can decay.
            let (next, signals) = mood::step(&state, None, delta);
            state = next;
            if signals.mood_changed {
                announce(&app, &mut speaker, state.mood);
            }
        } else {
            // Events in this batch are treated as simultaneous: the elapsed delta
            // applies to the first, the rest advance time by zero.
            let mut first = true;
            for event in &new_events {
                let d = if first { delta } else { Duration::ZERO };
                first = false;
                let (next, signals) = mood::step(&state, Some(event), d);
                state = next;
                if signals.mood_changed {
                    announce(&app, &mut speaker, state.mood);
                }
            }
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let handle = app.handle().clone();
            std::thread::spawn(move || watch_event_log(handle));
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

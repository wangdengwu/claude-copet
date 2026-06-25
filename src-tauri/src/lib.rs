// The pet shell. Window chrome/transparency/always-on-top live in
// tauri.conf.json so later slices can extend behavior without touching this.

pub mod events;

use std::path::PathBuf;
use std::time::Duration;
use tauri::Emitter;

/// The event-log location, shared by the Claude Code hooks and this watcher.
/// `$HOME/.claude-copet/events.jsonl` (`%USERPROFILE%` on Windows).
fn event_log_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
    Some(PathBuf::from(home).join(".claude-copet").join("events.jsonl"))
}

/// Poll the append-only event log and emit the mood for each new event. Mood is
/// ephemeral/live this slice, so we start at the file's current end (EOF) and only
/// react to events appended while the pet is running — history is left for slice 4
/// (XP) to consume from offset 0. The writer (hooks) may be absent or the file may
/// not exist yet; every error path just skips the tick and keeps looping.
fn watch_event_log(app: tauri::AppHandle) {
    let Some(path) = event_log_path() else {
        return;
    };

    // Start at current length so pre-existing events don't replay as moods.
    let mut offset = std::fs::metadata(&path)
        .map(|m| m.len() as usize)
        .unwrap_or(0);

    loop {
        if let Ok(bytes) = std::fs::read(&path) {
            if bytes.len() < offset {
                // File was truncated/rotated — restart from the beginning.
                offset = 0;
            }
            if bytes.len() > offset {
                let (new_events, new_offset) = events::parse(&bytes, offset);
                offset = new_offset;
                for event in &new_events {
                    if let Some(mood) = events::mood_for_event(event) {
                        let _ = app.emit("mood", mood);
                    }
                }
            }
        }
        std::thread::sleep(Duration::from_millis(250));
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

// The pet shell. Window chrome/transparency/always-on-top live in
// tauri.conf.json so later slices can extend behavior without touching this.

pub mod events;
pub mod growth;
pub mod mood;
pub mod settings;
pub mod speaker;

use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use speaker::{LlmClient as _, Speaker};
use tauri::Emitter;

/// The event-log location, shared by the Claude Code hooks and this watcher.
/// `$HOME/.claude-copet/events.jsonl` (`%USERPROFILE%` on Windows).
fn event_log_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
    Some(PathBuf::from(home).join(".claude-copet").join("events.jsonl"))
}

/// The persisted pet-state location.
/// `$HOME/.claude-copet/state.json` (`%USERPROFILE%` on Windows).
fn state_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
    Some(PathBuf::from(home).join(".claude-copet").join("state.json"))
}

/// The persisted settings location: `$HOME/.claude-copet/settings.json`.
fn settings_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
    Some(PathBuf::from(home).join(".claude-copet").join("settings.json"))
}

/// "YYYY-MM-DD" today. Pure arithmetic on the Unix epoch — no chrono crate needed.
fn today_string() -> String {
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let days_since_epoch = (dur.as_secs() / 86_400) as i64;

    // Howard Hinnant's civil-from-days algorithm.
    let z = days_since_epoch + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097; // day of era
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365; // year of era
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // day of year
    let mp = (5 * doy + 2) / 153; // month ordinal
    let d = doy - (153 * mp + 2) / 5 + 1; // day
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // month
    let y = if m <= 2 { y + 1 } else { y };

    format!("{y:04}-{m:02}-{d:02}")
}

/// Load persisted pet state, or create a fresh one with today's birth date.
fn load_or_init_state() -> growth::PetState {
    if let Some(path) = state_path() {
        if let Ok(bytes) = fs::read(&path) {
            if let Ok(state) = serde_json::from_slice::<growth::PetState>(&bytes) {
                return state;
            }
        }
    }
    growth::PetState::initial(today_string())
}

/// Persist pet state to disk. Never panics — file IO errors are silently ignored
/// so a transient disk issue doesn't kill the pet.
fn save_state(state: &growth::PetState) {
    if let Some(path) = state_path() {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_vec_pretty(state) {
            let _ = fs::write(&path, json);
        }
    }
}

/// Emit the new mood to the frontend and, on mood entry, a spoken template line.
fn announce(app: &tauri::AppHandle, speaker: &mut speaker::TemplateSpeaker, mood: events::Mood) {
    let _ = app.emit("mood", mood);
    if let Some(line) = speaker.speak(&speaker::SpeakContext {
        mood,
        event: None,
        state: None,
        prev_mood: None,
        stage_changed: false,
    }) {
        let _ = app.emit("speech", line);
    }
}

/// Drive the mood state machine AND growth aggregator from the event log.
/// New events preempt the mood, reset its decay timer, and feed the growth
/// system (XP/level/stage). Quiet polls are pure time ticks that let the mood
/// decay. Growth state persists to `state.json` so progress survives restarts.
fn watch_event_log(app: tauri::AppHandle, cfg: settings::Settings) {
    let Some(log_path) = event_log_path() else {
        return;
    };

    // Load persisted growth state (level/XP/stage) or initialise a new pet.
    let mut pet_state = load_or_init_state();
    // Emit the full pet state on first load so the frontend can display level/stage.
    let _ = app.emit("pet_state", serde_json::to_value(&pet_state).unwrap_or_default());

    // The events-log cursor is separate from the persisted offset: mood always
    // starts at the current end of the log so pre-existing events don't replay into
    // the ephemeral mood engine. Growth state is driven from persisted offset.
    let mut mood_offset = fs::metadata(&log_path)
        .map(|m| m.len() as usize)
        .unwrap_or(0);
    let mut mood_state = mood::MoodState::initial();

    // Seed line selection from the wall clock for run-to-run variety.
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0xC0FF_EE00);
    let mut template_speaker = speaker::TemplateSpeaker::new(seed);
    let mut last = Instant::now();

    // Optional LLM cooldown state — shared with the spawned threads.
    // Cooldown: 5 minutes between LLM calls.
    let llm_last_call: Arc<Mutex<Option<Instant>>> = Arc::new(Mutex::new(None));
    const LLM_COOLDOWN_SECS: u64 = 300;

    loop {
        std::thread::sleep(Duration::from_millis(250));
        let now = Instant::now();
        let delta = now.duration_since(last);
        last = now;

        let mut new_events: Vec<events::Event> = Vec::new();
        if let Ok(bytes) = fs::read(&log_path) {
            if bytes.len() < mood_offset {
                mood_offset = 0; // file truncated/rotated — restart from beginning
            }
            if bytes.len() > mood_offset {
                let (evs, new_offset) = events::parse(&bytes, mood_offset);
                mood_offset = new_offset;
                new_events = evs;
            }
        }

        // ── Growth: feed consumed events into the aggregator ──
        if !new_events.is_empty() {
            let delta_s = delta.as_secs();
            let today = today_string();
            let stage_before = pet_state.pet.stage;
            growth::aggregate(&mut pet_state, &new_events, delta_s, &today);
            let stage_changed = pet_state.pet.stage != stage_before;
            if stage_changed {
                let _ = app.emit("stage", pet_state.pet.stage);
            }
            save_state(&pet_state);

            // ── LLM special-moment path ──
            if cfg.llm_enabled {
                let env_key = std::env::var("ANTHROPIC_API_KEY").ok();
                if let Some(api_key) = cfg.resolve_api_key(env_key.as_deref()) {
                    let model = cfg.model.clone();

                    // Check each event for a special moment; fire at most one LLM call.
                    let mut fired = false;
                    let prev_mood_for_batch = mood_state.mood;

                    for event in &new_events {
                        if fired {
                            break;
                        }
                        // Compute candidate mood for this event.
                        let candidate_mood = events::mood_for_event(event).unwrap_or(mood_state.mood);
                        if speaker::is_special_moment(
                            event,
                            &pet_state,
                            candidate_mood,
                            Some(prev_mood_for_batch),
                            stage_changed,
                        ) {
                            // Check cooldown before spawning.
                            let cooldown_ok = {
                                let guard = llm_last_call.lock().unwrap();
                                guard.map_or(true, |t: Instant| {
                                    t.elapsed() >= Duration::from_secs(LLM_COOLDOWN_SECS)
                                })
                            };
                            if cooldown_ok {
                                {
                                    let mut guard = llm_last_call.lock().unwrap();
                                    *guard = Some(Instant::now());
                                }
                                fired = true;
                                let app_clone = app.clone();
                                let ev_clone = event.clone();
                                let st_clone = pet_state.clone();
                                let key_clone = api_key.clone();
                                let model_clone = model.clone();
                                let mood_snap = candidate_mood;

                                std::thread::spawn(move || {
                                    let client = speaker::AnthropicClient {
                                        api_key: key_clone,
                                        model: model_clone,
                                    };
                                    let summary = speaker::build_summary(&ev_clone, &st_clone, mood_snap);
                                    let system = "You are a small pixel desktop pet companion for \
                                        a developer. Respond with exactly one short, friendly, \
                                        encouraging line (max 12 words). No hashtags, no special characters.";
                                    if let Ok(line) = client.complete(system, &summary) {
                                        if !line.is_empty() {
                                            let _ = app_clone.emit("speech", line);
                                        }
                                    }
                                });
                            }
                        }
                    }
                }
            }
        }

        // ── Mood: drive the state machine ──
        if new_events.is_empty() {
            // No events this tick: advance time so the current mood can decay.
            let (next, signals) = mood::step(&mood_state, None, delta);
            mood_state = next;
            if signals.mood_changed {
                announce(&app, &mut template_speaker, mood_state.mood);
            }
        } else {
            // Events in this batch are treated as simultaneous: the elapsed delta
            // applies to the first, the rest advance time by zero.
            let mut first = true;
            for event in &new_events {
                let d = if first { delta } else { Duration::ZERO };
                first = false;
                let (next, signals) = mood::step(&mood_state, Some(event), d);
                mood_state = next;
                if signals.mood_changed {
                    announce(&app, &mut template_speaker, mood_state.mood);
                }
            }
        }
    }
}

// ─────────────────────────── Tauri commands ──────────────────────────────────

#[tauri::command]
fn get_settings() -> settings::Settings {
    settings_path()
        .and_then(|p| settings::Settings::load_from(&p).ok())
        .unwrap_or_else(settings::Settings::default)
}

#[tauri::command]
fn set_settings(s: settings::Settings) {
    if let Some(path) = settings_path() {
        let _ = s.save_to(&path);
    }
}

// ─────────────────────────── entry point ─────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Load settings once at startup; inject into the watcher.
    let cfg = settings_path()
        .and_then(|p| settings::Settings::load_from(&p).ok())
        .unwrap_or_else(settings::Settings::default);

    tauri::Builder::default()
        .setup(|app| {
            let handle = app.handle().clone();
            std::thread::spawn(move || watch_event_log(handle, cfg));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_settings, set_settings])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// The HUD shell. Window chrome/transparency/always-on-top live in
// tauri.conf.json. This file owns the event-log watcher (drives the corner
// pet's mood) and the Tauri commands the frontend invokes.

pub mod events;
pub mod hooks_install;
pub mod mood;
pub mod session;
pub mod settings;

use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use serde::Serialize;
use tauri::Emitter;

/// The full HUD snapshot emitted to the frontend. Grows across slices
/// (model / context % / activity / needs-human); slice 2 carries the session
/// identity. Re-emitted on startup and whenever any field changes.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
struct HudSnapshot {
    #[serde(rename = "sessionLabel")]
    session_label: String,
    #[serde(rename = "sessionId")]
    session_id: String,
    /// Friendly model name (e.g. "Opus 4.8"), or `null` when no transcript usage
    /// is available.
    #[serde(rename = "model")]
    model: Option<String>,
    /// Context used as a percentage, or `null` when no transcript is readable.
    #[serde(rename = "contextPercent")]
    context_percent: Option<f64>,
}

/// Read at most `max_bytes` from the END of `path` (a bounded tail — never loads
/// a huge transcript fully). Returns `None` if the file can't be read.
fn read_tail(path: &str, max_bytes: u64) -> Option<Vec<u8>> {
    use std::io::{Read, Seek, SeekFrom};
    let mut f = fs::File::open(path).ok()?;
    let len = f.metadata().ok()?.len();
    let start = len.saturating_sub(max_bytes);
    f.seek(SeekFrom::Start(start)).ok()?;
    let mut buf = Vec::new();
    f.take(max_bytes).read_to_end(&mut buf).ok()?;
    Some(buf)
}

/// How much of a transcript tail we read to find the latest assistant usage.
const TRANSCRIPT_TAIL_BYTES: u64 = 512 * 1024;

/// The event-log location, shared by the Claude Code hooks and this watcher.
/// `$HOME/.claude-copet/events.jsonl` (`%USERPROFILE%` on Windows).
fn event_log_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
    Some(PathBuf::from(home).join(".claude-copet").join("events.jsonl"))
}

/// The persisted settings location: `$HOME/.claude-copet/settings.json`.
fn settings_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
    Some(PathBuf::from(home).join(".claude-copet").join("settings.json"))
}

// ─────────────────────── hook-install helpers ────────────────────────────────

/// The hook script source, bundled at compile time.
const HOOK_SCRIPT: &str = include_str!("../../hooks/claude-copet-hook.sh");

/// Where we deploy the bundled hook script: `$HOME/.claude-copet/claude-copet-hook.sh`.
fn hook_script_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
    Some(
        PathBuf::from(home)
            .join(".claude-copet")
            .join("claude-copet-hook.sh"),
    )
}

/// Claude Code's settings file: `$HOME/.claude/settings.json`.
fn claude_settings_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
    Some(PathBuf::from(home).join(".claude").join("settings.json"))
}

/// Read `path` as a JSON Value, or return `{}` if the file is missing/corrupt.
fn read_json_or_empty(path: &PathBuf) -> serde_json::Value {
    fs::read(path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_else(|| serde_json::json!({}))
}

/// Write `value` back as pretty-printed JSON, creating parent dirs as needed.
fn write_json(path: &PathBuf, value: &serde_json::Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let bytes = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    fs::write(path, bytes).map_err(|e| e.to_string())
}

/// Emit the new mood to the frontend (drives the corner pet's sprite).
fn announce(app: &tauri::AppHandle, mood: events::Mood) {
    let _ = app.emit("mood", mood);
}

/// Drive the mood state machine from the event log. New events preempt the mood
/// and reset its decay timer; quiet polls are pure time ticks that let the mood
/// decay. Mood is ephemeral — nothing is persisted.
fn watch_event_log(app: tauri::AppHandle) {
    let Some(log_path) = event_log_path() else {
        return;
    };

    // Mood always starts at the current end of the log so pre-existing events
    // don't replay into the ephemeral mood engine.
    let mut mood_offset = fs::metadata(&log_path)
        .map(|m| m.len() as usize)
        .unwrap_or(0);
    let mut mood_state = mood::MoodState::initial();
    let mut last = Instant::now();

    // Active-session context: the session/cwd of the most-recently-consumed
    // event owns the card. Emit an initial (empty) snapshot so the frontend
    // renders a placeholder until the first event arrives.
    let mut active_session: Option<String> = None;
    let mut active_cwd: Option<String> = None;
    let mut active_transcript: Option<String> = None;
    let mut last_hud = HudSnapshot::default();
    let _ = app.emit("hud", &last_hud);

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

        // ── Active session: the latest event carrying a session owns the card ──
        if !new_events.is_empty() {
            for ev in &new_events {
                if let Some(s) = &ev.session {
                    if !s.is_empty() {
                        active_session = Some(s.clone());
                        active_cwd = ev.cwd.clone();
                        active_transcript = ev.transcript_path.clone();
                    }
                }
            }

            // Read the active session's transcript tail for model + context %.
            // Missing/unreadable transcript → both stay None (frontend shows "—").
            let (model, context_percent) = active_transcript
                .as_deref()
                .filter(|p| !p.is_empty())
                .and_then(|p| read_tail(p, TRANSCRIPT_TAIL_BYTES))
                .and_then(|tail| session::latest_usage_and_model(&tail))
                .map(|um| {
                    let pct = session::context_percent(&um.usage, &um.model);
                    (Some(session::model_friendly_name(&um.model)), Some(pct))
                })
                .unwrap_or((None, None));

            let hud = HudSnapshot {
                session_label: active_cwd
                    .as_deref()
                    .map(session::session_label)
                    .unwrap_or_default(),
                session_id: active_session.clone().unwrap_or_default(),
                model,
                context_percent,
            };
            if hud != last_hud {
                let _ = app.emit("hud", &hud);
                last_hud = hud;
            }
        }

        // ── Mood: drive the state machine ──
        if new_events.is_empty() {
            // No events this tick: advance time so the current mood can decay.
            let (next, signals) = mood::step(&mood_state, None, delta);
            mood_state = next;
            if signals.mood_changed {
                announce(&app, mood_state.mood);
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
                    announce(&app, mood_state.mood);
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

/// Invoked by the frontend when the user clicks (not drags) the pet.
/// Emits a Happy mood entry — same path as the watcher.
#[tauri::command]
fn pet_clicked(app: tauri::AppHandle) {
    announce(&app, events::Mood::Happy);
}

/// Clean exit from the context menu.
#[tauri::command]
fn quit_app(app: tauri::AppHandle) {
    app.exit(0);
}

/// Write the bundled hook script to `~/.claude-copet/claude-copet-hook.sh`
/// (chmod 755 on Unix), then idempotently merge the six hook entries into
/// `~/.claude/settings.json`. A `.bak` copy is written before any modification.
#[tauri::command]
fn install_hooks() -> Result<(), String> {
    // 1. Write the hook script to ~/.claude-copet/
    let script_path = hook_script_path().ok_or("cannot resolve HOME")?;
    if let Some(parent) = script_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&script_path, HOOK_SCRIPT).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        let perms = fs::Permissions::from_mode(0o755);
        fs::set_permissions(&script_path, perms).map_err(|e| e.to_string())?;
    }

    // 2. Read existing ~/.claude/settings.json (missing => {}).
    let claude_path = claude_settings_path().ok_or("cannot resolve HOME")?;
    let existing_file = claude_path.exists();
    let current = read_json_or_empty(&claude_path);

    // 3. Back up before modifying (only when the file already existed).
    if existing_file {
        let bak = claude_path.with_extension("json.bak");
        let bak_bytes = serde_json::to_vec_pretty(&current).map_err(|e| e.to_string())?;
        fs::write(&bak, bak_bytes).map_err(|e| e.to_string())?;
    }

    // 4. Merge and write back.
    let script_str = script_path.to_string_lossy().into_owned();
    let updated = hooks_install::merge_copet_hooks(current, &script_str);
    write_json(&claude_path, &updated)
}

/// Remove only our hook entries from `~/.claude/settings.json`, leaving all
/// other hooks intact. A `.bak` copy is written before any modification.
#[tauri::command]
fn uninstall_hooks() -> Result<(), String> {
    let claude_path = claude_settings_path().ok_or("cannot resolve HOME")?;
    if !claude_path.exists() {
        return Ok(()); // nothing to remove
    }
    let current = read_json_or_empty(&claude_path);

    // Back up before modifying.
    let bak = claude_path.with_extension("json.bak");
    let bak_bytes = serde_json::to_vec_pretty(&current).map_err(|e| e.to_string())?;
    fs::write(&bak, bak_bytes).map_err(|e| e.to_string())?;

    let updated = hooks_install::remove_copet_hooks(current);
    write_json(&claude_path, &updated)
}

/// Return true iff all six copet hooks are present in `~/.claude/settings.json`.
#[tauri::command]
fn hooks_status() -> bool {
    let Some(path) = claude_settings_path() else {
        return false;
    };
    if !path.exists() {
        return false;
    }
    let settings = read_json_or_empty(&path);
    hooks_install::copet_hooks_installed(&settings)
}

// ─────────────────────────── entry point ─────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .setup(|app| {
            let handle = app.handle().clone();
            std::thread::spawn(move || watch_event_log(handle));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_settings, set_settings, pet_clicked, quit_app,
            install_hooks, uninstall_hooks, hooks_status
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

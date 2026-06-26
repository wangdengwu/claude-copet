// The HUD shell. Window chrome/transparency/always-on-top live in
// tauri.conf.json. This file owns the event-log watcher (drives the corner
// pet's mood) and the Tauri commands the frontend invokes.

pub mod events;
pub mod hooks_install;
pub mod mood;
pub mod session;
pub mod settings;

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use serde::Serialize;
use tauri::Emitter;

use crate::session::ContextClient;

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
    /// Current activity line, e.g. "Running Bash" or "Idle".
    #[serde(rename = "activity")]
    activity: String,
    /// True while Claude is waiting on the user (permission/input or turn done).
    #[serde(rename = "needsHuman")]
    needs_human: bool,
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

// ─────────────────────── production ContextClient ────────────────────────────

struct ClaudeCliContextClient;

impl session::ContextClient for ClaudeCliContextClient {
    fn fetch_context(&self, session_id: &str, cwd: &str) -> Result<String, ()> {
        let output = std::process::Command::new("claude")
            .arg("-p")
            .arg("--resume")
            .arg(session_id)
            .arg("--output-format")
            .arg("text")
            .arg("/context")
            .current_dir(cwd)
            .output()
            .map_err(|_| ())?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let _ = std::fs::write(
            dirs_next().join(".claude-copet").join("context-debug.log"),
            format!(
                "sid={session_id} cwd={cwd} ok={} stdout={stdout} stderr={stderr}\n",
                output.status.success()
            ),
        );
        if !output.status.success() {
            return Err(());
        }
        Ok(stdout)
    }
}

fn dirs_next() -> std::path::PathBuf {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .unwrap_or_default();
    std::path::PathBuf::from(home)
}

/// How much of a transcript tail we read to find the latest assistant usage.
/// Generous so the most recent assistant line (which can be large near full
/// context — exactly when the % matters most) isn't truncated at the tail's head.
/// Still bounded — never the whole file.
const TRANSCRIPT_TAIL_BYTES: u64 = 2 * 1024 * 1024;

/// The event-log location, shared by the Claude Code hooks and this watcher.
/// `$HOME/.claude-copet/events.jsonl` (`%USERPROFILE%` on Windows).
fn event_log_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
    Some(
        PathBuf::from(home)
            .join(".claude-copet")
            .join("events.jsonl"),
    )
}

/// The persisted settings location: `$HOME/.claude-copet/settings.json`.
fn settings_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
    Some(
        PathBuf::from(home)
            .join(".claude-copet")
            .join("settings.json"),
    )
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
    // Cached transcript-derived fields (refreshed only on event batches, since the
    // transcript only grows on events) + the running activity/attention state.
    let mut cached_model: Option<String> = None;
    let mut cached_context: Option<f64> = None;
    // Snapshot of the last transcript read, so we can recompute context %
    // on every tick when the L1 cache is updated by the background /context thread.
    let mut last_usage: Option<session::Usage> = None;
    let mut last_seen_model: Option<String> = None;
    let mut last_tool: Option<String> = None;
    let mut attention = false;
    let mut last_hud = HudSnapshot::default();
    // Context-client state: a PER-SESSION L1 cache from /context (keyed by
    // session id), an in-flight guard, and a pending-fetch flag. The map is the
    // fix for the "every switch re-flashes 15%→3%" bug: each session keeps its
    // own resolved window across active-session switches, so returning to a
    // session reuses its cached window instead of re-running /context.
    let cached_ctx: Arc<Mutex<HashMap<String, session::CachedContext>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let context_in_flight: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
    // Pending /context request: (session_id, cwd). When set and not in-flight,
    // the watcher spawns /context for this session on the next tick. Kept as
    // Option so rapid switches overwrite; NOT consumed with .take() so it
    // survives when in_flight blocks the spawn.
    let mut pending_context_fetch: Option<(String, String)> = None;
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
                if !evs.is_empty() {}
                new_events = evs;
            }
        }

        // ── Active session + attention + activity inputs (event-driven) ──
        if !new_events.is_empty() {
            let prev_session = active_session.clone();
            for ev in &new_events {
                if let Some(s) = &ev.session {
                    if !s.is_empty() {
                        active_session = Some(s.clone());
                        active_cwd = ev.cwd.clone();
                        active_transcript = ev.transcript_path.clone();
                    }
                }
            }

            // Only fold attention + tool for events matching the active session
            // (the one shown on the card). Cross-session events are ignored here.
            if let Some(ref active_sid) = active_session {
                for ev in &new_events {
                    if ev.session.as_deref() == Some(active_sid.as_str()) {
                        attention = session::attention_step(attention, ev);
                        match ev.event_type.as_str() {
                            "PreToolUse" => {
                                if let Some(t) = &ev.tool {
                                    if !t.is_empty() {
                                        last_tool = Some(t.clone());
                                    }
                                }
                            }
                            "PostToolUse" | "Stop" => last_tool = None,
                            _ => {}
                        }
                    }
                }
            }

            // A new session owns the card: drop the previous session's derived
            // DISPLAY state (model / context % / tool / attention) and the
            // transcript snapshot (last_usage / last_seen_model) so the per-tick
            // recompute below can't keep showing the previous session's figures.
            // We do NOT clear the /context cache map — each session keeps its own
            // entry — and we only schedule a /context fetch when this session has
            // no cached window yet (model-change re-fetch is handled separately).
            if active_session != prev_session {
                cached_model = None;
                cached_context = None;
                last_usage = None;
                last_seen_model = None;
                last_tool = None;
                attention = false;
                let need_fetch = active_session
                    .as_deref()
                    .map(|sid| !cached_ctx.lock().unwrap().contains_key(sid))
                    .unwrap_or(false);
                pending_context_fetch = if need_fetch {
                    active_session
                        .as_ref()
                        .zip(active_cwd.as_ref())
                        .map(|(s, c)| (s.clone(), c.clone()))
                } else {
                    None
                };
            }

            // Refresh model + context % from the active session's transcript tail
            // (bounded read). Only overwrite the cache on a SUCCESSFUL read, so a
            // transient unreadable/incomplete transcript (common before the first
            // assistant turn is flushed) doesn't flicker the card back to "—".
            let read_result = active_transcript
                .as_deref()
                .filter(|p| !p.is_empty())
                .and_then(|p| read_tail(p, TRANSCRIPT_TAIL_BYTES))
                .and_then(|tail| session::latest_usage_and_model(&tail));
            if let Some(um) = read_result {
                // The session that owns the card right now. Every cache lookup
                // below is keyed on this session id, so one session's /context
                // result can never leak its window/model onto another's card.
                let active_sid = active_session.as_deref().unwrap_or("");

                // Snapshot for per-tick recomputation.
                last_usage = Some(um.usage.clone());
                last_seen_model = Some(um.model.clone());

                // Update last_transcript_model FIRST — before the model_changed
                // check — so the first read after a /context fetch doesn't
                // falsely detect a mismatch (the thread leaves it empty).
                if let Some(ctx) = cached_ctx.lock().unwrap().get_mut(active_sid) {
                    if ctx.last_transcript_model.is_empty() {
                        ctx.last_transcript_model = um.model.clone();
                    }
                }

                // Model mismatch detection: if the transcript model differs from
                // the one last seen at this session's /context fetch time,
                // re-fetch context for this session.
                let model_different = {
                    let mut guard = cached_ctx.lock().unwrap();
                    match guard.get_mut(active_sid) {
                        Some(ctx) => {
                            let diff =
                                session::model_changed(Some(&ctx.last_transcript_model), &um.model);
                            if diff {
                                // Update baseline so we don't re-fire on every tick.
                                ctx.last_transcript_model = um.model.clone();
                            }
                            diff
                        }
                        None => false,
                    }
                };
                if model_different {
                    pending_context_fetch = active_session
                        .as_ref()
                        .zip(active_cwd.as_ref())
                        .map(|(s, c)| (s.clone(), c.clone()));
                }

                // Compute window using L1 (this session's cached /context) or L3
                // fallback.
                let window = {
                    let guard = cached_ctx.lock().unwrap();
                    let cached = guard.get(active_sid).map(|c| c.window_size);
                    session::resolve_window(cached, &um.usage, &um.model)
                };
                cached_context = Some(session::context_percent(&um.usage, window));

                // Prefer this session's cached model alias, fall back to the
                // transcript model id.
                cached_model = {
                    let guard = cached_ctx.lock().unwrap();
                    guard
                        .get(active_sid)
                        .map(|c| session::model_friendly_name(&c.model_alias))
                        .or_else(|| Some(session::model_friendly_name(&um.model)))
                };

                // last_transcript_model already updated above (before window).
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

        // ── Recompute context % on every tick (cache keyed by the active
        //    session so a stale /context result can't skew the window/model).
        if let (Some(ref usage), Some(ref model)) = (&last_usage, &last_seen_model) {
            let active_sid = active_session.as_deref().unwrap_or("");
            let window = {
                let guard = cached_ctx.lock().unwrap();
                session::resolve_window(guard.get(active_sid).map(|c| c.window_size), usage, model)
            };
            cached_context = Some(session::context_percent(usage, window));
            cached_model = {
                let guard = cached_ctx.lock().unwrap();
                guard
                    .get(active_sid)
                    .map(|c| session::model_friendly_name(&c.model_alias))
                    .or_else(|| Some(session::model_friendly_name(model)))
            };
        }

        // ── HUD: rebuild every tick (so activity decays to "Idle" and the
        //    needs-human alert clears even on quiet polls) and emit on change. ──
        let hud = HudSnapshot {
            session_label: active_cwd
                .as_deref()
                .map(session::session_label)
                .unwrap_or_default(),
            session_id: active_session.clone().unwrap_or_default(),
            model: cached_model.clone(),
            context_percent: cached_context,
            activity: session::activity_label(mood_state.mood, last_tool.as_deref()),
            needs_human: attention,
        };
        if hud != last_hud {
            let _ = app.emit("hud", &hud);
            last_hud = hud;
        }

        // ── L1 context: spawn a one-shot /context fetch when needed ──
        if let Some(ref pending) = pending_context_fetch {
            if !*context_in_flight.lock().unwrap() {
                let (sid, scwd) = pending.clone();
                pending_context_fetch = None; // consumed
                if !sid.is_empty() {
                    *context_in_flight.lock().unwrap() = true;
                    let ctx_clone = cached_ctx.clone();
                    let inflight_clone = context_in_flight.clone();
                    std::thread::spawn(move || {
                        let client = ClaudeCliContextClient;
                        let mut attempt = 0u8;
                        let max_attempts: u8 = 2;
                        while attempt < max_attempts {
                            attempt += 1;
                            match client.fetch_context(&sid, &scwd) {
                                Ok(stdout) => {
                                    match session::parse_context_output(&stdout) {
                                        Some(info) => {
                                            ctx_clone.lock().unwrap().insert(
                                                sid.clone(),
                                                session::CachedContext {
                                                    model_alias: info.model_alias,
                                                    window_size: info.window_size,
                                                    last_transcript_model: String::new(),
                                                },
                                            );
                                        }
                                        None => {}
                                    }
                                    break;
                                }
                                Err(()) => {
                                    if attempt < max_attempts {
                                        std::thread::sleep(Duration::from_secs(2));
                                    }
                                }
                            }
                        }
                        *inflight_clone.lock().unwrap() = false;
                    });
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
        // Persist only the window POSITION, never its size: the HUD card has a
        // fixed size from tauri.conf.json, and persisting size would let a stale
        // saved value (e.g. the old square 320×320) override the card dimensions.
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_state_flags(tauri_plugin_window_state::StateFlags::POSITION)
                .build(),
        )
        .setup(|app| {
            let handle = app.handle().clone();
            std::thread::spawn(move || watch_event_log(handle));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_settings,
            set_settings,
            pet_clicked,
            quit_app,
            install_hooks,
            uninstall_hooks,
            hooks_status
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

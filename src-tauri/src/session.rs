//! Pure session-context derivation for the HUD. No IO/clock/Tauri here — the
//! watcher in `lib.rs` feeds these functions and emits the result.
//!
//! Slice 2: `session_label` (the cwd basename shown on the card). Later slices
//! add transcript-tail parsing for context % and model.

use serde::Deserialize;

/// The token-usage components that count toward the context window. Mirrors the
/// fields of a transcript assistant message's `usage` block (others ignored).
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct Usage {
    #[serde(default)]
    pub input_tokens: u64,
    #[serde(default)]
    pub cache_read_input_tokens: u64,
    #[serde(default)]
    pub cache_creation_input_tokens: u64,
}

/// The model id + usage of the latest assistant message in a transcript tail.
#[derive(Debug, Clone, PartialEq)]
pub struct UsageAndModel {
    pub usage: Usage,
    pub model: String,
}

// Internal shape for deserializing one transcript line. Only the assistant
// message's model + usage matter; everything else is ignored.
#[derive(Deserialize)]
struct TranscriptLine {
    #[serde(rename = "type")]
    line_type: Option<String>,
    message: Option<TranscriptMessage>,
}

#[derive(Deserialize)]
struct TranscriptMessage {
    model: Option<String>,
    usage: Option<Usage>,
}

/// Scan JSONL `tail_bytes` and return the **last** assistant message that carries
/// both a `usage` block and a `model`. Malformed lines are skipped. `None` if no
/// such message exists. Pure — the watcher owns the bounded file read.
pub fn latest_usage_and_model(tail_bytes: &[u8]) -> Option<UsageAndModel> {
    let mut found: Option<UsageAndModel> = None;
    for line in tail_bytes.split(|&b| b == b'\n') {
        if line.is_empty() {
            continue;
        }
        let Ok(parsed) = serde_json::from_slice::<TranscriptLine>(line) else {
            continue; // skip malformed / partial lines
        };
        if parsed.line_type.as_deref() != Some("assistant") {
            continue;
        }
        let Some(msg) = parsed.message else { continue };
        if let (Some(model), Some(usage)) = (msg.model, msg.usage) {
            found = Some(UsageAndModel { usage, model });
        }
    }
    found
}

/// The context-window size (in tokens) for a model id. Default 200_000; the
/// `…[1m]` variants are 1_000_000; unknown ids fall back to the default.
pub fn context_window(model_id: &str) -> u64 {
    if model_id.contains("[1m]") {
        1_000_000
    } else {
        200_000
    }
}

/// Context used as a percentage of the model's window, summing the three input
/// token components, clamped to `[0, 100]`.
pub fn context_percent(usage: &Usage, model_id: &str) -> f64 {
    let used = usage.input_tokens
        + usage.cache_read_input_tokens
        + usage.cache_creation_input_tokens;
    let window = context_window(model_id);
    if window == 0 {
        return 0.0;
    }
    let pct = (used as f64 / window as f64) * 100.0;
    pct.clamp(0.0, 100.0)
}

/// Friendly display name for a model id: `claude-opus-4-8` → `Opus 4.8`.
/// Ignores a trailing `[1m]` marker and any dated suffix. Unknown ids (no
/// `claude-` prefix, or an unrecognised family) degrade to the raw id.
pub fn model_friendly_name(model_id: &str) -> String {
    let Some(rest) = model_id.strip_prefix("claude-") else {
        return model_id.to_string();
    };
    // Drop a "[1m]" (or any "[…]") marker before splitting.
    let base = rest.split('[').next().unwrap_or(rest);
    let mut parts = base.split('-');

    let family = match parts.next() {
        Some(f) => f,
        None => return model_id.to_string(),
    };
    let family_label = match family {
        "opus" => "Opus",
        "sonnet" => "Sonnet",
        "haiku" => "Haiku",
        "fable" => "Fable",
        _ => return model_id.to_string(), // unrecognised family
    };

    // Take the leading numeric segments as the version (e.g. "4", "8" → "4.8"),
    // stopping at the first non-numeric segment (e.g. a date) — at most two.
    let version: Vec<&str> = parts
        .take_while(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
        .take(2)
        .collect();
    if version.is_empty() {
        return model_id.to_string();
    }
    format!("{} {}", family_label, version.join("."))
}

/// The HUD's session label: the last path component of `cwd`.
///
/// Handles `/` and `\` separators, ignores a single trailing separator, and
/// returns `""` for a root path (`"/"`) or empty input. A bare name with no
/// separator is returned unchanged.
pub fn session_label(cwd: &str) -> String {
    // Drop trailing separators so ".../proj/" labels as "proj", not "".
    let trimmed = cwd.trim_end_matches(['/', '\\']);
    match trimmed.rfind(['/', '\\']) {
        Some(i) => trimmed[i + 1..].to_string(),
        None => trimmed.to_string(),
    }
}

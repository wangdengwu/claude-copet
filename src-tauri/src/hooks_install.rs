// Pure logic for installing and removing claude-copet perception hooks into
// ~/.claude/settings.json.  No filesystem access — all functions operate on
// in-memory serde_json::Value so they are fully unit-testable.

use serde_json::{json, Value};

// The six Claude Code hook events we register, and whether each needs the
// `"matcher": "*"` field (true for PreToolUse / PostToolUse).
const HOOK_EVENTS: &[(&str, bool)] = &[
    ("SessionStart",     false),
    ("UserPromptSubmit", false),
    ("PreToolUse",       true),
    ("PostToolUse",      true),
    ("Stop",             false),
    ("Notification",     false),
];

/// Sentinel string embedded in every entry we own; used for both install-check
/// and removal.
const HOOK_MARKER: &str = "claude-copet-hook.sh";

// ─────────────────────────── helpers ─────────────────────────────────────────

/// Return true iff `entry` is an object that contains one of OUR hook commands.
fn entry_is_ours(entry: &Value) -> bool {
    let Some(hooks_arr) = entry.get("hooks").and_then(|v| v.as_array()) else {
        return false;
    };
    hooks_arr.iter().any(|h| {
        h.get("command")
            .and_then(|c| c.as_str())
            .map_or(false, |cmd| cmd.contains(HOOK_MARKER))
    })
}

// ─────────────────────────── public API ──────────────────────────────────────

/// Idempotently merge the six copet hook entries into `settings`.
/// For each event: append our entry only if no existing entry already references
/// `claude-copet-hook.sh`; all unrelated entries are preserved unchanged.
/// Returns the updated Value.
pub fn merge_copet_hooks(mut settings: Value, script_path: &str) -> Value {
    // Ensure `hooks` exists and is an object.
    if !settings.get("hooks").map_or(false, |v| v.is_object()) {
        settings["hooks"] = json!({});
    }

    let hooks_obj = settings["hooks"].as_object_mut().unwrap();

    for &(event, needs_matcher) in HOOK_EVENTS {
        // Ensure the event array exists.
        let event_arr = hooks_obj
            .entry(event)
            .or_insert_with(|| json!([]));

        // Skip if we are already present (idempotency).
        if let Some(arr) = event_arr.as_array() {
            if arr.iter().any(entry_is_ours) {
                continue;
            }
        }

        let command = format!("sh {} {}", script_path, event);
        let hook_inner = json!([{ "type": "command", "command": command }]);

        let mut entry = json!({ "hooks": hook_inner });
        if needs_matcher {
            entry["matcher"] = json!("*");
        }

        event_arr
            .as_array_mut()
            .expect("event array must be an array")
            .push(entry);
    }

    settings
}

/// Remove only the entries we own from each event array.
/// Unrelated entries survive; event keys with zero remaining entries are kept
/// as empty arrays (preserves round-trip fidelity for tooling that may rely on
/// the key being present).
pub fn remove_copet_hooks(mut settings: Value) -> Value {
    let Some(hooks_obj) = settings
        .get_mut("hooks")
        .and_then(|v| v.as_object_mut())
    else {
        return settings; // nothing to do
    };

    for arr in hooks_obj.values_mut() {
        if let Some(vec) = arr.as_array_mut() {
            vec.retain(|entry| !entry_is_ours(entry));
        }
    }

    settings
}

/// True iff all six hook events have at least one entry referencing
/// `claude-copet-hook.sh`.
pub fn copet_hooks_installed(settings: &Value) -> bool {
    let Some(hooks_obj) = settings.get("hooks").and_then(|v| v.as_object()) else {
        return false;
    };

    HOOK_EVENTS.iter().all(|&(event, _)| {
        hooks_obj
            .get(event)
            .and_then(|v| v.as_array())
            .map_or(false, |arr| arr.iter().any(entry_is_ours))
    })
}

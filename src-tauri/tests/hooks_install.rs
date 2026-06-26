// Integration tests for the `hooks_install` module.
// All tests operate exclusively on in-memory serde_json::Value — they never
// read from or write to the real ~/.claude or ~/.claude-copet directories.

use claude_copet_lib::hooks_install::{
    copet_hooks_installed, merge_copet_hooks, remove_copet_hooks,
};
use serde_json::json;

// A stable, fake script path used across tests.
const SCRIPT: &str = "/home/user/.claude-copet/claude-copet-hook.sh";

// ── Behavior 1: merge adds all six hook events ────────────────────────────────

#[test]
fn merge_adds_all_six_events() {
    let settings = merge_copet_hooks(json!({}), SCRIPT);

    let hooks = settings["hooks"]
        .as_object()
        .expect("hooks must be an object");

    for event in &[
        "SessionStart",
        "UserPromptSubmit",
        "PreToolUse",
        "PostToolUse",
        "Stop",
        "Notification",
    ] {
        let arr = hooks[*event]
            .as_array()
            .expect("event key must be an array");
        assert!(
            !arr.is_empty(),
            "event {} must have at least one entry after merge",
            event
        );

        // Verify the command string references our script.
        let has_our_hook = arr.iter().any(|entry| {
            entry
                .get("hooks")
                .and_then(|h| h.as_array())
                .map_or(false, |hs| {
                    hs.iter().any(|h| {
                        h.get("command")
                            .and_then(|c| c.as_str())
                            .map_or(false, |cmd| cmd.contains("claude-copet-hook.sh"))
                    })
                })
        });
        assert!(
            has_our_hook,
            "event {} must contain claude-copet-hook.sh command",
            event
        );
    }
}

// ── Behavior 2: merge is idempotent — merging twice produces no duplicates ───

#[test]
fn merge_is_idempotent() {
    let once = merge_copet_hooks(json!({}), SCRIPT);
    let twice = merge_copet_hooks(once.clone(), SCRIPT);

    let hooks_once = once["hooks"].as_object().unwrap();
    let hooks_twice = twice["hooks"].as_object().unwrap();

    for event in &[
        "SessionStart",
        "UserPromptSubmit",
        "PreToolUse",
        "PostToolUse",
        "Stop",
        "Notification",
    ] {
        let count_once = hooks_once[*event].as_array().map_or(0, |a| a.len());
        let count_twice = hooks_twice[*event].as_array().map_or(0, |a| a.len());
        assert_eq!(
            count_once, count_twice,
            "event {} must not gain entries on second merge (idempotency)",
            event
        );
    }
}

// ── Behavior 3: merge preserves unrelated hooks alongside ours ───────────────

#[test]
fn merge_preserves_unrelated_hooks() {
    // Pre-populate a user hook on SessionStart (e.g. from tma1 / RTK).
    let existing = json!({
        "hooks": {
            "SessionStart": [
                {
                    "hooks": [
                        { "type": "command", "command": "echo hi" }
                    ]
                }
            ]
        }
    });

    let updated = merge_copet_hooks(existing, SCRIPT);
    let session_start = updated["hooks"]["SessionStart"]
        .as_array()
        .expect("SessionStart must be an array");

    // Must now contain at least two entries: the user's and ours.
    assert!(
        session_start.len() >= 2,
        "SessionStart must contain both the user's 'echo hi' hook and ours"
    );

    // The user's original hook must still be present.
    let user_hook_still_present = session_start.iter().any(|entry| {
        entry
            .get("hooks")
            .and_then(|h| h.as_array())
            .map_or(false, |hs| {
                hs.iter().any(|h| {
                    h.get("command")
                        .and_then(|c| c.as_str())
                        .map_or(false, |cmd| cmd == "echo hi")
                })
            })
    });
    assert!(
        user_hook_still_present,
        "user's 'echo hi' hook must survive the merge"
    );
}

// ── Behavior 4: remove deletes only our entries, leaves user entries intact ──

#[test]
fn remove_deletes_only_ours() {
    // Start from a fully-merged state that also has the user's echo hook.
    let existing = json!({
        "hooks": {
            "SessionStart": [
                {
                    "hooks": [
                        { "type": "command", "command": "echo hi" }
                    ]
                }
            ]
        }
    });
    let merged = merge_copet_hooks(existing, SCRIPT);
    let removed = remove_copet_hooks(merged);

    let session_start = removed["hooks"]["SessionStart"]
        .as_array()
        .expect("SessionStart array must be present after remove");

    // Our entry is gone.
    let our_hook_gone = !session_start.iter().any(|entry| {
        entry
            .get("hooks")
            .and_then(|h| h.as_array())
            .map_or(false, |hs| {
                hs.iter().any(|h| {
                    h.get("command")
                        .and_then(|c| c.as_str())
                        .map_or(false, |cmd| cmd.contains("claude-copet-hook.sh"))
                })
            })
    });
    assert!(
        our_hook_gone,
        "our hook must be removed after remove_copet_hooks"
    );

    // The user's hook is still there.
    let user_hook_remains = session_start.iter().any(|entry| {
        entry
            .get("hooks")
            .and_then(|h| h.as_array())
            .map_or(false, |hs| {
                hs.iter().any(|h| {
                    h.get("command")
                        .and_then(|c| c.as_str())
                        .map_or(false, |cmd| cmd == "echo hi")
                })
            })
    });
    assert!(
        user_hook_remains,
        "user's 'echo hi' hook must survive the remove"
    );
}

// ── Behavior 5: copet_hooks_installed is true after merge, false otherwise ───

#[test]
fn installed_true_after_merge() {
    let settings = merge_copet_hooks(json!({}), SCRIPT);
    assert!(
        copet_hooks_installed(&settings),
        "copet_hooks_installed must return true after a full merge"
    );
}

#[test]
fn installed_false_after_remove() {
    let merged = merge_copet_hooks(json!({}), SCRIPT);
    let removed = remove_copet_hooks(merged);
    assert!(
        !copet_hooks_installed(&removed),
        "copet_hooks_installed must return false after remove_copet_hooks"
    );
}

#[test]
fn installed_false_on_empty_settings() {
    assert!(
        !copet_hooks_installed(&json!({})),
        "copet_hooks_installed must return false for empty settings"
    );
}

// ── Behavior 6: matcher field is set only for PreToolUse and PostToolUse ─────

#[test]
fn matcher_present_only_for_tool_events() {
    let settings = merge_copet_hooks(json!({}), SCRIPT);
    let hooks = settings["hooks"].as_object().unwrap();

    let tool_events = ["PreToolUse", "PostToolUse"];
    let non_tool_events = ["SessionStart", "UserPromptSubmit", "Stop", "Notification"];

    for event in &tool_events {
        let arr = hooks[*event].as_array().unwrap();
        let our_entry = arr
            .iter()
            .find(|e| {
                e.get("hooks")
                    .and_then(|h| h.as_array())
                    .map_or(false, |hs| {
                        hs.iter().any(|h| {
                            h.get("command")
                                .and_then(|c| c.as_str())
                                .map_or(false, |cmd| cmd.contains("claude-copet-hook.sh"))
                        })
                    })
            })
            .expect("our entry must exist");
        assert_eq!(
            our_entry.get("matcher").and_then(|m| m.as_str()),
            Some("*"),
            "event {} must have matcher = \"*\"",
            event
        );
    }

    for event in &non_tool_events {
        let arr = hooks[*event].as_array().unwrap();
        let our_entry = arr
            .iter()
            .find(|e| {
                e.get("hooks")
                    .and_then(|h| h.as_array())
                    .map_or(false, |hs| {
                        hs.iter().any(|h| {
                            h.get("command")
                                .and_then(|c| c.as_str())
                                .map_or(false, |cmd| cmd.contains("claude-copet-hook.sh"))
                        })
                    })
            })
            .expect("our entry must exist");
        assert!(
            our_entry.get("matcher").is_none(),
            "event {} must NOT have a matcher field",
            event
        );
    }
}

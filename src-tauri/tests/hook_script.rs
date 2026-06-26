//! The bundled hook shell script must append one JSON line carrying the new
//! `cwd` / `transcript_path` fields (extracted from Claude Code's stdin payload),
//! stay fire-and-forget, and always exit 0. Runs the real script under `sh` with
//! `$HOME` pointed at a temp dir so it never touches the developer's home.

use std::io::Write;
use std::process::{Command, Stdio};
use tempfile::TempDir;

fn script_path() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("hooks")
        .join("claude-copet-hook.sh")
}

#[test]
fn hook_appends_enriched_line_and_exits_zero() {
    let home = TempDir::new().expect("temp home");

    let payload = r#"{"tool_name":"Bash","session_id":"sess-123","cwd":"/Users/me/proj","transcript_path":"/tmp/t.jsonl","other":"ignored"}"#;

    let mut child = Command::new("sh")
        .arg(script_path())
        .arg("PreToolUse")
        .env("HOME", home.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()
        .expect("spawn hook");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(payload.as_bytes())
        .unwrap();
    let status = child.wait().expect("wait hook");
    assert!(status.success(), "hook must always exit 0");

    let log = home.path().join(".claude-copet").join("events.jsonl");
    let contents = std::fs::read_to_string(&log).expect("hook must write the log");
    let line = contents.lines().last().expect("at least one line");

    let v: serde_json::Value = serde_json::from_str(line).expect("line must be valid JSON");
    assert_eq!(v["type"], "PreToolUse");
    assert_eq!(v["tool"], "Bash");
    assert_eq!(v["session"], "sess-123");
    assert_eq!(v["cwd"], "/Users/me/proj");
    assert_eq!(v["transcript_path"], "/tmp/t.jsonl");
}

/// Real Claude Code payloads nest a `tool_input` object that can carry colliding
/// keys (e.g. a tool argument literally named "cwd"). The session-level `cwd`
/// is top-level and must win — a greedy match would wrongly grab the nested one.
#[test]
fn top_level_cwd_wins_over_a_nested_tool_input_key() {
    let home = TempDir::new().expect("temp home");

    // Top-level cwd is the session dir; tool_input has its own bogus "cwd".
    let payload = r#"{"session_id":"s1","transcript_path":"/tmp/t.jsonl","cwd":"/Users/me/realproj","hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"ls","cwd":"/some/other/dir"}}"#;

    let mut child = Command::new("sh")
        .arg(script_path())
        .arg("PreToolUse")
        .env("HOME", home.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()
        .expect("spawn hook");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(payload.as_bytes())
        .unwrap();
    assert!(child.wait().expect("wait").success());

    let log = home.path().join(".claude-copet").join("events.jsonl");
    let contents = std::fs::read_to_string(&log).expect("log");
    let v: serde_json::Value =
        serde_json::from_str(contents.lines().last().unwrap()).expect("valid JSON");

    assert_eq!(
        v["cwd"], "/Users/me/realproj",
        "top-level session cwd must win"
    );
    assert_eq!(v["tool"], "Bash");
    assert_eq!(v["session"], "s1");
}

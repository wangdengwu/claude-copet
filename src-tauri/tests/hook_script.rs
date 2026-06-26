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

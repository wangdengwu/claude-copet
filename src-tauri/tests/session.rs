//! Pure session-context derivation. Slice 2: `session_label` (cwd basename).

use claude_copet_lib::session::session_label;

#[test]
fn label_is_last_component_of_unix_path() {
    assert_eq!(session_label("/Users/me/Study/claude-copet"), "claude-copet");
}

#[test]
fn label_handles_windows_backslash_path() {
    assert_eq!(session_label("C:\\Users\\me\\my-proj"), "my-proj");
}

#[test]
fn label_ignores_a_trailing_separator() {
    assert_eq!(session_label("/Users/me/proj/"), "proj");
    assert_eq!(session_label("C:\\Users\\me\\proj\\"), "proj");
}

#[test]
fn label_of_a_bare_name_is_itself() {
    assert_eq!(session_label("claude-copet"), "claude-copet");
}

#[test]
fn label_of_root_or_empty_is_empty() {
    assert_eq!(session_label("/"), "");
    assert_eq!(session_label(""), "");
}

#[test]
fn label_uses_the_last_separator_of_either_kind() {
    // A path mixing both separators resolves on whichever comes last.
    assert_eq!(session_label("/Users/me/proj\\sub"), "sub");
}

//! Pure session-context derivation for the HUD. No IO/clock/Tauri here — the
//! watcher in `lib.rs` feeds these functions and emits the result.
//!
//! Slice 2: `session_label` (the cwd basename shown on the card). Later slices
//! add transcript-tail parsing for context % and model.

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

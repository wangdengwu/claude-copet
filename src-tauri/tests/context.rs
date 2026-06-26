//! Pure functions for the HUD context-% pipeline. Slice 1 of the real-context-window
//! PRD adds: `parse_context_output` (stdout parser), `model_changed` (mismatch detect),
//! and refactors `context_percent` to accept an explicit window_size.

use claude_copet_lib::session::{
    context_percent, context_window, latest_usage_and_model, model_changed,
    model_friendly_name, parse_context_output, resolve_window, CachedContext, Usage,
};

fn assistant_line(model: &str, input: u64, cache_read: u64, cache_create: u64) -> String {
    format!(
        r#"{{"type":"assistant","message":{{"model":"{model}","usage":{{"input_tokens":{input},"cache_read_input_tokens":{cache_read},"cache_creation_input_tokens":{cache_create},"output_tokens":42}}}}}}"#
    )
}

// ─────────────────────────── latest_usage_and_model ──────────────────────────

#[test]
fn picks_the_last_assistant_message_with_usage() {
    let tail = format!(
        "{}\n{}\n",
        assistant_line("claude-opus-4-8", 1, 2, 3),
        assistant_line("claude-sonnet-4-6", 100, 200, 300),
    );
    let got = latest_usage_and_model(tail.as_bytes()).expect("should find usage");
    assert_eq!(got.model, "claude-sonnet-4-6", "must pick the LAST assistant message");
    assert_eq!(got.usage.input_tokens, 100);
    assert_eq!(got.usage.cache_read_input_tokens, 200);
    assert_eq!(got.usage.cache_creation_input_tokens, 300);
}

#[test]
fn skips_interleaved_malformed_lines() {
    let tail = format!(
        "not json at all\n{}\n{{ broken\n",
        assistant_line("claude-opus-4-8", 5, 6, 7),
    );
    let got = latest_usage_and_model(tail.as_bytes()).expect("should still find the good line");
    assert_eq!(got.model, "claude-opus-4-8");
    assert_eq!(got.usage.input_tokens, 5);
}

#[test]
fn returns_none_when_no_assistant_message_has_usage() {
    let tail = concat!(
        r#"{"type":"user","message":{"content":"hi"}}"#,
        "\n",
        r#"{"type":"assistant","message":{"model":"claude-opus-4-8"}}"#, // no usage block
        "\n",
    );
    assert!(latest_usage_and_model(tail.as_bytes()).is_none());
}

// ─────────────────────────── context_window (unchanged) ──────────────────────

#[test]
fn window_defaults_and_1m_variants() {
    assert_eq!(context_window("claude-opus-4-8"), 200_000);
    assert_eq!(context_window("claude-sonnet-4-6"), 200_000);
    assert_eq!(context_window("claude-opus-4-8[1m]"), 1_000_000);
    assert_eq!(context_window("claude-sonnet-4-6[1m]"), 1_000_000);
    assert_eq!(context_window("something-unknown"), 200_000, "unknown falls back to default");
}

// ─────────────────────────── context_percent (refactored) ────────────────────

fn usage(input: u64, cache_read: u64, cache_create: u64) -> Usage {
    Usage {
        input_tokens: input,
        cache_read_input_tokens: cache_read,
        cache_creation_input_tokens: cache_create,
    }
}

const EPS: f64 = 1e-9;

#[test]
fn percent_sums_all_three_token_components() {
    // (100000 + 50000 + 50000) / 200000 = 100% — proves all three are summed.
    let p = context_percent(&usage(100_000, 50_000, 50_000), 200_000);
    assert!((p - 100.0).abs() < EPS, "got {p}");

    let p2 = context_percent(&usage(20_000, 20_000, 20_000), 200_000);
    assert!((p2 - 30.0).abs() < EPS, "got {p2}"); // 60000/200000 = 30%
}

#[test]
fn percent_clamps_to_0_and_100() {
    assert!((context_percent(&usage(0, 0, 0), 200_000) - 0.0).abs() < EPS);
    let over = context_percent(&usage(1_500_000, 0, 0), 1_000_000);
    assert!((over - 100.0).abs() < EPS, "must clamp at 100, got {over}");
}

#[test]
fn percent_with_1m_window() {
    // 500_000 / 1_000_000 = 50%. Caller provides the window explicitly.
    let p = context_percent(&usage(500_000, 0, 0), 1_000_000);
    assert!((p - 50.0).abs() < EPS, "got {p}");
}

#[test]
fn percent_window_zero_returns_zero() {
    assert!((context_percent(&usage(100_000, 0, 0), 0) - 0.0).abs() < EPS);
}

#[test]
fn percent_real_1m_session_example() {
    // A real 215k-usage 1M session — with the correct window, no heuristic needed.
    let p = context_percent(&usage(2, 214_355, 891), 1_000_000);
    assert!((p - 21.5).abs() < 0.1, "expected ~21.5%, got {p}");
}

#[test]
fn percent_200k_window_usage_stays_correct() {
    // Standard 200k session with high usage — not upgraded, just clamped.
    let p = context_percent(&usage(150_000, 0, 0), 200_000);
    assert!((p - 75.0).abs() < EPS);
}

// ─────────────────────────── model_friendly_name (unchanged) ─────────────────

#[test]
fn friendly_names_for_known_models() {
    assert_eq!(model_friendly_name("claude-opus-4-8"), "Opus 4.8");
    assert_eq!(model_friendly_name("claude-sonnet-4-6"), "Sonnet 4.6");
    assert_eq!(model_friendly_name("claude-haiku-4-5"), "Haiku 4.5");
    assert_eq!(model_friendly_name("claude-fable-5"), "Fable 5");
}

#[test]
fn friendly_name_ignores_dated_and_1m_suffixes() {
    assert_eq!(model_friendly_name("claude-haiku-4-5-20251001"), "Haiku 4.5");
    assert_eq!(model_friendly_name("claude-opus-4-8[1m]"), "Opus 4.8");
}

#[test]
fn unknown_model_degrades_to_raw_id() {
    assert_eq!(model_friendly_name("gpt-4o"), "gpt-4o");
    assert_eq!(model_friendly_name("claude-3-5-sonnet-20241022"), "claude-3-5-sonnet-20241022");
}

// ─────────────────────────── parse_context_output ────────────────────────────

/// The exact output we saw from `claude -p --resume "/context"` in the experiment.
const REAL_CONTEXT_OUTPUT: &str = "\
## Context Usage

**Model:** deepseek-v4-pro[1m]
**Tokens:** 170k / 1m (17%)

### Estimated usage by category
| Category | Tokens |
|----------|--------|
| System   | 1.5k   |
";

#[test]
fn parse_real_context_output() {
    let info = parse_context_output(REAL_CONTEXT_OUTPUT).expect("must parse real output");
    assert_eq!(info.model_alias, "deepseek-v4-pro[1m]");
    assert_eq!(info.window_size, 1_000_000);
}

#[test]
fn parse_200k_model() {
    let stdout = "## Context Usage\n\n**Model:** sonnet  \n**Tokens:** 80k / 200k (40%)";
    let info = parse_context_output(stdout).expect("must parse 200k model");
    assert_eq!(info.model_alias, "sonnet");
    assert_eq!(info.window_size, 200_000);
}

#[test]
fn parse_bare_number_window() {
    let stdout = "**Model:** claude-opus-4-8\n**Tokens:** 215248 / 200000 (107%)";
    let info = parse_context_output(stdout).expect("must parse bare-number window");
    assert_eq!(info.model_alias, "claude-opus-4-8");
    assert_eq!(info.window_size, 200_000);
}

#[test]
fn parse_returns_none_when_model_line_missing() {
    let stdout = "## Context Usage\n\n**Tokens:** 80k / 200k (40%)";
    assert!(parse_context_output(stdout).is_none());
}

#[test]
fn parse_returns_none_when_tokens_line_missing() {
    let stdout = "## Context Usage\n\n**Model:** sonnet";
    assert!(parse_context_output(stdout).is_none());
}

#[test]
fn parse_returns_none_on_garbled_input() {
    assert!(parse_context_output("not a context output").is_none());
}

// ─────────────────────────── model_changed ───────────────────────────────────

#[test]
fn model_changed_false_on_first_observation() {
    // None = we haven't seen any model yet (not a change, just initialisation).
    assert!(!model_changed(None, "claude-opus-4-8"));
}

#[test]
fn model_changed_false_when_same() {
    assert!(!model_changed(Some("claude-opus-4-8"), "claude-opus-4-8"));
}

#[test]
fn model_changed_true_when_different() {
    assert!(model_changed(Some("claude-opus-4-8"), "claude-sonnet-4-6"));
}

// ─────────────────────────── resolve_window ───────────────────────────────────

#[test]
fn resolve_uses_cached_window_when_present() {
    // L1 cache hit — always use the cached window, no heuristic.
    let w = resolve_window(Some(1_000_000), &usage(50_000, 0, 0), "claude-opus-4-8");
    assert_eq!(w, 1_000_000, "cached window must be used directly");
}

#[test]
fn resolve_l3_upgrades_when_usage_exceeds_default_window() {
    // No cache, usage > 200k → L3 heuristic upgrades to 1M.
    let w = resolve_window(None, &usage(2, 214_355, 891), "claude-opus-4-8");
    assert_eq!(w, 1_000_000);
}

#[test]
fn resolve_l3_stays_at_200k_when_usage_is_low() {
    // No cache, usage < 200k → standard 200k window.
    let w = resolve_window(None, &usage(50_000, 0, 0), "claude-opus-4-8");
    assert_eq!(w, 200_000);
}

#[test]
fn resolve_l3_respects_explicit_1m_in_model_id() {
    // model_id contains [1m] → context_window returns 1M directly.
    let w = resolve_window(None, &usage(50_000, 0, 0), "claude-opus-4-8[1m]");
    assert_eq!(w, 1_000_000);
}

// ─────────────────────────── CachedContext ────────────────────────────────────

#[test]
fn cached_context_defaults() {
    let c = CachedContext {
        model_alias: "opus[1m]".into(),
        window_size: 1_000_000,
        last_transcript_model: "claude-opus-4-8".into(),
    };
    assert_eq!(c.window_size, 1_000_000);
    assert_eq!(c.model_alias, "opus[1m]");
}

//! Slice 3: deriving context % + model from a transcript tail. Pure functions,
//! mirroring the real Claude Code transcript shape
//! ({ type:"assistant", message:{ model, usage:{ input_tokens, ... } } }).

use claude_copet_lib::session::{
    context_percent, context_window, latest_usage_and_model, model_friendly_name, Usage,
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

// ─────────────────────────── context_window ──────────────────────────────────

#[test]
fn window_defaults_and_1m_variants() {
    assert_eq!(context_window("claude-opus-4-8"), 200_000);
    assert_eq!(context_window("claude-sonnet-4-6"), 200_000);
    assert_eq!(context_window("claude-opus-4-8[1m]"), 1_000_000);
    assert_eq!(context_window("claude-sonnet-4-6[1m]"), 1_000_000);
    assert_eq!(context_window("something-unknown"), 200_000, "unknown falls back to default");
}

// ─────────────────────────── context_percent ─────────────────────────────────

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
    let p = context_percent(&usage(100_000, 50_000, 50_000), "claude-opus-4-8");
    assert!((p - 100.0).abs() < EPS, "got {p}");

    let p2 = context_percent(&usage(20_000, 20_000, 20_000), "claude-opus-4-8");
    assert!((p2 - 30.0).abs() < EPS, "got {p2}"); // 60000/200000 = 30%
}

#[test]
fn percent_clamps_to_0_and_100() {
    assert!((context_percent(&usage(0, 0, 0), "claude-opus-4-8") - 0.0).abs() < EPS);
    let over = context_percent(&usage(500_000, 0, 0), "claude-opus-4-8");
    assert!((over - 100.0).abs() < EPS, "must clamp at 100, got {over}");
}

#[test]
fn percent_uses_the_1m_window_for_1m_models() {
    // 500_000 / 1_000_000 = 50% (would be clamped to 100 against a 200k window).
    let p = context_percent(&usage(500_000, 0, 0), "claude-opus-4-8[1m]");
    assert!((p - 50.0).abs() < EPS, "got {p}");
}

// ─────────────────────────── model_friendly_name ─────────────────────────────

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

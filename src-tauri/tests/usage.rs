//! Pure functions for the HUD usage-limits feature (the ticket
//! `usage-limits-hud`): parse `claude -p "/usage"` stdout into the 5h/7d
//! windows, and the auto-poll / manual-throttle / back-off / fetch-fold
//! decisions. No IO/clock here — the watcher feeds these and emits the result.

use std::time::Duration;

use claude_copet_lib::session::{
    apply_usage_fetch, parse_usage_output, usage_manual_allowed, usage_should_auto_poll,
    UsageLimits, USAGE_NO_LIMIT_BACKOFF,
};

// ─────────────────────────── parse_usage_output ──────────────────────────────

/// The EXACT stdout captured from `claude -p --output-format text "/usage"` on a
/// real Claude subscription (trimmed of nothing material). The parser must pull
/// the two windows out of this and ignore everything else.
const REAL_USAGE_OUTPUT: &str = "\
You are currently using your subscription to power your Claude Code usage

Current session: 31% used · resets Jun 26 at 11:59pm (Asia/Shanghai)
Current week (all models): 77% used · resets Jun 30 at 3pm (Asia/Shanghai)

What's contributing to your limits usage?
Approximate, based on local sessions on this machine — does not include other devices or claude.ai. Behaviors are independent characteristics, not a breakdown.

Last 24h · 1776 requests · 18 sessions
  85% of your usage came from subagent-heavy sessions
  71% of your usage was at >150k context
  Top skills: /weee:dev 8%, /weee:ticket 2%, /weee:discuss 1%

Last 7d · 4230 requests · 45 sessions
  71% of your usage came from subagent-heavy sessions
";

#[test]
fn parses_both_windows_from_real_output() {
    let u = parse_usage_output(REAL_USAGE_OUTPUT).expect("must parse the real /usage output");
    assert_eq!(u.session_percent, 31);
    assert_eq!(u.week_percent, 77);
    // Reset strings are carried verbatim (after "resets "), trimmed — the
    // frontend decides how compactly to render them.
    assert_eq!(u.session_reset, "Jun 26 at 11:59pm (Asia/Shanghai)");
    assert_eq!(u.week_reset, "Jun 30 at 3pm (Asia/Shanghai)");
}

#[test]
fn ignores_the_contributing_breakdown_lines() {
    // The breakdown lines also contain "% of your usage" — they must NOT be
    // mistaken for the session/week windows.
    let u = parse_usage_output(REAL_USAGE_OUTPUT).expect("must parse");
    assert_eq!(u.session_percent, 31, "breakdown % lines must not leak in");
    assert_eq!(u.week_percent, 77);
}

#[test]
fn matches_week_line_with_or_without_all_models_qualifier() {
    let without = "\
Current session: 5% used · resets tomorrow at 9am
Current week: 12% used · resets Monday at 9am";
    let u = parse_usage_output(without).expect("must match 'Current week:' without qualifier");
    assert_eq!(u.session_percent, 5);
    assert_eq!(u.week_percent, 12);
    assert_eq!(u.week_reset, "Monday at 9am");
}

#[test]
fn returns_none_when_session_line_missing() {
    let only_week = "Current week (all models): 77% used · resets Jun 30 at 3pm";
    assert!(parse_usage_output(only_week).is_none());
}

#[test]
fn returns_none_when_week_line_missing() {
    let only_session = "Current session: 31% used · resets Jun 26 at 11:59pm";
    assert!(parse_usage_output(only_session).is_none());
}

#[test]
fn returns_none_on_non_claude_api_key_output() {
    // Non-Claude / API-key setups have no 5h/7d limits — /usage prints a
    // different message with no session/week lines. Parser must return None
    // (no panic), which the watcher maps to "hide the usage block".
    // NOTE: stand-in shape — we lack a captured real non-Claude sample.
    let api_key = "\
You are using a custom API key to power your Claude Code usage.
Usage is billed directly to your Anthropic Console account.
Run /login to switch to a subscription.";
    assert!(parse_usage_output(api_key).is_none());
}

#[test]
fn returns_none_on_empty_or_garbled() {
    assert!(parse_usage_output("").is_none());
    assert!(parse_usage_output("totally unrelated text").is_none());
}

#[test]
fn parses_percent_defensively_from_decimal() {
    // Integers are the observed case; a decimal must not panic — take the
    // integer part.
    let s = "\
Current session: 8.5% used · resets later
Current week: 0% used · resets Monday";
    let u = parse_usage_output(s).expect("decimal percent must not break parsing");
    assert_eq!(u.session_percent, 8);
    assert_eq!(u.week_percent, 0);
}

// ── Percent tokens with a comparator/approx prefix (coverage-adversary Critical) ──
// These are PLAUSIBLE-VARIANT inputs (not captured): a fresh window can read
// "<1%" and a near-cap one ">99%". A naive `"<1".parse::<u8>()` errors and drops
// the WHOLE struct → blank HUD for a real Claude session. The numeric part must
// be extracted instead, ignoring a leading `<`, `>`, or `~`.

#[test]
fn parses_less_than_one_percent_without_dropping_the_window() {
    let s = "\
Current session: <1% used · resets Jun 26 at 11:59pm (Asia/Shanghai)
Current week (all models): 3% used · resets Jun 30 at 3pm (Asia/Shanghai)";
    let u = parse_usage_output(s).expect("'<1%' must still parse, not blank the HUD");
    assert_eq!(u.session_percent, 1);
    assert_eq!(u.week_percent, 3);
}

#[test]
fn parses_greater_than_99_percent() {
    let s = "\
Current session: >99% used · resets Jun 26 at 11:59pm
Current week: 100% used · resets Jun 30 at 3pm";
    let u = parse_usage_output(s).expect("'>99%' / '100%' must parse");
    assert_eq!(u.session_percent, 99);
    assert_eq!(
        u.week_percent, 100,
        "100% is a valid u8 and must round-trip"
    );
}

#[test]
fn parses_approximate_percent_marker() {
    let s = "\
Current session: ~5% used · resets soon
Current week: ~12% used · resets Monday";
    let u = parse_usage_output(s).expect("'~5%' approximate marker must parse");
    assert_eq!(u.session_percent, 5);
    assert_eq!(u.week_percent, 12);
}

#[test]
fn matches_session_line_with_a_parenthetical_qualifier() {
    // The week line already carries "(all models)"; the session line could gain a
    // similar qualifier in a future CLI version. Matching must not require the
    // bare "Current session:" with a colon immediately after.
    let s = "\
Current session (5h): 31% used · resets Jun 26 at 11:59pm
Current week (all models): 77% used · resets Jun 30 at 3pm";
    let u = parse_usage_output(s).expect("a parenthetical on the session line must still match");
    assert_eq!(u.session_percent, 31);
    assert_eq!(u.week_percent, 77);
}

// ─────────────────────────── usage_should_auto_poll ──────────────────────────

#[test]
fn auto_poll_fires_on_startup_before_any_fetch() {
    // elapsed=None => no fetch yet => poll now (startup fetch), streak 0.
    assert!(usage_should_auto_poll(None, Duration::from_secs(300), 0));
}

#[test]
fn auto_poll_waits_until_interval_elapses() {
    let interval = Duration::from_secs(300);
    assert!(!usage_should_auto_poll(
        Some(Duration::from_secs(100)),
        interval,
        0
    ));
    assert!(usage_should_auto_poll(
        Some(Duration::from_secs(300)),
        interval,
        0
    ));
    assert!(usage_should_auto_poll(
        Some(Duration::from_secs(301)),
        interval,
        0
    ));
}

#[test]
fn auto_poll_backs_off_after_consecutive_no_limit_results() {
    let interval = Duration::from_secs(300);
    // One no-limit result: still polling.
    assert!(usage_should_auto_poll(
        Some(interval),
        interval,
        USAGE_NO_LIMIT_BACKOFF - 1
    ));
    // Reached the back-off threshold: auto-poll stops even though interval passed.
    assert!(!usage_should_auto_poll(
        Some(interval),
        interval,
        USAGE_NO_LIMIT_BACKOFF
    ));
    assert!(!usage_should_auto_poll(
        None,
        interval,
        USAGE_NO_LIMIT_BACKOFF
    ));
}

// ─────────────────────────── usage_manual_allowed ────────────────────────────

#[test]
fn manual_allowed_when_no_prior_fetch() {
    assert!(usage_manual_allowed(None, Duration::from_secs(30)));
}

#[test]
fn manual_throttled_within_min_gap_allowed_after() {
    let gap = Duration::from_secs(30);
    assert!(
        !usage_manual_allowed(Some(Duration::from_secs(5)), gap),
        "rapid re-click dropped"
    );
    assert!(!usage_manual_allowed(Some(Duration::from_secs(29)), gap));
    assert!(
        usage_manual_allowed(Some(Duration::from_secs(30)), gap),
        "allowed at the gap"
    );
    assert!(usage_manual_allowed(Some(Duration::from_secs(60)), gap));
}

// ─────────────────────────── apply_usage_fetch ───────────────────────────────

fn sample() -> UsageLimits {
    UsageLimits {
        session_percent: 31,
        session_reset: "Jun 26 at 11:59pm".into(),
        week_percent: 77,
        week_reset: "Jun 30 at 3pm".into(),
    }
}

#[test]
fn fetch_ok_some_replaces_payload_and_resets_streak() {
    let (payload, streak) = apply_usage_fetch(None, 1, Ok(Some(sample())));
    assert_eq!(payload, Some(sample()));
    assert_eq!(streak, 0, "a real result clears the no-limit streak");
}

#[test]
fn fetch_ok_none_clears_payload_and_increments_streak() {
    // Non-Claude: successful fetch, no limit lines → hide (None) + count toward back-off.
    let (payload, streak) = apply_usage_fetch(Some(sample()), 0, Ok(None));
    assert_eq!(payload, None, "no-limits result hides the block");
    assert_eq!(streak, 1);
}

#[test]
fn fetch_err_keeps_previous_payload_and_streak() {
    // Transient failure must NOT blank a good card and must NOT count as no-limits.
    let (payload, streak) = apply_usage_fetch(Some(sample()), 0, Err(()));
    assert_eq!(
        payload,
        Some(sample()),
        "transient error preserves last good values"
    );
    assert_eq!(streak, 0, "an error is not a no-limits signal");
}

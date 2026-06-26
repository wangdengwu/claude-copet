//! The `UsageClient` seam (mirrors `ContextClient`): lets the watcher fetch
//! `/usage` without the real CLI. These tests drive the full pure pipeline a
//! stub client feeds — fetch → `parse_usage_output` → `apply_usage_fetch` —
//! proving success/no-limits/transient outcomes compose as specified.

use std::sync::{Arc, Mutex};

use claude_copet_lib::session::{apply_usage_fetch, parse_usage_output, UsageClient, UsageLimits};

/// A stub: returns canned stdout, or `Err` when constructed with an error flag.
struct StubUsageClient {
    canned: Option<String>,
    calls: Arc<Mutex<u32>>,
}

impl StubUsageClient {
    fn ok(stdout: &str) -> Self {
        Self {
            canned: Some(stdout.to_string()),
            calls: Arc::new(Mutex::new(0)),
        }
    }
    fn err() -> Self {
        Self {
            canned: None,
            calls: Arc::new(Mutex::new(0)),
        }
    }
    fn calls(&self) -> u32 {
        *self.calls.lock().unwrap()
    }
}

impl UsageClient for StubUsageClient {
    fn fetch_usage(&self) -> Result<String, ()> {
        *self.calls.lock().unwrap() += 1;
        self.canned.clone().ok_or(())
    }
}

const CLAUDE_USAGE: &str = "\
You are currently using your subscription to power your Claude Code usage

Current session: 31% used · resets Jun 26 at 11:59pm (Asia/Shanghai)
Current week (all models): 77% used · resets Jun 30 at 3pm (Asia/Shanghai)
";

const NON_CLAUDE_USAGE: &str = "\
You are using a custom API key to power your Claude Code usage.
Run /login to switch to a subscription.";

/// One full fetch cycle: client stdout → parse → fold into displayed payload.
fn cycle(
    client: &dyn UsageClient,
    prev: Option<UsageLimits>,
    streak: u32,
) -> (Option<UsageLimits>, u32) {
    let outcome = client.fetch_usage().map(|s| parse_usage_output(&s));
    apply_usage_fetch(prev, streak, outcome)
}

#[test]
fn successful_claude_fetch_surfaces_both_windows() {
    let client = StubUsageClient::ok(CLAUDE_USAGE);
    let (payload, streak) = cycle(&client, None, 0);
    let u = payload.expect("a Claude /usage must surface a payload");
    assert_eq!(u.session_percent, 31);
    assert_eq!(u.week_percent, 77);
    assert_eq!(streak, 0);
    assert_eq!(client.calls(), 1);
}

#[test]
fn non_claude_fetch_clears_payload_and_counts_toward_backoff() {
    let prev = Some(UsageLimits {
        session_percent: 31,
        session_reset: "x".into(),
        week_percent: 77,
        week_reset: "y".into(),
    });
    let client = StubUsageClient::ok(NON_CLAUDE_USAGE);
    let (payload, streak) = cycle(&client, prev, 0);
    assert_eq!(payload, None, "no-limits output hides the block");
    assert_eq!(streak, 1);
}

#[test]
fn transient_error_preserves_previous_payload() {
    let prev = Some(UsageLimits {
        session_percent: 31,
        session_reset: "x".into(),
        week_percent: 77,
        week_reset: "y".into(),
    });
    let client = StubUsageClient::err();
    let (payload, streak) = cycle(&client, prev.clone(), 0);
    assert_eq!(
        payload, prev,
        "a transient CLI error keeps the last good values"
    );
    assert_eq!(streak, 0, "error is not a no-limits signal");
    assert_eq!(client.calls(), 1);
}

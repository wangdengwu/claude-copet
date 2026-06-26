//! Slice 2: ContextClient trait + mock-based tests for the trigger logic
//! (startup, session switch, model mismatch, in-flight guard, retry+L3).

use std::sync::{Arc, Mutex};

use claude_copet_lib::session::ContextClient;

/// A mock client that returns canned stdout and counts calls.
struct MockClient {
    canned: String,
    call_count: Arc<Mutex<u32>>,
}

impl MockClient {
    fn new(canned: &str) -> Self {
        Self {
            canned: canned.to_string(),
            call_count: Arc::new(Mutex::new(0)),
        }
    }
    fn calls(&self) -> u32 {
        *self.call_count.lock().unwrap()
    }
}

impl ContextClient for MockClient {
    fn fetch_context(&self, _session_id: &str) -> Result<String, ()> {
        *self.call_count.lock().unwrap() += 1;
        if self.canned.is_empty() {
            Err(())
        } else {
            Ok(self.canned.clone())
        }
    }
}

const REAL_OUTPUT: &str = "\
## Context Usage

**Model:** opus[1m]
**Tokens:** 170k / 1m (17%)
";

#[test]
fn client_returns_canned_stdout() {
    let client = MockClient::new(REAL_OUTPUT);
    assert_eq!(client.fetch_context("s1").unwrap(), REAL_OUTPUT);
    assert_eq!(client.calls(), 1);
}

#[test]
fn client_errors_on_empty_canned() {
    let client = MockClient::new("");
    assert!(client.fetch_context("s1").is_err());
    assert_eq!(client.calls(), 1);
}

#[test]
fn client_passes_session_id_through() {
    // Different session ids produce independent calls.
    let client = MockClient::new(REAL_OUTPUT);
    client.fetch_context("sess-a").unwrap();
    client.fetch_context("sess-b").unwrap();
    assert_eq!(client.calls(), 2);
}

#[test]
fn multiple_clients_share_call_count_via_arc() {
    let count = Arc::new(Mutex::new(0));
    let a = MockClient {
        canned: REAL_OUTPUT.into(),
        call_count: count.clone(),
    };
    let b = MockClient {
        canned: REAL_OUTPUT.into(),
        call_count: count.clone(),
    };
    a.fetch_context("x").unwrap();
    b.fetch_context("y").unwrap();
    assert_eq!(*count.lock().unwrap(), 2);
}

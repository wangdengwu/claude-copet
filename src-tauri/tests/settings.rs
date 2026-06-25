//! Failing tests for seam 5 of slice 5 (Settings persistence + API-key resolution).
//!
//! Required public API (implementer must expose these):
//!
//!   // in claude_copet_lib::settings
//!   #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
//!   pub struct Settings {
//!       pub llm_enabled: bool,
//!       pub provider: String,          // e.g. "anthropic"
//!       pub model: String,             // e.g. "claude-haiku-4-5"
//!       pub api_key: String,           // stored locally, never committed
//!   }
//!
//!   impl Settings {
//!       pub fn default() -> Self;
//!       /// Load from an explicit path (for tests / injection). Returns default on
//!       /// missing file; propagates IO/JSON errors as Err(String).
//!       pub fn load_from(path: &std::path::Path) -> Result<Self, String>;
//!       /// Persist to an explicit path (parent dir created if absent).
//!       pub fn save_to(&self, path: &std::path::Path) -> Result<(), String>;
//!       /// Resolve the active API key: Settings.api_key if non-empty, else the
//!       /// `ANTHROPIC_API_KEY` environment variable, else None.
//!       /// Takes the env value as a parameter so callers can inject it (no global env side-effect).
//!       pub fn resolve_api_key(&self, env_key: Option<&str>) -> Option<String>;
//!   }

use std::path::PathBuf;
use tempfile::TempDir;

use claude_copet_lib::settings::Settings;

// ─────────────────────────── helpers ────────────────────────────────────────

fn temp_settings_path(dir: &TempDir) -> PathBuf {
    dir.path().join("settings.json")
}

// ─────────────────────────── seam 5 — load / save round-trip ────────────────

/// A newly saved Settings can be loaded back unchanged.
#[test]
fn round_trip_preserves_all_fields() {
    let dir = TempDir::new().expect("temp dir");
    let path = temp_settings_path(&dir);

    let original = Settings {
        llm_enabled: true,
        provider: "anthropic".to_string(),
        model: "claude-haiku-4-5".to_string(),
        api_key: "sk-test-round-trip".to_string(),
    };

    original.save_to(&path).expect("save must succeed");
    let loaded = Settings::load_from(&path).expect("load must succeed");

    assert_eq!(original, loaded, "round-trip must preserve all fields");
}

/// `load_from` on a non-existent path returns the default (no error).
#[test]
fn load_from_missing_path_returns_default() {
    let dir = TempDir::new().expect("temp dir");
    let path = dir.path().join("does_not_exist.json");

    let result = Settings::load_from(&path);
    assert!(result.is_ok(), "missing file must return Ok(default), not Err");
    let loaded = result.unwrap();
    let default = Settings::default();
    assert_eq!(loaded, default, "missing file must yield the default Settings");
}

/// save_to creates parent directories automatically.
#[test]
fn save_to_creates_parent_dirs() {
    let dir = TempDir::new().expect("temp dir");
    let nested = dir.path().join("a").join("b").join("c").join("settings.json");

    let s = Settings::default();
    s.save_to(&nested).expect("save must create parent dirs and succeed");
    assert!(nested.exists(), "settings file must exist after save");
}

/// Toggling llm_enabled survives a save/load cycle.
#[test]
fn toggle_llm_enabled_persists() {
    let dir = TempDir::new().expect("temp dir");
    let path = temp_settings_path(&dir);

    let mut s = Settings::default();
    s.llm_enabled = true;
    s.save_to(&path).unwrap();
    let loaded = Settings::load_from(&path).unwrap();
    assert!(loaded.llm_enabled, "llm_enabled=true must persist");

    s.llm_enabled = false;
    s.save_to(&path).unwrap();
    let loaded2 = Settings::load_from(&path).unwrap();
    assert!(!loaded2.llm_enabled, "llm_enabled=false must persist after re-save");
}

// ─────────────────────────── seam 5 — API-key resolution ────────────────────

/// Settings.api_key is non-empty → it wins over the env var.
#[test]
fn api_key_in_settings_wins_over_env() {
    let s = Settings {
        llm_enabled: true,
        provider: "anthropic".to_string(),
        model: "claude-haiku-4-5".to_string(),
        api_key: "settings-key".to_string(),
    };
    let resolved = s.resolve_api_key(Some("env-key"));
    assert_eq!(
        resolved,
        Some("settings-key".to_string()),
        "settings api_key must take precedence over env"
    );
}

/// Settings.api_key is empty → falls back to the env var.
#[test]
fn empty_settings_key_falls_back_to_env() {
    let s = Settings {
        llm_enabled: true,
        provider: "anthropic".to_string(),
        model: "claude-haiku-4-5".to_string(),
        api_key: "".to_string(),
    };
    let resolved = s.resolve_api_key(Some("env-key"));
    assert_eq!(
        resolved,
        Some("env-key".to_string()),
        "empty settings key must fall back to the env var"
    );
}

/// Both empty → None.
#[test]
fn no_key_anywhere_returns_none() {
    let s = Settings {
        llm_enabled: false,
        provider: "anthropic".to_string(),
        model: "claude-haiku-4-5".to_string(),
        api_key: "".to_string(),
    };
    let resolved = s.resolve_api_key(None);
    assert_eq!(resolved, None, "no key in settings or env must yield None");
}

/// Only env var present.
#[test]
fn only_env_key_resolves_correctly() {
    let s = Settings {
        llm_enabled: true,
        provider: "anthropic".to_string(),
        model: "claude-haiku-4-5".to_string(),
        api_key: "".to_string(),
    };
    let resolved = s.resolve_api_key(Some("only-env-key"));
    assert_eq!(resolved, Some("only-env-key".to_string()));
}

/// Whitespace-only api_key in settings is treated as empty → falls back to env.
#[test]
fn whitespace_only_settings_key_treated_as_empty() {
    let s = Settings {
        llm_enabled: true,
        provider: "anthropic".to_string(),
        model: "claude-haiku-4-5".to_string(),
        api_key: "   ".to_string(),
    };
    let resolved = s.resolve_api_key(Some("env-key"));
    assert_eq!(
        resolved,
        Some("env-key".to_string()),
        "whitespace-only settings key must be treated as absent"
    );
}

// ─────────────────────────── default shape ──────────────────────────────────

/// The default Settings must have the right model id and be LLM-disabled.
#[test]
fn default_settings_shape() {
    let d = Settings::default();
    assert!(!d.llm_enabled, "LLM must be off by default");
    assert_eq!(d.model, "claude-haiku-4-5", "default model must be claude-haiku-4-5");
    assert_eq!(d.provider, "claude-cli", "default provider must be claude-cli");
    assert!(d.api_key.is_empty(), "default api_key must be empty");
}

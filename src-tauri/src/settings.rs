//! Persisted user settings for the pet (slice 5). Stored in
//! `$HOME/.claude-copet/settings.json` — outside the repo so the API key
//! is never committed.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    pub llm_enabled: bool,
    pub provider: String,
    pub model: String,
    /// API key stored locally; never committed. Empty string means "not set".
    pub api_key: String,
}

impl Settings {
    pub fn default() -> Self {
        Settings {
            llm_enabled: false,
            provider: "claude-cli".to_string(),
            model: "claude-haiku-4-5".to_string(),
            api_key: String::new(),
        }
    }

    /// Load settings from an explicit path (useful for tests / DI).
    /// Returns `Ok(default)` when the file does not exist.
    /// Propagates IO or JSON errors as `Err(String)`.
    pub fn load_from(path: &Path) -> Result<Self, String> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let bytes = fs::read(path).map_err(|e| e.to_string())?;
        serde_json::from_slice(&bytes).map_err(|e| e.to_string())
    }

    /// Persist to an explicit path, creating any missing parent directories.
    pub fn save_to(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let json = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        fs::write(path, json).map_err(|e| e.to_string())
    }

    /// Resolve the active API key:
    /// 1. `self.api_key` if non-empty (after trimming whitespace).
    /// 2. `env_key` parameter (caller injects; avoids global env side-effects in tests).
    /// 3. `None`.
    pub fn resolve_api_key(&self, env_key: Option<&str>) -> Option<String> {
        let trimmed = self.api_key.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
        env_key.map(|s| s.to_string())
    }
}

//! Persisted user settings. Stored in `$HOME/.claude-copet/settings.json`.
//!
//! The HUD product no longer configures an LLM voice, so this struct currently
//! carries no fields — `get_settings` / `set_settings` are kept as stable seams
//! for future preferences. Unknown keys in an existing file (e.g. an old install's
//! `llm_enabled` / `api_key`) are ignored on load.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

fn default_usage_refresh_minutes() -> u8 {
    5
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default = "default_usage_refresh_minutes")]
    pub usage_refresh_minutes: u8,
    /// When `true`, the auto-connect-on-startup logic is skipped.
    /// Defaults to `false` so fresh installs connect automatically.
    #[serde(default)]
    pub hooks_opt_out: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            usage_refresh_minutes: 5,
            hooks_opt_out: false,
        }
    }
}

impl Settings {
    /// Returns the refresh interval clamped to the offered set {5, 10, 15}.
    /// Any other persisted value falls back to 5.
    pub fn effective_refresh_minutes(&self) -> u8 {
        match self.usage_refresh_minutes {
            5 | 10 | 15 => self.usage_refresh_minutes,
            _ => 5,
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
}

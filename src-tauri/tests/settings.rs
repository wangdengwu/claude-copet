//! Settings persistence. The HUD product carries no settings fields today;
//! these tests pin the load/save seam and tolerance of an old install's file.

use std::path::PathBuf;
use tempfile::TempDir;

use claude_copet_lib::settings::Settings;

fn temp_settings_path(dir: &TempDir) -> PathBuf {
    dir.path().join("settings.json")
}

/// A saved Settings can be loaded back unchanged.
#[test]
fn round_trip_preserves_settings() {
    let dir = TempDir::new().expect("temp dir");
    let path = temp_settings_path(&dir);

    let original = Settings::default();
    original.save_to(&path).expect("save must succeed");
    let loaded = Settings::load_from(&path).expect("load must succeed");

    assert_eq!(original, loaded, "round-trip must preserve settings");
}

/// `load_from` on a non-existent path returns the default (no error).
#[test]
fn load_from_missing_path_returns_default() {
    let dir = TempDir::new().expect("temp dir");
    let path = dir.path().join("does_not_exist.json");

    let result = Settings::load_from(&path);
    assert!(result.is_ok(), "missing file must return Ok(default), not Err");
    assert_eq!(result.unwrap(), Settings::default());
}

/// save_to creates parent directories automatically.
#[test]
fn save_to_creates_parent_dirs() {
    let dir = TempDir::new().expect("temp dir");
    let nested = dir.path().join("a").join("b").join("c").join("settings.json");

    Settings::default().save_to(&nested).expect("save must create parent dirs and succeed");
    assert!(nested.exists(), "settings file must exist after save");
}

/// An old install's settings file (with now-removed keys) still loads — unknown
/// fields are ignored, so re-running Connect never trips over a stale file.
#[test]
fn old_install_file_with_extra_keys_loads_ok() {
    let dir = TempDir::new().expect("temp dir");
    let path = temp_settings_path(&dir);
    std::fs::write(
        &path,
        br#"{"llm_enabled":true,"provider":"anthropic","model":"x","api_key":"sk-old"}"#,
    )
    .unwrap();

    let loaded = Settings::load_from(&path).expect("legacy file must still load");
    assert_eq!(loaded, Settings::default(), "removed keys must be ignored");
}

//! Internationalization support: locale enum and menu label string tables.

use serde::{Deserialize, Serialize};

/// The user's chosen display language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Locale {
    En,
    Zh,
}

impl Default for Locale {
    fn default() -> Self {
        Locale::En
    }
}

/// Keys for every native right-click menu label that requires localization.
#[derive(Debug, Clone, Copy)]
pub enum MenuKey {
    Refresh,
    Settings,
    Language,
    EnglishName,
    ChineseName,
    Quit,
}

/// Return the static menu label for `key` in `locale`.
pub fn menu_label(locale: Locale, key: MenuKey) -> &'static str {
    match (locale, key) {
        // ── English ──────────────────────────────────────────────────────────
        (Locale::En, MenuKey::Refresh) => "Refresh usage",
        (Locale::En, MenuKey::Settings) => "Settings",
        (Locale::En, MenuKey::Language) => "Language",
        (Locale::En, MenuKey::Quit) => "Quit",
        // Endonyms: same in both locales
        (_, MenuKey::EnglishName) => "English",
        (_, MenuKey::ChineseName) => "中文",
        // ── Chinese ──────────────────────────────────────────────────────────
        (Locale::Zh, MenuKey::Refresh) => "刷新用量",
        (Locale::Zh, MenuKey::Settings) => "设置",
        (Locale::Zh, MenuKey::Language) => "语言",
        (Locale::Zh, MenuKey::Quit) => "退出",
    }
}

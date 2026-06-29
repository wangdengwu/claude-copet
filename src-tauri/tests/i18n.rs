//! The Rust-side i18n string table for native-menu labels. Pins the menu
//! vocabulary in both locales so a translation gap (a key empty in one locale)
//! is caught at test time, not by a user staring at a blank menu item.

use claude_copet_lib::i18n::{menu_label, Locale, MenuKey};

/// Every menu key resolves to a non-empty string in BOTH locales — no missing
/// translation can ship.
#[test]
fn every_menu_key_resolves_non_empty_in_both_locales() {
    let keys = [
        MenuKey::Refresh,
        MenuKey::Settings,
        MenuKey::Language,
        MenuKey::EnglishName,
        MenuKey::ChineseName,
        MenuKey::Quit,
    ];
    for key in keys {
        for locale in [Locale::En, Locale::Zh] {
            assert!(
                !menu_label(locale, key).is_empty(),
                "menu_label({:?}, {:?}) must not be empty",
                locale,
                key
            );
        }
    }
}

/// English labels match today's hardcoded menu exactly, so the default-locale UX
/// is byte-identical to the pre-i18n build.
#[test]
fn english_labels_match_the_pre_i18n_menu() {
    assert_eq!(menu_label(Locale::En, MenuKey::Refresh), "Refresh usage");
    assert_eq!(menu_label(Locale::En, MenuKey::Settings), "Settings");
    assert_eq!(menu_label(Locale::En, MenuKey::Language), "Language");
    assert_eq!(menu_label(Locale::En, MenuKey::Quit), "Quit");
}

/// Chinese labels are the agreed translations.
#[test]
fn chinese_labels_are_translated() {
    assert_eq!(menu_label(Locale::Zh, MenuKey::Refresh), "刷新用量");
    assert_eq!(menu_label(Locale::Zh, MenuKey::Settings), "设置");
    assert_eq!(menu_label(Locale::Zh, MenuKey::Language), "语言");
    assert_eq!(menu_label(Locale::Zh, MenuKey::Quit), "退出");
}

/// The two language-choice items are shown in their OWN script regardless of the
/// active UI locale — an English speaker still sees "中文", a Chinese speaker
/// still sees "English" — so a user can always recognize the language they want.
#[test]
fn language_names_are_endonyms_in_both_locales() {
    for locale in [Locale::En, Locale::Zh] {
        assert_eq!(menu_label(locale, MenuKey::EnglishName), "English");
        assert_eq!(menu_label(locale, MenuKey::ChineseName), "中文");
    }
}

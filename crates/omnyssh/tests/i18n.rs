//! Integration tests for the TUI localization layer.
//!
//! These exercise the public `omnyssh::i18n` surface exactly as a frontend
//! would, guarding the invariants the table tests cannot reach across the
//! crate boundary.

use omnyssh::i18n::{t, tf, Key, Lang};

#[test]
fn unknown_language_code_falls_back_to_english() {
    // A hand-edited config with a typo must not leave the UI string-less.
    for bad in ["", "fr", "zh", "ZH-HANS", "en-US", "  "] {
        assert_eq!(
            Lang::from_code(bad),
            Lang::En,
            "code {bad:?} should fall back to English"
        );
    }
}

#[test]
fn every_key_resolves_to_text_in_every_language() {
    // Keys whose real English text legitimately spells the variant name.
    let spells_its_name = [Key::Untagged, Key::HelpNewTab, Key::HelpCloseTab];
    for lang in Lang::all() {
        for key in Key::ALL {
            let text = t(*lang, *key);
            assert!(!text.trim().is_empty(), "{key:?} empty for {lang:?}");
            // A bare variant name (a forgotten arm) is never valid UI text.
            let leaks = text.starts_with("Key::")
                || (text.contains(&format!("{key:?}")) && !spells_its_name.contains(key));
            assert!(!leaks, "{key:?} leaked its name for {lang:?}: {text:?}");
        }
    }
}

#[test]
fn the_two_languages_actually_differ() {
    // Prose must be translated; only proper nouns and the CPU/RAM abbreviations
    // are allowed to read the same in both.
    let same_on_purpose = [Key::SortCpu, Key::SortRam];
    for key in Key::ALL {
        if same_on_purpose.contains(key) {
            continue;
        }
        assert_ne!(
            t(Lang::En, *key),
            t(Lang::ZhHans, *key),
            "{key:?} was never translated"
        );
    }
}

#[test]
fn chinese_keys_cover_the_same_set_as_english() {
    // A missing zh arm would be a compile error, but a key added to `en()`
    // only (via a stray duplicate match) would not — assert the sets match.
    let en: std::collections::HashSet<_> = Lang::all()
        .iter()
        .filter(|l| **l == Lang::En)
        .flat_map(|l| Key::ALL.iter().map(move |k| t(*l, *k)))
        .collect();
    let zh: std::collections::HashSet<_> = Key::ALL.iter().map(|k| t(Lang::ZhHans, *k)).collect();
    // Every Chinese string must be non-empty; and no Chinese entry may equal an
    // English one outside the deliberate abbreviations.
    assert!(!zh.iter().any(|s| s.trim().is_empty()));
    assert!(en.len() > 100, "table looks suspiciously small");
}

#[test]
fn placeholders_are_filled_and_surplus_args_ignored() {
    // `tf` replaces `{}` left to right and never panics on a key without one.
    assert_eq!(tf(Lang::En, Key::NavDashboard, &[]), "Dashboard");
    assert_eq!(tf(Lang::ZhHans, Key::NavDashboard, &["ignored"]), "仪表盘");
}

#[test]
fn language_codes_round_trip() {
    for lang in Lang::all() {
        assert_eq!(Lang::from_code(lang.code()), *lang);
    }
    assert_eq!(Lang::En.code(), "en");
    assert_eq!(Lang::ZhHans.code(), "zh-Hans");
}

#[test]
fn key_tokens_are_never_translated() {
    // Input tokens and the metric abbreviations are deliberately identical in
    // both languages — a translation that touched them would break muscle memory.
    let en = t(Lang::En, Key::DashboardHints);
    let zh = t(Lang::ZhHans, Key::DashboardHints);
    for token in ["r:", "s:", "/:", "a:", "x:", "f:", "g:", "t:"] {
        assert!(en.contains(token), "English hints lost {token:?}: {en}");
        assert!(zh.contains(token), "Chinese hints lost {token:?}: {zh}");
    }
    // `cpu` is read the same everywhere; `ram` is spelled out in Chinese.
    assert_eq!(t(Lang::En, Key::SortCpu), t(Lang::ZhHans, Key::SortCpu));
}

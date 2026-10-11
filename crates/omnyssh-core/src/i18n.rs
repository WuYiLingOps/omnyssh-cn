//! Core-side localization.
//!
//! The engine itself produces a small amount of user-facing text — today only
//! the key-setup step descriptions — that both frontends render.  Those live
//! here so the TUI and the desktop app share one source of truth instead of
//! each shipping its own copy.
//!
//! Strings that belong to a *frontend's* own screens do not belong here: only
//! text the engine authors does, exactly as `KeySetupStep::description` did
//! before this module existed.

use crate::ssh::key_setup::KeySetupStep;

/// A UI language, resolved from a BCP-47 short code.
///
/// `En` is both the default and the fallback: an unrecognized or missing code
/// never leaves the UI without strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Lang {
    /// English (default).
    #[default]
    En,
    /// Simplified Chinese.
    ZhHans,
}

impl Lang {
    /// Resolves a BCP-47 short code to a [`Lang`], falling back to [`Lang::En`]
    /// for anything unrecognized (including the empty string).
    ///
    /// # Examples
    /// ```
    /// # use omnyssh_core::i18n::Lang;
    /// assert_eq!(Lang::from_code("zh-Hans"), Lang::ZhHans);
    /// assert_eq!(Lang::from_code("en"), Lang::En);
    /// assert_eq!(Lang::from_code("fr"), Lang::En);
    /// ```
    pub fn from_code(code: &str) -> Self {
        match code {
            "zh-Hans" => Self::ZhHans,
            _ => Self::En,
        }
    }

    /// The canonical BCP-47 code for this language.
    pub fn code(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::ZhHans => "zh-Hans",
        }
    }

    /// Every language the UI ships, in menu order.
    pub fn all() -> &'static [Self] {
        &[Self::En, Self::ZhHans]
    }
}

/// The key-setup step description in `lang`.
///
/// This is the localized counterpart of [`KeySetupStep::description`], which
/// returns the English text unconditionally.
pub fn key_setup_step(step: KeySetupStep, lang: Lang) -> &'static str {
    match lang {
        Lang::En => match step {
            KeySetupStep::GenerateKey => "Generating Ed25519 key pair",
            KeySetupStep::CopyPublicKey => "Copying public key to server",
            KeySetupStep::VerifyKeyAuth => "Verifying key authentication",
            KeySetupStep::DisablePassword => "Disabling password authentication",
            KeySetupStep::ReloadSshd => "Reloading SSH service",
            KeySetupStep::FinalCheck => "Final verification",
        },
        Lang::ZhHans => match step {
            KeySetupStep::GenerateKey => "正在生成 Ed25519 密钥对",
            KeySetupStep::CopyPublicKey => "正在将公钥复制到服务器",
            KeySetupStep::VerifyKeyAuth => "正在验证密钥认证",
            KeySetupStep::DisablePassword => "正在禁用密码认证",
            KeySetupStep::ReloadSshd => "正在重载 SSH 服务",
            KeySetupStep::FinalCheck => "最终校验",
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_code_maps_known_codes_and_falls_back() {
        assert_eq!(Lang::from_code("en"), Lang::En);
        assert_eq!(Lang::from_code("zh-Hans"), Lang::ZhHans);
        assert_eq!(Lang::from_code("fr"), Lang::En);
        assert_eq!(Lang::from_code(""), Lang::En);
    }

    #[test]
    fn code_round_trips_through_from_code() {
        for lang in Lang::all() {
            assert_eq!(Lang::from_code(lang.code()), *lang);
        }
    }

    #[test]
    fn every_step_has_a_non_empty_description_in_every_language() {
        for lang in Lang::all() {
            for step in KeySetupStep::all_steps() {
                assert!(
                    !key_setup_step(step, *lang).trim().is_empty(),
                    "{:?} missing for {:?}",
                    step,
                    lang
                );
            }
        }
    }

    #[test]
    fn english_matches_the_step_enum_default() {
        for step in KeySetupStep::all_steps() {
            assert_eq!(key_setup_step(step, Lang::En), step.description());
        }
    }
}

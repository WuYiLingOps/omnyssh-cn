//! Strings the *desktop shell* itself draws: the macOS app menu and the system
//! tray. Everything the webview renders is translated in the frontend message
//! packs; these few are the only user-facing strings that never pass through it.
//!
//! The language comes from the shared config (`ui.language`), the same source the
//! frontend mirrors into the plugin store — one setting, three frontends.

use omnyssh_core::config::app_config::{load_app_config, save_language_to_config};
use omnyssh_core::i18n::Lang;
use tauri::{AppHandle, Manager, Wry};

use crate::state::GuiState;

/// Reads the shared config's `ui.language`, falling back to English for a missing
/// file or an unrecognized code.
pub fn current_lang() -> Lang {
    match load_app_config(None) {
        Ok(config) => Lang::from_code(&config.ui.language),
        Err(_) => Lang::En,
    }
}

/// The menu item and tray labels the shell owns, resolved for one [`Lang`].
///
/// A `struct` rather than a `match` per call site: both builders want the whole
/// set at once, and every field is a visible string with no logic behind it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeStrings {
    /// The File menu's title.
    pub file_menu: &'static str,
    /// The item that asks the page to close the active window.
    pub close_window: &'static str,
    /// The main menu's Quit item.
    pub quit: &'static str,
    /// The tray menu's item that brings the window forward.
    pub show_window: &'static str,
    /// The tray icon's hover tooltip.
    pub tray_tooltip: &'static str,
}

/// The shell strings in `lang`.
pub fn strings(lang: Lang) -> NativeStrings {
    match lang {
        Lang::En => NativeStrings {
            file_menu: "File",
            close_window: "Close Window",
            quit: "Quit OmnySSH",
            show_window: "Show OmnySSH",
            tray_tooltip: "OmnySSH",
        },
        Lang::ZhHans => NativeStrings {
            file_menu: "文件",
            close_window: "关闭窗口",
            quit: "退出 OmnySSH",
            show_window: "显示 OmnySSH",
            // The product name is a brand token: untranslated in every language
            // (tech-gui.md §2.4 white list).
            tray_tooltip: "OmnySSH",
        },
    }
}

/// Points the shell's native decorations at `code`: persists it to the shared
/// config so the TUI renders the same language, rebuilds the macOS menu and the
/// tray text, and returns the resolved code so the frontend can reconcile.
///
/// Best-effort on the rebuilds: a failed menu or tray swap leaves the old labels
/// in place and must never fail the switch the user actually asked for.
#[tauri::command]
#[specta::specta]
pub fn set_language(app: AppHandle<Wry>, code: String) -> Result<String, String> {
    save_language_to_config(&code).map_err(|e| e.to_string())?;
    let lang = Lang::from_code(&code);

    if let Some(state) = app.try_state::<GuiState>() {
        state.set_language(lang);
    }

    #[cfg(target_os = "macos")]
    {
        // Best effort: a failed rebuild keeps the current menu. `build`
        // resolves the language itself (state, then config).
        if let Ok(menu) = crate::menu::build(&app) {
            let _ = app.set_menu(menu);
        }
    }
    crate::tray::retranslate(&app, lang);

    Ok(lang.code().to_string())
}

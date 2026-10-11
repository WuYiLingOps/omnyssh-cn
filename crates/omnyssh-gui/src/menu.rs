//! The macOS app menu. Tauri's default binds ⌘W and ⌘Q to `performClose:` and
//! `terminate:`, which close the window and end the app without asking; this one keeps
//! everything else and routes both through the page instead.

use tauri::menu::{AboutMetadata, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Manager, Wry};

use crate::i18n;

pub const QUIT_ID: &str = "app-quit";
pub const CLOSE_WINDOW_ID: &str = "app-close-window";

/// No item claims ⌘W, so the page gets it and closes the active tab.
pub fn build(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    // The shell keeps its language in managed state; fall back to the config for a
    // call that arrives before `manage` (unlikely but harmless).
    let lang = app
        .try_state::<crate::state::GuiState>()
        .map_or_else(i18n::current_lang, |state| state.language());
    let strings = i18n::strings(lang);
    let info = app.package_info();
    let about = AboutMetadata {
        name: Some(info.name.clone()),
        version: Some(info.version.to_string()),
        copyright: app.config().bundle.copyright.clone(),
        authors: app.config().bundle.publisher.clone().map(|p| vec![p]),
        ..Default::default()
    };
    let separator = || PredefinedMenuItem::separator(app);

    let app_menu = Submenu::with_items(
        app,
        info.name.clone(),
        true,
        &[
            &PredefinedMenuItem::about(app, None, Some(about))?,
            &separator()?,
            &PredefinedMenuItem::services(app, None)?,
            &separator()?,
            &PredefinedMenuItem::hide(app, None)?,
            &PredefinedMenuItem::hide_others(app, None)?,
            &PredefinedMenuItem::show_all(app, None)?,
            &separator()?,
            &MenuItem::with_id(app, QUIT_ID, strings.quit, true, Some("CmdOrCtrl+Q"))?,
        ],
    )?;
    // Closing goes through `CloseRequested`, so the tray or the page's question still apply.
    let file = Submenu::with_items(
        app,
        strings.file_menu,
        true,
        &[&MenuItem::with_id(
            app,
            CLOSE_WINDOW_ID,
            strings.close_window,
            true,
            Some("CmdOrCtrl+Shift+W"),
        )?],
    )?;
    // WKWebView's clipboard and undo shortcuts work only through these items.
    let edit = Submenu::with_items(
        app,
        "Edit",
        true,
        &[
            &PredefinedMenuItem::undo(app, None)?,
            &PredefinedMenuItem::redo(app, None)?,
            &separator()?,
            &PredefinedMenuItem::cut(app, None)?,
            &PredefinedMenuItem::copy(app, None)?,
            &PredefinedMenuItem::paste(app, None)?,
            &PredefinedMenuItem::select_all(app, None)?,
        ],
    )?;
    let view = Submenu::with_items(
        app,
        "View",
        true,
        &[&PredefinedMenuItem::fullscreen(app, None)?],
    )?;
    let window = Submenu::with_items(
        app,
        "Window",
        true,
        &[
            &PredefinedMenuItem::minimize(app, None)?,
            // "Zoom" on macOS.
            &PredefinedMenuItem::maximize(app, None)?,
        ],
    )?;
    Menu::with_items(app, &[&app_menu, &file, &edit, &view, &window])
}

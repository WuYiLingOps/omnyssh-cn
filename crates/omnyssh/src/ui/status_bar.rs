use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::{AppState, FileManagerPopup, Screen, SnippetPopup, ViewState};
use crate::i18n::{t, Key};

/// Renders the bottom status bar with context-sensitive key hints.
pub fn render(frame: &mut Frame, area: Rect, state: &AppState, view: &ViewState) {
    // Apply active theme colours.
    let key_style = Style::default()
        .fg(view.theme.key_badge_fg)
        .bg(view.theme.key_badge_bg)
        .add_modifier(Modifier::BOLD);
    let sep_style = Style::default().fg(view.theme.separator_fg);
    let hint_style = Style::default().fg(view.theme.hint_fg);

    // Translation of a key in the active language; the short forms keep the
    // hint tables below readable.
    let tr = |key: Key| t(view.lang, key);

    macro_rules! key {
        ($k:expr) => {
            Span::styled(format!(" {} ", $k), key_style)
        };
    }
    macro_rules! hint {
        ($h:expr) => {
            Span::styled(format!(" {} ", tr($h)), hint_style)
        };
    }
    macro_rules! sep {
        () => {
            Span::styled(" │ ", sep_style)
        };
    }

    // If a status message is set, show it instead of key hints.
    if let Some(msg) = &view.status_message {
        let line = Line::from(Span::styled(
            format!(" {}", msg),
            Style::default().fg(Color::Yellow),
        ));
        frame.render_widget(
            Paragraph::new(line).style(Style::default().bg(Color::Reset)),
            area,
        );
        return;
    }

    let hlv = &view.host_list;

    // Search mode: show search-specific hints.
    if hlv.search_mode {
        let line = Line::from(vec![
            Span::styled(
                format!(" {} ", tr(Key::SearchBadge)),
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" {}  ", tr(Key::SearchTypeToFilter)),
                Style::default().fg(Color::Gray),
            ),
            key!("Enter"),
            hint!(Key::HintConfirm),
            sep!(),
            key!("Esc"),
            hint!(Key::HintClear),
        ]);
        frame.render_widget(
            Paragraph::new(line).style(Style::default().bg(Color::Reset)),
            area,
        );
        return;
    }

    // Popup mode: show popup-specific hints.
    if let Some(popup) = &hlv.popup {
        use crate::app::HostPopup;
        let line = match popup {
            HostPopup::Add(_) | HostPopup::Edit { .. } => Line::from(vec![
                key!("Tab"),
                hint!(Key::HintNextField),
                sep!(),
                key!("Shift+Tab"),
                hint!(Key::HintPrevField),
                sep!(),
                key!("Enter"),
                hint!(Key::HintSave),
                sep!(),
                key!("Esc"),
                hint!(Key::HintCancel),
            ]),
            HostPopup::DeleteConfirm(_) => Line::from(vec![
                key!("y"),
                hint!(Key::HintConfirmDelete),
                sep!(),
                key!("n / Esc"),
                hint!(Key::HintCancel),
            ]),
            HostPopup::KeySetupConfirm(_) => Line::from(vec![
                key!("y / Enter"),
                hint!(Key::HintConfirmSetup),
                sep!(),
                key!("n / Esc"),
                hint!(Key::HintCancel),
            ]),
            HostPopup::KeySetupProgress { .. } => Line::from(vec![
                Span::styled(format!(" {} ", tr(Key::SettingUpKeys)), hint_style),
                sep!(),
                key!("Esc"),
                hint!(Key::HintClose),
            ]),
        };
        frame.render_widget(
            Paragraph::new(line).style(Style::default().bg(Color::Reset)),
            area,
        );
        return;
    }

    // Snippets search mode.
    if view.snippets_view.search_mode {
        let line = Line::from(vec![
            Span::styled(
                format!(" {} ", tr(Key::SearchBadge)),
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" {}  ", tr(Key::SearchTypeToFilter)),
                Style::default().fg(Color::Gray),
            ),
            key!("Enter"),
            hint!(Key::HintConfirm),
            sep!(),
            key!("Esc"),
            hint!(Key::HintClear),
        ]);
        frame.render_widget(
            Paragraph::new(line).style(Style::default().bg(Color::Reset)),
            area,
        );
        return;
    }

    // Snippets popup hints.
    if let Some(popup) = &view.snippets_view.popup {
        let line = match popup {
            SnippetPopup::Add(_) | SnippetPopup::Edit { .. } => Line::from(vec![
                key!("Tab"),
                hint!(Key::HintNextField),
                sep!(),
                key!("Shift+Tab"),
                hint!(Key::HintPrevField),
                sep!(),
                key!("Enter"),
                hint!(Key::HintSave),
                sep!(),
                key!("Esc"),
                hint!(Key::HintCancel),
            ]),
            SnippetPopup::DeleteConfirm(_) => Line::from(vec![
                key!("y"),
                hint!(Key::HintConfirmDelete),
                sep!(),
                key!("n / Esc"),
                hint!(Key::HintCancel),
            ]),
            SnippetPopup::ParamInput { .. } => Line::from(vec![
                key!("Tab"),
                hint!(Key::HintNextParam),
                sep!(),
                key!("Enter"),
                hint!(Key::HintRun),
                sep!(),
                key!("Esc"),
                hint!(Key::HintCancel),
            ]),
            SnippetPopup::BroadcastPicker { .. } => Line::from(vec![
                key!("j/k"),
                hint!(Key::HintNavigate),
                sep!(),
                key!("Space"),
                hint!(Key::HintToggle),
                sep!(),
                key!("Enter"),
                hint!(Key::HintRun),
                sep!(),
                key!("Esc"),
                hint!(Key::HintCancel),
            ]),
            SnippetPopup::QuickExecuteInput { .. } => Line::from(vec![
                key!("Enter"),
                hint!(Key::HintRun),
                sep!(),
                key!("Esc"),
                hint!(Key::HintCancel),
            ]),
            SnippetPopup::Results { .. } => Line::from(vec![
                key!("j/k"),
                hint!(Key::HintScroll),
                sep!(),
                key!("Esc"),
                hint!(Key::HintClose),
            ]),
        };
        frame.render_widget(
            Paragraph::new(line).style(Style::default().bg(Color::Reset)),
            area,
        );
        return;
    }

    // File manager popup hints.
    if matches!(state.screen, Screen::FileManager) {
        if let Some(fm_popup) = &view.file_manager.popup {
            let line = match fm_popup {
                FileManagerPopup::HostPicker { .. } => Line::from(vec![
                    key!("j/k"),
                    hint!(Key::HintNavigate),
                    sep!(),
                    key!("Enter"),
                    hint!(Key::HintConnect),
                    sep!(),
                    key!("Esc"),
                    hint!(Key::HintCancel),
                ]),
                FileManagerPopup::DeleteConfirm { .. } => Line::from(vec![
                    key!("y"),
                    hint!(Key::HintConfirmDelete),
                    sep!(),
                    key!("n / Esc"),
                    hint!(Key::HintCancel),
                ]),
                FileManagerPopup::MkDir(_) | FileManagerPopup::Rename { .. } => Line::from(vec![
                    key!("Enter"),
                    hint!(Key::HintConfirm),
                    sep!(),
                    key!("Esc"),
                    hint!(Key::HintCancel),
                ]),
                FileManagerPopup::TransferProgress {
                    filename,
                    done,
                    total,
                    ..
                } => {
                    let pct = if *total > 0 {
                        (((*done as f64 / *total as f64) * 100.0) as u64).min(100)
                    } else {
                        0
                    };
                    Line::from(vec![
                        Span::styled(
                            format!(
                                " {}: {}  {}% ",
                                crate::i18n::t(view.lang, Key::HintTransferring),
                                filename,
                                pct
                            ),
                            hint_style,
                        ),
                        sep!(),
                        key!("Esc"),
                        hint!(Key::HintCancel),
                    ])
                }
            };
            frame.render_widget(
                Paragraph::new(line).style(Style::default().bg(Color::Reset)),
                area,
            );
            return;
        }
    }

    // Tag popup: show its own hints.
    if hlv.tag_popup_open {
        let line = Line::from(vec![
            key!("j/k"),
            hint!(Key::HintNavigate),
            sep!(),
            key!("Enter"),
            hint!(Key::HintSelect),
            sep!(),
            key!("Esc"),
            hint!(Key::HintClose),
        ]);
        frame.render_widget(
            Paragraph::new(line).style(Style::default().bg(Color::Reset)),
            area,
        );
        return;
    }

    // Global hints only - screen-specific hints are now in page headers.
    let spans = vec![
        key!("1"),
        hint!(Key::NavDashboard),
        sep!(),
        key!("2"),
        hint!(Key::NavFiles),
        sep!(),
        key!("3"),
        hint!(Key::NavSnippets),
        sep!(),
        key!("4"),
        hint!(Key::NavTerminal),
        sep!(),
        key!("?"),
        hint!(Key::NavHelp),
        sep!(),
        key!("q"),
        hint!(Key::NavQuit),
    ];

    frame.render_widget(
        Paragraph::new(Line::from(spans)).style(Style::default().bg(Color::Reset)),
        area,
    );
}

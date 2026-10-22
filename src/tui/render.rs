//! Drawing of the screens.
//!
//! Every screen has the same minimalist layout: a title line, the content, and
//! a key hint at the bottom.

use i18n_embed::fluent::FluentLanguageLoader;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Margin};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Paragraph, Wrap};

use super::state::{Action, MainEntry, Scope, Screen, State};
use crate::i18n::fl;
use crate::language::Language;

/// Marker of the selected menu entry.
const CURSOR: &str = "› ";
/// Indentation of the other menu entries, as wide as the cursor.
const NO_CURSOR: &str = "  ";

/// Draws the current screen of `state`.
pub fn draw(frame: &mut Frame, state: &State, loader: &FluentLanguageLoader) {
    let [title_area, content_area, hint_area] = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(frame.area().inner(Margin::new(1, 1)));

    let app_title = fl!(loader, "app-title");
    let (title, content, hint) = match state.screen() {
        Screen::MainMenu => (
            app_title,
            main_menu(state, loader),
            fl!(loader, "hint-main-menu"),
        ),
        Screen::ScopeMenu(action) => (
            breadcrumb(&app_title, &action_label(loader, *action)),
            scope_menu(state, loader),
            fl!(loader, "hint-submenu"),
        ),
        Screen::LanguageMenu => (
            breadcrumb(&app_title, &fl!(loader, "menu-language")),
            language_menu(state),
            fl!(loader, "hint-submenu"),
        ),
        Screen::Loading(message) => (app_title, Text::from(message.clone()), String::new()),
        Screen::Message(message) => (
            app_title,
            Text::from(message.clone()),
            fl!(loader, "press-any-key"),
        ),
    };

    frame.render_widget(Line::from(title).bold(), title_area);
    frame.render_widget(
        Paragraph::new(content).wrap(Wrap { trim: false }),
        content_area,
    );
    frame.render_widget(Line::from(hint).fg(Color::DarkGray), hint_area);
}

fn breadcrumb(title: &str, page: &str) -> String {
    format!("{title} › {page}")
}

fn action_label(loader: &FluentLanguageLoader, action: Action) -> String {
    match action {
        Action::Protect => fl!(loader, "menu-protect"),
        Action::Delete => fl!(loader, "menu-delete"),
    }
}

fn main_menu(state: &State, loader: &FluentLanguageLoader) -> Text<'static> {
    MainEntry::ALL
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let label = match entry {
                MainEntry::Protect => fl!(loader, "menu-protect"),
                MainEntry::Delete => fl!(loader, "menu-delete"),
                MainEntry::Language => fl!(loader, "menu-language"),
                MainEntry::Quit => fl!(loader, "menu-quit"),
            };
            menu_line(label, index == state.main_cursor(), None)
        })
        .collect()
}

fn scope_menu(state: &State, loader: &FluentLanguageLoader) -> Text<'static> {
    Scope::ALL
        .iter()
        .enumerate()
        .map(|(index, scope)| {
            let label = match scope {
                Scope::All => fl!(loader, "scope-all"),
                Scope::SinceLastDeletion => fl!(loader, "scope-since-last-deletion"),
            };
            let disabled_label =
                (!state.is_scope_enabled(*scope)).then(|| fl!(loader, "label-no-deletion-yet"));
            menu_line(label, index == state.submenu_cursor(), disabled_label)
        })
        .collect()
}

fn language_menu(state: &State) -> Text<'static> {
    Language::ALL
        .iter()
        .enumerate()
        .map(|(index, language)| {
            menu_line(
                language.native_name().to_owned(),
                index == state.submenu_cursor(),
                None,
            )
        })
        .collect()
}

/// Line of a menu entry. A `disabled_label` grays the entry out and explains why.
fn menu_line(label: String, selected: bool, disabled_label: Option<String>) -> Line<'static> {
    let cursor = if selected { CURSOR } else { NO_CURSOR };
    match disabled_label {
        Some(reason) => Line::from(vec![
            Span::raw(cursor),
            Span::raw(label),
            Span::raw("  "),
            Span::raw(reason),
        ])
        .fg(Color::DarkGray),
        None if selected => Line::from(vec![
            Span::raw(cursor),
            Span::styled(label, Style::new().add_modifier(Modifier::BOLD)),
        ]),
        None => Line::from(vec![Span::raw(cursor), Span::raw(label)]),
    }
}

#[cfg(test)]
mod tests {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use super::*;
    use crate::i18n;

    /// Renders `state` in a 60×12 terminal and returns its text, line by line.
    fn render(state: &State, language: Language) -> String {
        let loader = i18n::load(language).unwrap();
        let mut terminal = Terminal::new(TestBackend::new(60, 12)).unwrap();
        terminal.draw(|frame| draw(frame, state, &loader)).unwrap();

        let buffer = terminal.backend().buffer();
        (0..buffer.area.height)
            .map(|y| {
                (0..buffer.area.width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
                    .trim_end()
                    .to_owned()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn main_menu_shows_the_entries_and_the_cursor() {
        let screen = render(&State::new(Language::English, None), Language::English);

        assert!(screen.contains("Repository Cleaner"));
        assert!(screen.contains("› Protect repositories"));
        assert!(screen.contains("  Quit"));
        assert!(screen.contains("↑/↓ move · Enter select"));
        assert!(!screen.contains("Esc"));
    }

    #[test]
    fn main_menu_is_translated() {
        let screen = render(&State::new(Language::French, None), Language::French);

        assert!(screen.contains("Nettoyeur de dépôts"));
        assert!(screen.contains("› Protéger des dépôts"));
    }

    #[test]
    fn scope_menu_shows_a_breadcrumb_and_the_disabled_scope() {
        let mut state = State::new(Language::English, None);
        state.open_scope_menu(Action::Delete);

        let screen = render(&state, Language::English);

        assert!(screen.contains("Repository Cleaner › Delete repositories"));
        assert!(screen.contains("Only repositories since last deletion  no deletion yet"));
        assert!(screen.contains("Esc back"));
    }

    #[test]
    fn loading_screen_has_no_hint() {
        let mut state = State::new(Language::English, None);
        state.show_loading("Checking permissions…".to_owned());

        let screen = render(&state, Language::English);

        assert!(screen.contains("Checking permissions…"));
        assert!(!screen.contains("Press any key"));
    }

    #[test]
    fn message_shows_how_to_continue() {
        let mut state = State::new(Language::English, None);
        state.show_message("Something went wrong".to_owned());

        let screen = render(&state, Language::English);

        assert!(screen.contains("Something went wrong"));
        assert!(screen.contains("Press any key to return to the menu"));
    }
}

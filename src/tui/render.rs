//! Drawing of the screens.
//!
//! Every screen has the same minimalist layout: a title line, the content, and
//! a key hint at the bottom.

use i18n_embed::fluent::FluentLanguageLoader;
use jiff::tz::TimeZone;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Margin, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Cell, HighlightSpacing, Paragraph, Row, Table, TableState, Wrap};

use super::format;
use super::list::centered_offset;
use super::state::{Action, MainEntry, ProtectionList, Scope, Screen, State};
use crate::github::Repository;
use crate::i18n::fl;
use crate::language::Language;

/// Marker of the selected menu entry.
const CURSOR: &str = "› ";
/// Indentation of the other menu entries, as wide as the cursor.
const NO_CURSOR: &str = "  ";

/// Body of a screen, below its title.
enum Body<'a> {
    Text(Text<'static>),
    ProtectionList(&'a ProtectionList),
}

/// Draws the current screen of `state`.
pub fn draw(frame: &mut Frame, state: &State, loader: &FluentLanguageLoader) {
    let app_title = fl!(loader, "app-title");
    let (title, body, hint) = match state.screen() {
        Screen::MainMenu => (
            app_title,
            Body::Text(main_menu(state, loader)),
            fl!(loader, "hint-main-menu"),
        ),
        Screen::ScopeMenu(action) => (
            breadcrumb(&app_title, &action_label(loader, *action)),
            Body::Text(scope_menu(state, loader)),
            fl!(loader, "hint-submenu"),
        ),
        Screen::LanguageMenu => (
            breadcrumb(&app_title, &fl!(loader, "menu-language")),
            Body::Text(language_menu(state)),
            fl!(loader, "hint-submenu"),
        ),
        Screen::ProtectionList(protection) => (
            breadcrumb(&app_title, &fl!(loader, "menu-protect")),
            Body::ProtectionList(protection),
            fl!(loader, "hint-protection"),
        ),
        Screen::Loading(message) => (
            app_title,
            Body::Text(Text::from(message.clone())),
            String::new(),
        ),
        Screen::Message(message) => (
            app_title,
            Body::Text(Text::from(message.clone())),
            fl!(loader, "press-any-key"),
        ),
    };

    let area = frame.area().inner(Margin::new(1, 1));
    // Long key hints wrap on a second line in narrow terminals.
    let hint_height = if hint.chars().count() > usize::from(area.width) {
        2
    } else {
        1
    };
    let [title_area, content_area, hint_area] = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(0),
        Constraint::Length(hint_height),
    ])
    .areas(area);

    frame.render_widget(Line::from(title).bold(), title_area);
    frame.render_widget(
        Paragraph::new(hint)
            .fg(Color::DarkGray)
            .wrap(Wrap { trim: true }),
        hint_area,
    );
    match body {
        Body::Text(text) => frame.render_widget(
            Paragraph::new(text).wrap(Wrap { trim: false }),
            content_area,
        ),
        Body::ProtectionList(protection) => {
            draw_protection_list(frame, content_area, protection, loader);
        }
    }
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

fn draw_protection_list(
    frame: &mut Frame,
    area: Rect,
    protection: &ProtectionList,
    loader: &FluentLanguageLoader,
) {
    let [header_area, table_area] =
        Layout::vertical([Constraint::Length(2), Constraint::Min(0)]).areas(area);
    let time_zone = TimeZone::system();
    let repositories = protection.list().repositories();

    let total = match protection.since() {
        Some(since) => fl!(
            loader,
            "new-repository-count",
            count = repositories.len(),
            since = format::date(since, &time_zone)
        ),
        None => fl!(loader, "repository-count", count = repositories.len()),
    };
    let protected = fl!(
        loader,
        "protected-count",
        count = protection.protected_count()
    );
    frame.render_widget(Line::from(format!("{total} · {protected}")), header_area);

    let table = repository_table(repositories, loader, &time_zone, |repository| {
        if protection.is_protected(repository) {
            (
                Span::styled("[P]", Style::new().fg(Color::Green)),
                Style::new(),
            )
        } else {
            (Span::raw("[ ]"), Style::new())
        }
    });
    let mut table_state = table_state(protection.list().cursor(), repositories.len(), table_area);
    frame.render_stateful_widget(table, table_area, &mut table_state);
}

/// Table state with the cursor kept in the middle of the visible rows.
fn table_state(cursor: Option<usize>, len: usize, area: Rect) -> TableState {
    let offset = cursor.map_or(0, |cursor| {
        centered_offset(cursor, len, usize::from(area.height))
    });
    TableState::new().with_selected(cursor).with_offset(offset)
}

/// Translated labels shown in the table, computed once per drawing.
struct Labels {
    public: String,
    private: String,
    fork: String,
    archived: String,
}

impl Labels {
    fn new(loader: &FluentLanguageLoader) -> Self {
        Self {
            public: fl!(loader, "label-public"),
            private: fl!(loader, "label-private"),
            fork: fl!(loader, "label-fork"),
            archived: fl!(loader, "label-archived"),
        }
    }
}

/// Table of `repositories`. `row_marker` gives the marker that starts each
/// row and the style of the whole row.
fn repository_table(
    repositories: &[Repository],
    loader: &FluentLanguageLoader,
    time_zone: &TimeZone,
    row_marker: impl Fn(&Repository) -> (Span<'static>, Style),
) -> Table<'static> {
    let labels = Labels::new(loader);
    let columns: Vec<[String; 4]> = repositories
        .iter()
        .map(|repository| {
            let visibility = if repository.private {
                &labels.private
            } else {
                &labels.public
            };
            let status = [
                repository.fork.then_some(labels.fork.as_str()),
                repository.archived.then_some(labels.archived.as_str()),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(" ");
            [
                visibility.clone(),
                status,
                format::size(loader, repository.size_kb),
                format::date(repository.last_update(), time_zone),
            ]
        })
        .collect();

    let width = |column: usize| {
        let width = columns
            .iter()
            .map(|row| row[column].chars().count())
            .max()
            .unwrap_or(0);
        Constraint::Length(u16::try_from(width).unwrap_or(u16::MAX))
    };
    let widths = [
        Constraint::Length(3),
        Constraint::Fill(1),
        width(0),
        width(1),
        width(2),
        width(3),
    ];

    let rows =
        repositories
            .iter()
            .zip(columns)
            .map(|(repository, [visibility, status, size, date])| {
                let (marker, style) = row_marker(repository);
                Row::new([
                    Cell::from(marker),
                    Cell::from(repository.name.clone()),
                    Cell::from(visibility).fg(Color::DarkGray),
                    Cell::from(status).fg(Color::DarkGray),
                    Cell::from(Line::from(size).right_aligned()),
                    Cell::from(date),
                ])
                .style(style)
            });

    Table::new(rows, widths)
        .column_spacing(2)
        .highlight_symbol(CURSOR)
        .highlight_spacing(HighlightSpacing::Always)
        .row_highlight_style(Style::new().add_modifier(Modifier::BOLD))
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

    use std::collections::BTreeSet;

    use super::super::keymap::Command;
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
    fn protection_list_shows_the_header_markers_and_labels() {
        let mut state = State::new(Language::English, None);
        let repository = |name: &str, private, fork, archived, size_kb| Repository {
            name: name.to_owned(),
            private,
            fork,
            archived,
            size_kb,
            created_at: "2021-01-01T12:00:00Z".parse().unwrap(),
            pushed_at: Some("2023-05-01T12:00:00Z".parse().unwrap()),
        };
        state.open_protection_list(
            vec![
                repository("alpha", false, false, false, 764),
                repository("beta", true, true, true, 2048),
            ],
            ["beta".to_owned()].into(),
            Scope::All,
        );

        let screen = render(&state, Language::English);

        assert!(screen.contains("Repository Cleaner › Protect repositories"));
        assert!(screen.contains("2 repositories · 1 protected"));
        assert!(screen.contains("› [ ]  alpha"));
        assert!(screen.contains("[P]  beta"));
        assert!(screen.contains("private  fork archived"));
        assert!(screen.contains("764 KB  2023-05-01"));
        assert!(screen.contains("2 MB  2023-05-01"));
    }

    /// Opens the protection list with `count` repositories named `repo-00`, `repo-01`…
    fn protection_with(count: usize) -> State {
        let mut state = State::new(Language::English, None);
        let repositories = (0..count)
            .map(|index| Repository {
                name: format!("repo-{index:02}"),
                private: false,
                fork: false,
                archived: false,
                size_kb: 0,
                created_at: "2021-01-01T12:00:00Z".parse().unwrap(),
                pushed_at: None,
            })
            .collect();
        state.open_protection_list(repositories, BTreeSet::new(), Scope::All);
        state
    }

    #[test]
    fn last_repository_is_visible_after_end() {
        let mut state = protection_with(20);

        state.handle(Command::Last);
        let screen = render(&state, Language::English);

        assert!(screen.contains("› [ ]  repo-19"));
        assert!(!screen.contains("repo-00"));
    }

    #[test]
    fn cursor_stays_in_the_middle_of_the_visible_rows() {
        let mut state = protection_with(20);
        for _ in 0..10 {
            state.handle(Command::Down);
        }

        let screen = render(&state, Language::English);
        let rows: Vec<_> = screen
            .lines()
            .filter(|line| line.contains("repo-"))
            .collect();

        // The 60×12 test terminal shows 4 rows (the hint takes 2 lines):
        // the cursor is on the third one, after half of the visible rows.
        assert_eq!(rows.len(), 4);
        assert!(rows[2].contains("› [ ]  repo-10"));
        assert!(rows[0].contains("repo-08") && rows[3].contains("repo-11"));
    }

    #[test]
    fn long_hints_wrap_instead_of_being_cut() {
        let state = protection_with(3);

        let screen = render(&state, Language::English);

        assert!(screen.contains("Esc cancel"));
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

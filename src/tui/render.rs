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
use ratatui::widgets::{
    Cell, HighlightSpacing, LineGauge, Paragraph, Row, Table, TableState, Wrap,
};

use super::format;
use super::list::centered_offset;
use super::state::{Action, DeletionList, MainEntry, ProtectionList, Scope, Screen, State};
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
    DeletionList(&'a DeletionList),
    ConfirmDeletion(&'a DeletionList),
    DeletionProgress { done: usize, total: usize },
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
        Screen::DeletionList(deletion) => (
            breadcrumb(&app_title, &fl!(loader, "menu-delete")),
            Body::DeletionList(deletion),
            fl!(loader, "hint-deletion"),
        ),
        Screen::ConfirmDeletion(deletion) => (
            breadcrumb(&app_title, &fl!(loader, "menu-delete")),
            Body::ConfirmDeletion(deletion),
            String::new(),
        ),
        &Screen::DeletionProgress { done, total } => (
            breadcrumb(&app_title, &fl!(loader, "menu-delete")),
            Body::DeletionProgress { done, total },
            fl!(loader, "hint-deletion-progress"),
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
        Body::DeletionList(deletion) => draw_deletion_list(frame, content_area, deletion, loader),
        Body::ConfirmDeletion(deletion) => {
            draw_confirm_deletion(frame, content_area, deletion, loader);
        }
        Body::DeletionProgress { done, total } => {
            draw_deletion_progress(frame, content_area, done, total, loader);
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
            RowDecoration::marker(Span::styled("[P]", Style::new().fg(Color::Green)))
        } else {
            RowDecoration::marker(Span::raw("[ ]"))
        }
    });
    let mut table_state = table_state(protection.list().cursor(), repositories.len(), table_area);
    frame.render_stateful_widget(table, table_area, &mut table_state);
}

fn draw_deletion_list(
    frame: &mut Frame,
    area: Rect,
    deletion: &DeletionList,
    loader: &FluentLanguageLoader,
) {
    let [header_area, notice_area, table_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(0),
    ])
    .areas(area);
    let time_zone = TimeZone::system();
    let repositories = deletion.list().repositories();

    let total = match deletion.since() {
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
        count = deletion.protected_count()
    );
    let marked = fl!(loader, "marked-count", count = deletion.marked_count());
    frame.render_widget(
        Line::from(format!("{total} · {protected} · {marked}")),
        header_area,
    );
    if deletion.nothing_selected() {
        frame.render_widget(
            Line::from(fl!(loader, "no-repository-selected")).fg(Color::Yellow),
            notice_area,
        );
    }

    let protected_label = fl!(loader, "label-protected");
    let table = repository_table(repositories, loader, &time_zone, |repository| {
        if deletion.is_protected(repository) {
            RowDecoration {
                marker: Span::raw("[ ]"),
                style: Style::new().fg(Color::DarkGray),
                label: Some(protected_label.clone()),
            }
        } else if deletion.is_marked(repository) {
            RowDecoration::marker(Span::styled("[✗]", Style::new().fg(Color::Red)))
        } else {
            RowDecoration::marker(Span::raw("[ ]"))
        }
    });
    let mut table_state = table_state(deletion.list().cursor(), repositories.len(), table_area);
    frame.render_stateful_widget(table, table_area, &mut table_state);
}

fn draw_confirm_deletion(
    frame: &mut Frame,
    area: Rect,
    deletion: &DeletionList,
    loader: &FluentLanguageLoader,
) {
    let [question_area, names_area] =
        Layout::vertical([Constraint::Length(2), Constraint::Min(0)]).areas(area);
    let names = deletion.marked_names();

    frame.render_widget(
        Line::from(fl!(loader, "confirm-deletion", count = names.len())).bold(),
        question_area,
    );
    let lines = pack_names(
        &names,
        usize::from(names_area.width),
        usize::from(names_area.height),
        |count| fl!(loader, "more-repositories", count = count),
    );
    frame.render_widget(
        lines.into_iter().map(Line::from).collect::<Text>(),
        names_area,
    );
}

fn draw_deletion_progress(
    frame: &mut Frame,
    area: Rect,
    done: usize,
    total: usize,
    loader: &FluentLanguageLoader,
) {
    let [message_area, gauge_area] =
        Layout::vertical([Constraint::Length(2), Constraint::Length(1)]).areas(area);
    let percent = percent(done, total);

    frame.render_widget(Line::from(fl!(loader, "deletion-progress")), message_area);
    frame.render_widget(
        LineGauge::default()
            .ratio(f64::from(percent) / 100.0)
            .label(fl!(
                loader,
                "deletion-progress-count",
                done = done,
                total = total,
                percent = percent
            ))
            .filled_symbol("━")
            .unfilled_symbol("─")
            .unfilled_style(Style::new().fg(Color::DarkGray)),
        gauge_area,
    );
}

/// Share of `done` in `total`, as a whole percentage rounded down, so that 100%
/// is only shown when everything is done. An empty `total` counts as done.
fn percent(done: usize, total: usize) -> u16 {
    if total == 0 {
        return 100;
    }
    let percent = done.min(total) * 100 / total;
    u16::try_from(percent).unwrap_or(100)
}

/// Packs `names` into lines of at most `width` characters, separated by commas.
/// When they do not fit in `max_lines`, the last line ends with `more(count)`
/// for the names left out.
fn pack_names(
    names: &[String],
    width: usize,
    max_lines: usize,
    more: impl Fn(usize) -> String,
) -> Vec<String> {
    let text_width = |line: &[&str]| {
        line.iter()
            .map(|name| name.chars().count() + 2)
            .sum::<usize>()
    };

    let mut lines: Vec<Vec<&str>> = Vec::new();
    for name in names {
        match lines.last_mut() {
            Some(line) if text_width(line) + name.chars().count() < width => line.push(name),
            _ => lines.push(vec![name]),
        }
    }

    let mut left_out = 0;
    if lines.len() > max_lines {
        lines.truncate(max_lines.max(1));
        let shown = |lines: &[Vec<&str>]| lines.iter().map(Vec::len).sum::<usize>();
        // Remove names from the last line until the "more" text fits.
        loop {
            left_out = names.len() - shown(&lines);
            let Some(last) = lines.last_mut() else {
                break;
            };
            if last.is_empty() || text_width(last) + more(left_out).chars().count() <= width {
                break;
            }
            last.pop();
        }
    }

    let line_count = lines.len();
    lines
        .into_iter()
        .enumerate()
        .map(|(index, line)| {
            let mut text = line.join(", ");
            let is_last = index + 1 == line_count;
            if is_last && left_out > 0 {
                if !text.is_empty() {
                    text.push_str(", ");
                }
                text.push_str(&more(left_out));
            } else if !is_last {
                text.push(',');
            }
            text
        })
        .collect()
}

/// Table state with the cursor kept in the middle of the visible rows.
fn table_state(cursor: Option<usize>, len: usize, area: Rect) -> TableState {
    let offset = cursor.map_or(0, |cursor| {
        centered_offset(cursor, len, usize::from(area.height))
    });
    TableState::new().with_selected(cursor).with_offset(offset)
}

/// How a row starts and looks, decided by each list.
struct RowDecoration {
    marker: Span<'static>,
    /// Style of the whole row, for example gray for a row that cannot be marked.
    style: Style,
    /// Extra label shown before the status labels, such as `protected`.
    label: Option<String>,
}

impl RowDecoration {
    fn marker(marker: Span<'static>) -> Self {
        Self {
            marker,
            style: Style::new(),
            label: None,
        }
    }
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

/// Table of `repositories`. `decorate` gives the marker, style and extra label
/// of each row.
fn repository_table(
    repositories: &[Repository],
    loader: &FluentLanguageLoader,
    time_zone: &TimeZone,
    decorate: impl Fn(&Repository) -> RowDecoration,
) -> Table<'static> {
    let labels = Labels::new(loader);
    let decorations: Vec<_> = repositories.iter().map(decorate).collect();
    let columns: Vec<[String; 4]> = repositories
        .iter()
        .zip(&decorations)
        .map(|(repository, decoration)| {
            let visibility = if repository.private {
                &labels.private
            } else {
                &labels.public
            };
            let status = [
                decoration.label.as_deref(),
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

    let rows = repositories.iter().zip(decorations).zip(columns).map(
        |((repository, decoration), [visibility, status, size, date])| {
            Row::new([
                Cell::from(decoration.marker),
                Cell::from(repository.name.clone()),
                Cell::from(visibility).fg(Color::DarkGray),
                Cell::from(status).fg(Color::DarkGray),
                Cell::from(Line::from(size).right_aligned()),
                Cell::from(date),
            ])
            .style(decoration.style)
        },
    );

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
        let screen = render(&State::new(Language::English, 'y', None), Language::English);

        assert!(screen.contains("Repository Cleaner"));
        assert!(screen.contains("› Protect repositories"));
        assert!(screen.contains("  Quit"));
        assert!(screen.contains("↑/↓ move · Enter select"));
        assert!(!screen.contains("Esc"));
    }

    #[test]
    fn main_menu_is_translated() {
        let screen = render(&State::new(Language::French, 'o', None), Language::French);

        assert!(screen.contains("Nettoyeur de dépôts"));
        assert!(screen.contains("› Protéger des dépôts"));
    }

    #[test]
    fn scope_menu_shows_a_breadcrumb_and_the_disabled_scope() {
        let mut state = State::new(Language::English, 'y', None);
        state.open_scope_menu(Action::Delete);

        let screen = render(&state, Language::English);

        assert!(screen.contains("Repository Cleaner › Delete repositories"));
        assert!(screen.contains("Only repositories since last deletion  no deletion yet"));
        assert!(screen.contains("Esc back"));
    }

    #[test]
    fn loading_screen_has_no_hint() {
        let mut state = State::new(Language::English, 'y', None);
        state.show_loading("Checking permissions…".to_owned());

        let screen = render(&state, Language::English);

        assert!(screen.contains("Checking permissions…"));
        assert!(!screen.contains("Press any key"));
    }

    #[test]
    fn protection_list_shows_the_header_markers_and_labels() {
        let mut state = State::new(Language::English, 'y', None);
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
        let mut state = State::new(Language::English, 'y', None);
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
    fn deletion_list_shows_marks_protected_rows_and_counts() {
        let mut state = State::new(Language::English, 'y', None);
        let repositories = ["alpha", "beta", "gamma"]
            .map(|name| Repository {
                name: name.to_owned(),
                private: false,
                fork: false,
                archived: false,
                size_kb: 0,
                created_at: "2021-01-01T12:00:00Z".parse().unwrap(),
                pushed_at: None,
            })
            .to_vec();
        state.open_deletion_list(repositories, ["gamma".to_owned()].into(), Scope::All);
        state.handle(Command::Toggle);

        let screen = render(&state, Language::English);

        assert!(screen.contains("3 repositories · 1 protected · 1 marked for deletion"));
        assert!(screen.contains("› [✗]  alpha"));
        let gamma = screen.lines().find(|line| line.contains("gamma")).unwrap();
        assert!(gamma.contains("[ ]  gamma") && gamma.contains("protected"));
    }

    #[test]
    fn deletion_list_warns_when_nothing_is_marked() {
        let mut state = State::new(Language::English, 'y', None);
        state.open_deletion_list(Vec::new(), BTreeSet::new(), Scope::All);
        state.handle(Command::Confirm);

        let screen = render(&state, Language::English);

        assert!(screen.contains("No repository selected"));
    }

    #[test]
    fn confirmation_shows_the_question_and_the_names() {
        let mut state = protection_with(3);
        let Screen::ProtectionList(protection) = state.screen() else {
            unreachable!()
        };
        let repositories = protection.list().repositories().to_vec();
        state.open_deletion_list(repositories, BTreeSet::new(), Scope::All);
        state.handle(Command::Toggle);
        state.handle(Command::Last);
        state.handle(Command::Toggle);
        state.handle(Command::Confirm);

        let screen = render(&state, Language::English);

        assert!(screen.contains("Do you really want to delete these 2 repositories? [y/N]"));
        assert!(screen.contains("repo-00, repo-02"));
    }

    #[test]
    fn confirmation_is_translated_with_its_key() {
        let mut state = State::new(Language::French, 'o', None);
        let repositories = vec![Repository {
            name: "alpha".to_owned(),
            private: false,
            fork: false,
            archived: false,
            size_kb: 0,
            created_at: "2021-01-01T12:00:00Z".parse().unwrap(),
            pushed_at: None,
        }];
        state.open_deletion_list(repositories, BTreeSet::new(), Scope::All);
        state.handle(Command::Toggle);
        state.handle(Command::Confirm);

        let screen = render(&state, Language::French);

        assert!(screen.contains("Voulez-vous vraiment supprimer ce dépôt\u{a0}? [o/N]"));
    }

    fn names(count: usize) -> Vec<String> {
        (0..count).map(|index| format!("repo-{index:02}")).collect()
    }

    fn more(count: usize) -> String {
        format!("… and {count} more")
    }

    #[test]
    fn names_are_packed_on_lines_separated_by_commas() {
        let lines = pack_names(&names(4), 18, 10, more);

        assert_eq!(lines, ["repo-00, repo-01,", "repo-02, repo-03"]);
    }

    #[test]
    fn names_that_do_not_fit_are_counted() {
        let lines = pack_names(&names(10), 36, 2, more);

        assert_eq!(
            lines,
            [
                "repo-00, repo-01, repo-02, repo-03,",
                "repo-04, repo-05, … and 4 more"
            ]
        );
        assert!(lines.iter().all(|line| line.chars().count() <= 36));
    }

    #[test]
    fn deletion_progress_shows_a_bar_with_the_count_and_the_percentage() {
        let mut state = State::new(Language::English, 'y', None);
        state.show_deletion_progress(4, 10);

        let screen = render(&state, Language::English);

        assert!(screen.contains("Deleting repositories…"));
        let bar = screen.lines().find(|line| line.contains("4/10")).unwrap();
        assert!(bar.trim_start().starts_with("4/10 · 40% ━"));
        let filled = bar.matches('━').count();
        let unfilled = bar.matches('─').count();
        assert_eq!(filled, (filled + unfilled) * 4 / 10);
        assert!(screen.contains("Esc stop after the current repository"));
    }

    #[test]
    fn deletion_progress_is_translated() {
        let mut state = State::new(Language::French, 'o', None);
        state.show_deletion_progress(1, 3);

        let screen = render(&state, Language::French);

        assert!(screen.contains("Suppression des dépôts…"));
        assert!(screen.contains("1/3 · 33\u{a0}% ━"));
    }

    #[test]
    fn percent_is_rounded_down_and_full_only_when_done() {
        assert_eq!(percent(0, 3), 0);
        assert_eq!(percent(1, 3), 33);
        assert_eq!(percent(199, 200), 99);
        assert_eq!(percent(3, 3), 100);
        assert_eq!(percent(0, 0), 100);
    }

    #[test]
    fn message_shows_how_to_continue() {
        let mut state = State::new(Language::English, 'y', None);
        state.show_message("Something went wrong".to_owned());

        let screen = render(&state, Language::English);

        assert!(screen.contains("Something went wrong"));
        assert!(screen.contains("Press any key to return to the menu"));
    }
}

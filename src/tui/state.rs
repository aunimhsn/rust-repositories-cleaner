//! State of the interface and key handling, independent of the terminal.
//!
//! Keys are first mapped to a [`Command`] by the keymap, then dispatched to
//! the handler of the current screen, which only changes the state. Side
//! effects (calling GitHub, saving the configuration, quitting) are returned
//! as an [`Effect`] for the event loop to run, which keeps this module
//! testable without a terminal or network.

use std::collections::BTreeSet;

use jiff::Timestamp;
use ratatui::crossterm::event::KeyEvent;

use super::keymap::{self, Command};
use super::list::{RepositoryList, in_scope};
use crate::github::Repository;
use crate::language::Language;

/// Entries of the main menu, in display order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainEntry {
    Protect,
    Delete,
    Language,
    Quit,
}

impl MainEntry {
    pub const ALL: [Self; 4] = [Self::Protect, Self::Delete, Self::Language, Self::Quit];
}

/// Action that works on a list of repositories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Protect,
    Delete,
}

/// Repositories to list, chosen in the scope menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    All,
    SinceLastDeletion,
}

impl Scope {
    pub const ALL: [Self; 2] = [Self::All, Self::SinceLastDeletion];
}

/// Screen currently displayed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Screen {
    MainMenu,
    ScopeMenu(Action),
    LanguageMenu,
    ProtectionList(ProtectionList),
    DeletionList(DeletionList),
    /// Confirmation of the deletion of the repositories marked in the list,
    /// which is kept to go back to it if the user cancels.
    ConfirmDeletion(DeletionList),
    /// Deletion in progress: number of repositories processed and total.
    DeletionProgress {
        done: usize,
        total: usize,
    },
    /// A waiting message, shown while a blocking call runs. Keys are ignored.
    Loading(String),
    /// A message, closed by any key to return to the main menu.
    Message(String),
}

/// List of repositories in which the user marks the ones to protect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtectionList {
    list: RepositoryList,
    /// Protected repositories, including those outside the list, which are kept.
    protected: BTreeSet<String>,
    /// Date of the last deletion, when only the newer repositories are listed.
    since: Option<Timestamp>,
}

impl ProtectionList {
    #[must_use]
    pub fn list(&self) -> &RepositoryList {
        &self.list
    }

    #[must_use]
    pub fn is_protected(&self, repository: &Repository) -> bool {
        self.protected.contains(&repository.name)
    }

    /// Number of protected repositories in the list.
    #[must_use]
    pub fn protected_count(&self) -> usize {
        self.list
            .repositories()
            .iter()
            .filter(|repository| self.is_protected(repository))
            .count()
    }

    #[must_use]
    pub fn since(&self) -> Option<Timestamp> {
        self.since
    }

    fn toggle_selected(&mut self) {
        if let Some(name) = self
            .list
            .selected()
            .map(|repository| repository.name.clone())
            && !self.protected.remove(&name)
        {
            self.protected.insert(name);
        }
    }
}

/// List of repositories in which the user marks the ones to delete.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeletionList {
    list: RepositoryList,
    /// Protected repositories, which cannot be marked.
    protected: BTreeSet<String>,
    marked: BTreeSet<String>,
    /// Date of the last deletion, when only the newer repositories are listed.
    since: Option<Timestamp>,
    /// Whether "No repository selected" is shown, until the next key.
    nothing_selected: bool,
}

impl DeletionList {
    #[must_use]
    pub fn list(&self) -> &RepositoryList {
        &self.list
    }

    #[must_use]
    pub fn is_protected(&self, repository: &Repository) -> bool {
        self.protected.contains(&repository.name)
    }

    #[must_use]
    pub fn is_marked(&self, repository: &Repository) -> bool {
        self.marked.contains(&repository.name)
    }

    /// Number of protected repositories in the list.
    #[must_use]
    pub fn protected_count(&self) -> usize {
        self.list
            .repositories()
            .iter()
            .filter(|repository| self.is_protected(repository))
            .count()
    }

    #[must_use]
    pub fn marked_count(&self) -> usize {
        self.marked.len()
    }

    /// Names of the marked repositories, in the order of the list.
    #[must_use]
    pub fn marked_names(&self) -> Vec<String> {
        self.list
            .repositories()
            .iter()
            .filter(|repository| self.is_marked(repository))
            .map(|repository| repository.name.clone())
            .collect()
    }

    #[must_use]
    pub fn since(&self) -> Option<Timestamp> {
        self.since
    }

    #[must_use]
    pub fn nothing_selected(&self) -> bool {
        self.nothing_selected
    }

    /// Toggles the mark of the selected repository, unless it is protected.
    fn toggle_selected(&mut self) {
        if let Some(repository) = self.list.selected()
            && !self.is_protected(repository)
        {
            let name = repository.name.clone();
            if !self.marked.remove(&name) {
                self.marked.insert(name);
            }
        }
    }
}

/// Side effect requested by a key, run by the event loop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    Quit,
    /// Check the `delete_repo` scope, then open the scope menu of the deletion.
    CheckDeleteScope,
    /// Save and apply a new interface language, different from the current one.
    ChangeLanguage(Language),
    /// Open the list of repositories for an action.
    OpenList(Action, Scope),
    /// Save the protected repositories.
    SaveProtections(BTreeSet<String>),
    /// Delete these repositories, confirmed by the user.
    DeleteRepositories(Vec<String>),
}

/// State of the interface.
#[derive(Debug)]
pub struct State {
    screen: Screen,
    /// Cursor in the main menu, kept when coming back to it.
    main_cursor: usize,
    /// Cursor in the scope or language menu.
    submenu_cursor: usize,
    language: Language,
    /// Key that confirms the deletion in the current language, in lowercase.
    confirmation_key: char,
    last_deletion: Option<Timestamp>,
}

impl State {
    #[must_use]
    pub fn new(
        language: Language,
        confirmation_key: char,
        last_deletion: Option<Timestamp>,
    ) -> Self {
        Self {
            screen: Screen::MainMenu,
            main_cursor: 0,
            submenu_cursor: 0,
            language,
            confirmation_key,
            last_deletion,
        }
    }

    #[must_use]
    pub fn screen(&self) -> &Screen {
        &self.screen
    }

    #[must_use]
    pub fn main_cursor(&self) -> usize {
        self.main_cursor
    }

    #[must_use]
    pub fn submenu_cursor(&self) -> usize {
        self.submenu_cursor
    }

    /// Returns whether a scope can be chosen: the repositories since last
    /// deletion need a previous deletion.
    #[must_use]
    pub fn is_scope_enabled(&self, scope: Scope) -> bool {
        match scope {
            Scope::All => true,
            Scope::SinceLastDeletion => self.last_deletion.is_some(),
        }
    }

    pub fn show_main_menu(&mut self) {
        self.screen = Screen::MainMenu;
    }

    pub fn open_scope_menu(&mut self, action: Action) {
        self.screen = Screen::ScopeMenu(action);
        self.submenu_cursor = 0;
    }

    /// Opens the protection list with the repositories of `scope`, among
    /// `repositories`, and the currently `protected` ones.
    pub fn open_protection_list(
        &mut self,
        repositories: Vec<Repository>,
        protected: BTreeSet<String>,
        scope: Scope,
    ) {
        self.screen = Screen::ProtectionList(ProtectionList {
            list: self.repository_list(repositories, scope),
            protected,
            since: self.since(scope),
        });
    }

    /// Opens the deletion list with the repositories of `scope`, among
    /// `repositories`; the `protected` ones cannot be marked.
    pub fn open_deletion_list(
        &mut self,
        repositories: Vec<Repository>,
        protected: BTreeSet<String>,
        scope: Scope,
    ) {
        self.screen = Screen::DeletionList(DeletionList {
            list: self.repository_list(repositories, scope),
            protected,
            marked: BTreeSet::new(),
            since: self.since(scope),
            nothing_selected: false,
        });
    }

    fn repository_list(&self, repositories: Vec<Repository>, scope: Scope) -> RepositoryList {
        RepositoryList::new(in_scope(repositories, scope, self.last_deletion))
    }

    /// Date shown in the header of a list limited to the newer repositories.
    fn since(&self, scope: Scope) -> Option<Timestamp> {
        match scope {
            Scope::All => None,
            Scope::SinceLastDeletion => self.last_deletion,
        }
    }

    /// Replaces the current screen by `next(screen)`.
    fn replace_screen(&mut self, next: impl FnOnce(Screen) -> Screen) {
        let screen = std::mem::replace(&mut self.screen, Screen::MainMenu);
        self.screen = next(screen);
    }

    pub fn show_deletion_progress(&mut self, done: usize, total: usize) {
        self.screen = Screen::DeletionProgress { done, total };
    }

    pub fn set_last_deletion(&mut self, last_deletion: Timestamp) {
        self.last_deletion = Some(last_deletion);
    }

    pub fn open_language_menu(&mut self) {
        self.screen = Screen::LanguageMenu;
        self.submenu_cursor = Language::ALL
            .iter()
            .position(|language| *language == self.language)
            .unwrap_or(0);
    }

    pub fn show_loading(&mut self, message: String) {
        self.screen = Screen::Loading(message);
    }

    pub fn show_message(&mut self, message: String) {
        self.screen = Screen::Message(message);
    }

    pub fn set_language(&mut self, language: Language, confirmation_key: char) {
        self.language = language;
        self.confirmation_key = confirmation_key;
    }

    /// Updates the state for `key` and returns the side effect to run, if any.
    pub fn handle_key(&mut self, key: KeyEvent) -> Option<Effect> {
        self.handle(keymap::command(key))
    }

    /// Dispatches `command` to the handler of the current screen.
    pub fn handle(&mut self, command: Command) -> Option<Effect> {
        if command == Command::Quit {
            return Some(Effect::Quit);
        }
        match &self.screen {
            Screen::MainMenu => self.handle_main_menu(command),
            Screen::ScopeMenu(action) => self.handle_scope_menu(*action, command),
            Screen::LanguageMenu => self.handle_language_menu(command),
            Screen::ProtectionList(_) => self.handle_protection_list(command),
            Screen::DeletionList(_) => self.handle_deletion_list(command),
            Screen::ConfirmDeletion(_) => self.handle_confirm_deletion(command),
            // While waiting, keys are ignored; while deleting, the event loop
            // reads them itself to stop.
            Screen::DeletionProgress { .. } | Screen::Loading(_) => None,
            Screen::Message(_) => {
                self.show_main_menu();
                None
            }
        }
    }

    fn handle_main_menu(&mut self, command: Command) -> Option<Effect> {
        match command {
            Command::Up | Command::Down => {
                self.main_cursor =
                    move_cursor(self.main_cursor, &[true; MainEntry::ALL.len()], command);
                None
            }
            Command::Confirm => match MainEntry::ALL[self.main_cursor] {
                MainEntry::Protect => {
                    self.open_scope_menu(Action::Protect);
                    None
                }
                MainEntry::Delete => Some(Effect::CheckDeleteScope),
                MainEntry::Language => {
                    self.open_language_menu();
                    None
                }
                MainEntry::Quit => Some(Effect::Quit),
            },
            // Esc does nothing in the main menu: the Quit entry exits.
            _ => None,
        }
    }

    fn handle_scope_menu(&mut self, action: Action, command: Command) -> Option<Effect> {
        match command {
            Command::Up | Command::Down => {
                let enabled = Scope::ALL.map(|scope| self.is_scope_enabled(scope));
                self.submenu_cursor = move_cursor(self.submenu_cursor, &enabled, command);
                None
            }
            Command::Confirm => {
                let scope = Scope::ALL[self.submenu_cursor];
                self.is_scope_enabled(scope)
                    .then_some(Effect::OpenList(action, scope))
            }
            Command::Back => {
                self.show_main_menu();
                None
            }
            _ => None,
        }
    }

    fn handle_language_menu(&mut self, command: Command) -> Option<Effect> {
        match command {
            Command::Up | Command::Down => {
                self.submenu_cursor =
                    move_cursor(self.submenu_cursor, &[true; Language::ALL.len()], command);
                None
            }
            Command::Confirm => {
                self.show_main_menu();
                let language = Language::ALL[self.submenu_cursor];
                // Saving the same language would only rewrite config.toml.
                (language != self.language).then_some(Effect::ChangeLanguage(language))
            }
            Command::Back => {
                self.show_main_menu();
                None
            }
            _ => None,
        }
    }

    fn handle_protection_list(&mut self, command: Command) -> Option<Effect> {
        let Screen::ProtectionList(protection) = &mut self.screen else {
            return None;
        };
        match command {
            Command::Toggle => {
                protection.toggle_selected();
                None
            }
            Command::Confirm => {
                let protected = std::mem::take(&mut protection.protected);
                self.show_main_menu();
                Some(Effect::SaveProtections(protected))
            }
            Command::Back => {
                self.show_main_menu();
                None
            }
            navigation => {
                protection.list.navigate(navigation);
                None
            }
        }
    }
}

impl State {
    fn handle_deletion_list(&mut self, command: Command) -> Option<Effect> {
        let Screen::DeletionList(deletion) = &mut self.screen else {
            return None;
        };
        deletion.nothing_selected = false;
        match command {
            Command::Toggle => deletion.toggle_selected(),
            Command::Confirm if deletion.marked.is_empty() => deletion.nothing_selected = true,
            Command::Confirm => self.replace_screen(|screen| match screen {
                Screen::DeletionList(deletion) => Screen::ConfirmDeletion(deletion),
                screen => screen,
            }),
            Command::Back => self.show_main_menu(),
            navigation => deletion.list.navigate(navigation),
        }
        None
    }

    fn handle_confirm_deletion(&mut self, command: Command) -> Option<Effect> {
        let confirmed = matches!(
            command,
            Command::Char(character) if character.to_lowercase().eq([self.confirmation_key])
        );
        if confirmed && let Screen::ConfirmDeletion(deletion) = &self.screen {
            // Protected repositories cannot be marked: filtering them again
            // guards the irreversible deletion against any future bug.
            let names = deletion
                .list
                .repositories()
                .iter()
                .filter(|repository| {
                    deletion.is_marked(repository) && !deletion.is_protected(repository)
                })
                .map(|repository| repository.name.clone())
                .collect();
            return Some(Effect::DeleteRepositories(names));
        }
        // Enter alone, Esc or any other key means "no": back to the list,
        // with the marks kept.
        self.replace_screen(|screen| match screen {
            Screen::ConfirmDeletion(deletion) => Screen::DeletionList(deletion),
            screen => screen,
        });
        None
    }
}

/// Moves `cursor` to the previous (`Up`) or next (`Down`) enabled entry,
/// staying in place at the ends of the menu.
fn move_cursor(cursor: usize, enabled: &[bool], command: Command) -> usize {
    let candidate = match command {
        Command::Up => (0..cursor).rev().find(|&index| enabled[index]),
        Command::Down => (cursor + 1..enabled.len()).find(|&index| enabled[index]),
        _ => None,
    };
    candidate.unwrap_or(cursor)
}

#[cfg(test)]
mod tests {
    use ratatui::crossterm::event::{KeyCode, KeyModifiers};

    use super::*;

    fn press(state: &mut State, code: KeyCode) -> Option<Effect> {
        state.handle_key(KeyEvent::from(code))
    }

    fn state() -> State {
        State::new(Language::English, 'y', None)
    }

    fn select_main_entry(state: &mut State, entry: MainEntry) {
        let index = MainEntry::ALL.iter().position(|e| *e == entry).unwrap();
        for _ in 0..index {
            press(state, KeyCode::Down);
        }
    }

    #[test]
    fn cursor_stays_inside_the_main_menu() {
        let mut state = state();

        press(&mut state, KeyCode::Up);
        assert_eq!(state.main_cursor(), 0);

        for _ in 0..10 {
            press(&mut state, KeyCode::Down);
        }
        assert_eq!(state.main_cursor(), MainEntry::ALL.len() - 1);
    }

    #[test]
    fn escape_does_nothing_in_the_main_menu() {
        let mut state = state();

        assert_eq!(press(&mut state, KeyCode::Esc), None);
        assert_eq!(state.screen(), &Screen::MainMenu);
    }

    #[test]
    fn quit_entry_quits() {
        let mut state = state();
        select_main_entry(&mut state, MainEntry::Quit);

        assert_eq!(press(&mut state, KeyCode::Enter), Some(Effect::Quit));
    }

    #[test]
    fn ctrl_c_quits_from_any_screen() {
        let mut state = state();
        state.open_scope_menu(Action::Protect);

        let effect = state.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));

        assert_eq!(effect, Some(Effect::Quit));
    }

    #[test]
    fn protect_opens_the_scope_menu() {
        let mut state = state();

        assert_eq!(press(&mut state, KeyCode::Enter), None);
        assert_eq!(state.screen(), &Screen::ScopeMenu(Action::Protect));
    }

    #[test]
    fn delete_checks_the_scope_first() {
        let mut state = state();
        select_main_entry(&mut state, MainEntry::Delete);

        assert_eq!(
            press(&mut state, KeyCode::Enter),
            Some(Effect::CheckDeleteScope)
        );
        assert_eq!(state.screen(), &Screen::MainMenu);
    }

    #[test]
    fn since_last_deletion_is_disabled_without_deletion() {
        let mut state = state();
        state.open_scope_menu(Action::Delete);

        press(&mut state, KeyCode::Down);

        assert_eq!(state.submenu_cursor(), 0);
        assert_eq!(
            press(&mut state, KeyCode::Enter),
            Some(Effect::OpenList(Action::Delete, Scope::All))
        );
    }

    #[test]
    fn since_last_deletion_is_enabled_after_a_deletion() {
        let mut state = State::new(Language::English, 'y', Some(Timestamp::UNIX_EPOCH));
        state.open_scope_menu(Action::Protect);

        press(&mut state, KeyCode::Down);

        assert_eq!(
            press(&mut state, KeyCode::Enter),
            Some(Effect::OpenList(Action::Protect, Scope::SinceLastDeletion))
        );
    }

    #[test]
    fn escape_leaves_submenus_and_keeps_the_main_cursor() {
        let mut state = state();
        select_main_entry(&mut state, MainEntry::Language);
        press(&mut state, KeyCode::Enter);

        assert_eq!(press(&mut state, KeyCode::Esc), None);
        assert_eq!(state.screen(), &Screen::MainMenu);
        assert_eq!(MainEntry::ALL[state.main_cursor()], MainEntry::Language);
    }

    #[test]
    fn language_menu_starts_on_the_current_language() {
        let mut state = State::new(Language::French, 'o', None);
        select_main_entry(&mut state, MainEntry::Language);

        press(&mut state, KeyCode::Enter);

        assert_eq!(state.screen(), &Screen::LanguageMenu);
        assert_eq!(Language::ALL[state.submenu_cursor()], Language::French);
    }

    #[test]
    fn language_menu_changes_the_language() {
        let mut state = state();
        select_main_entry(&mut state, MainEntry::Language);
        press(&mut state, KeyCode::Enter);
        press(&mut state, KeyCode::Down);

        assert_eq!(
            press(&mut state, KeyCode::Enter),
            Some(Effect::ChangeLanguage(Language::French))
        );
        assert_eq!(state.screen(), &Screen::MainMenu);
    }

    #[test]
    fn choosing_the_current_language_changes_nothing() {
        let mut state = state();
        state.open_language_menu();

        assert_eq!(press(&mut state, KeyCode::Enter), None);
        assert_eq!(state.screen(), &Screen::MainMenu);
    }

    #[test]
    fn escape_leaves_the_scope_menu() {
        let mut state = state();
        state.open_scope_menu(Action::Delete);

        assert_eq!(press(&mut state, KeyCode::Esc), None);
        assert_eq!(state.screen(), &Screen::MainMenu);
    }

    #[test]
    fn main_cursor_is_kept_after_a_submenu() {
        let mut state = state();
        select_main_entry(&mut state, MainEntry::Delete);
        state.open_scope_menu(Action::Delete);

        press(&mut state, KeyCode::Esc);

        assert_eq!(MainEntry::ALL[state.main_cursor()], MainEntry::Delete);
    }

    #[test]
    fn ctrl_c_quits_from_a_message() {
        let mut state = state();
        state.show_message("Error".to_owned());

        let effect = state.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));

        assert_eq!(effect, Some(Effect::Quit));
    }

    #[test]
    fn keys_are_ignored_while_loading() {
        let mut state = state();
        state.show_loading("Loading".to_owned());

        assert_eq!(press(&mut state, KeyCode::Enter), None);
        assert_eq!(state.screen(), &Screen::Loading("Loading".to_owned()));
    }

    fn repository(name: &str) -> Repository {
        Repository {
            name: name.to_owned(),
            private: false,
            fork: false,
            archived: false,
            size_kb: 0,
            created_at: "2021-01-01T00:00:00Z".parse().unwrap(),
            pushed_at: None,
        }
    }

    fn protection_list(state: &State) -> &ProtectionList {
        match state.screen() {
            Screen::ProtectionList(list) => list,
            screen => panic!("not a protection list: {screen:?}"),
        }
    }

    fn open_protection(state: &mut State, protected: &[&str]) {
        state.open_protection_list(
            vec![repository("alpha"), repository("beta"), repository("gamma")],
            protected.iter().map(|name| (*name).to_owned()).collect(),
            Scope::All,
        );
    }

    #[test]
    fn space_toggles_the_protection_of_the_selected_repository() {
        let mut state = state();
        open_protection(&mut state, &["gamma"]);

        press(&mut state, KeyCode::Down);
        press(&mut state, KeyCode::Char(' '));
        assert_eq!(protection_list(&state).protected_count(), 2);

        press(&mut state, KeyCode::Char(' '));
        assert_eq!(protection_list(&state).protected_count(), 1);
    }

    #[test]
    fn enter_saves_the_protections_including_unlisted_ones() {
        let mut state = state();
        open_protection(&mut state, &["deleted-meanwhile"]);
        press(&mut state, KeyCode::Char(' '));

        let effect = press(&mut state, KeyCode::Enter);

        let expected = BTreeSet::from(["alpha".to_owned(), "deleted-meanwhile".to_owned()]);
        assert_eq!(effect, Some(Effect::SaveProtections(expected)));
        assert_eq!(state.screen(), &Screen::MainMenu);
    }

    #[test]
    fn escape_discards_the_protection_changes() {
        let mut state = state();
        open_protection(&mut state, &[]);
        press(&mut state, KeyCode::Char(' '));

        assert_eq!(press(&mut state, KeyCode::Esc), None);
        assert_eq!(state.screen(), &Screen::MainMenu);
    }

    #[test]
    fn since_last_deletion_lists_only_newer_repositories() {
        let last_deletion: Timestamp = "2026-01-01T00:00:00Z".parse().unwrap();
        let mut state = State::new(Language::English, 'y', Some(last_deletion));
        let mut newer = repository("newer");
        newer.created_at = "2026-06-01T00:00:00Z".parse().unwrap();

        state.open_protection_list(
            vec![repository("older"), newer],
            BTreeSet::new(),
            Scope::SinceLastDeletion,
        );

        let list = protection_list(&state);
        assert_eq!(list.list().repositories().len(), 1);
        assert_eq!(list.since(), Some(last_deletion));
    }

    fn open_deletion(state: &mut State, protected: &[&str]) {
        state.open_deletion_list(
            vec![repository("alpha"), repository("beta"), repository("gamma")],
            protected.iter().map(|name| (*name).to_owned()).collect(),
            Scope::All,
        );
    }

    fn deletion_list(state: &State) -> &DeletionList {
        match state.screen() {
            Screen::DeletionList(deletion) | Screen::ConfirmDeletion(deletion) => deletion,
            screen => panic!("not a deletion screen: {screen:?}"),
        }
    }

    #[test]
    fn space_marks_and_unmarks_a_repository() {
        let mut state = state();
        open_deletion(&mut state, &[]);

        state.handle(Command::Toggle);
        assert_eq!(deletion_list(&state).marked_count(), 1);

        state.handle(Command::Toggle);
        assert_eq!(deletion_list(&state).marked_count(), 0);
    }

    #[test]
    fn protected_repositories_cannot_be_marked() {
        let mut state = state();
        open_deletion(&mut state, &["alpha"]);

        state.handle(Command::Toggle);

        assert_eq!(deletion_list(&state).marked_count(), 0);
    }

    #[test]
    fn enter_without_mark_shows_a_notice_until_the_next_key() {
        let mut state = state();
        open_deletion(&mut state, &[]);

        state.handle(Command::Confirm);
        assert!(deletion_list(&state).nothing_selected());
        assert!(matches!(state.screen(), Screen::DeletionList(_)));

        state.handle(Command::Down);
        assert!(!deletion_list(&state).nothing_selected());
    }

    #[test]
    fn enter_with_marks_asks_for_confirmation() {
        let mut state = state();
        open_deletion(&mut state, &[]);
        state.handle(Command::Down);
        state.handle(Command::Toggle);
        state.handle(Command::Last);
        state.handle(Command::Toggle);

        state.handle(Command::Confirm);

        assert!(matches!(state.screen(), Screen::ConfirmDeletion(_)));
        assert_eq!(deletion_list(&state).marked_names(), ["beta", "gamma"]);
    }

    fn confirm(state: &mut State) {
        state.handle(Command::Toggle);
        state.handle(Command::Last);
        state.handle(Command::Toggle);
        state.handle(Command::Confirm);
    }

    #[test]
    fn the_confirmation_key_deletes_exactly_the_marked_repositories() {
        let mut state = state();
        open_deletion(&mut state, &[]);
        confirm(&mut state);

        assert_eq!(
            state.handle(Command::Char('y')),
            Some(Effect::DeleteRepositories(vec![
                "alpha".to_owned(),
                "gamma".to_owned()
            ]))
        );
    }

    #[test]
    fn the_confirmation_key_works_in_uppercase() {
        let mut state = state();
        open_deletion(&mut state, &[]);
        confirm(&mut state);

        assert!(matches!(
            state.handle(Command::Char('Y')),
            Some(Effect::DeleteRepositories(_))
        ));
    }

    #[test]
    fn another_character_cancels_and_keeps_the_marks() {
        let mut state = state();
        open_deletion(&mut state, &[]);
        confirm(&mut state);

        assert_eq!(state.handle(Command::Char('n')), None);

        assert!(matches!(state.screen(), Screen::DeletionList(_)));
        assert_eq!(deletion_list(&state).marked_count(), 2);
    }

    #[test]
    fn the_english_key_does_not_confirm_in_french() {
        let mut state = State::new(Language::French, 'o', None);
        open_deletion(&mut state, &[]);
        confirm(&mut state);

        assert_eq!(state.handle(Command::Char('y')), None);

        state.handle(Command::Confirm);
        assert!(matches!(
            state.handle(Command::Char('o')),
            Some(Effect::DeleteRepositories(_))
        ));
    }

    #[test]
    fn enter_alone_cancels_and_keeps_the_marks() {
        let mut state = state();
        open_deletion(&mut state, &[]);
        state.handle(Command::Toggle);
        state.handle(Command::Confirm);

        assert_eq!(state.handle(Command::Confirm), None);

        assert!(matches!(state.screen(), Screen::DeletionList(_)));
        assert_eq!(deletion_list(&state).marked_count(), 1);
    }

    #[test]
    fn escape_leaves_the_deletion_list() {
        let mut state = state();
        open_deletion(&mut state, &[]);
        state.handle(Command::Toggle);

        state.handle(Command::Back);

        assert_eq!(state.screen(), &Screen::MainMenu);
    }

    #[test]
    fn any_key_closes_a_message() {
        let mut state = state();
        state.show_message("Error".to_owned());

        assert_eq!(press(&mut state, KeyCode::Char('x')), None);
        assert_eq!(state.screen(), &Screen::MainMenu);
    }

    #[test]
    fn cursor_skips_disabled_entries() {
        let enabled = [true, false, true];

        assert_eq!(move_cursor(0, &enabled, Command::Down), 2);
        assert_eq!(move_cursor(2, &enabled, Command::Up), 0);
        assert_eq!(move_cursor(0, &[true, false], Command::Down), 0);
    }
}

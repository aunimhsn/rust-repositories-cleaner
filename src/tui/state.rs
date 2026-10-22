//! State of the interface and key handling, independent of the terminal.
//!
//! Keys only change the state. Side effects (calling GitHub, saving the
//! configuration, quitting) are returned as an [`Effect`] for the event loop
//! to run, which keeps this module testable without a terminal or network.

use jiff::Timestamp;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

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
    /// A waiting message, shown while a blocking call runs. Keys are ignored.
    Loading(String),
    /// A message, closed by any key to return to the main menu.
    Message(String),
}

/// Side effect requested by a key, run by the event loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    Quit,
    /// Check the `delete_repo` scope, then open the scope menu of the deletion.
    CheckDeleteScope,
    /// Save and apply a new interface language, different from the current one.
    ChangeLanguage(Language),
    /// Open the list of repositories for an action.
    OpenList(Action, Scope),
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
    last_deletion: Option<Timestamp>,
}

impl State {
    #[must_use]
    pub fn new(language: Language, last_deletion: Option<Timestamp>) -> Self {
        Self {
            screen: Screen::MainMenu,
            main_cursor: 0,
            submenu_cursor: 0,
            language,
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

    pub fn set_language(&mut self, language: Language) {
        self.language = language;
    }

    /// Updates the state for `key` and returns the side effect to run, if any.
    pub fn handle_key(&mut self, key: KeyEvent) -> Option<Effect> {
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return Some(Effect::Quit);
        }

        match self.screen {
            Screen::MainMenu => self.handle_main_menu(key.code),
            Screen::ScopeMenu(action) => self.handle_scope_menu(action, key.code),
            Screen::LanguageMenu => self.handle_language_menu(key.code),
            Screen::Loading(_) => None,
            Screen::Message(_) => {
                self.show_main_menu();
                None
            }
        }
    }

    fn handle_main_menu(&mut self, code: KeyCode) -> Option<Effect> {
        match code {
            KeyCode::Up | KeyCode::Down => {
                self.main_cursor =
                    move_cursor(self.main_cursor, &[true; MainEntry::ALL.len()], code);
                None
            }
            KeyCode::Enter => match MainEntry::ALL[self.main_cursor] {
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

    fn handle_scope_menu(&mut self, action: Action, code: KeyCode) -> Option<Effect> {
        match code {
            KeyCode::Up | KeyCode::Down => {
                let enabled = Scope::ALL.map(|scope| self.is_scope_enabled(scope));
                self.submenu_cursor = move_cursor(self.submenu_cursor, &enabled, code);
                None
            }
            KeyCode::Enter => {
                let scope = Scope::ALL[self.submenu_cursor];
                self.is_scope_enabled(scope)
                    .then_some(Effect::OpenList(action, scope))
            }
            KeyCode::Esc => {
                self.show_main_menu();
                None
            }
            _ => None,
        }
    }

    fn handle_language_menu(&mut self, code: KeyCode) -> Option<Effect> {
        match code {
            KeyCode::Up | KeyCode::Down => {
                self.submenu_cursor =
                    move_cursor(self.submenu_cursor, &[true; Language::ALL.len()], code);
                None
            }
            KeyCode::Enter => {
                self.show_main_menu();
                let language = Language::ALL[self.submenu_cursor];
                // Saving the same language would only rewrite config.toml.
                (language != self.language).then_some(Effect::ChangeLanguage(language))
            }
            KeyCode::Esc => {
                self.show_main_menu();
                None
            }
            _ => None,
        }
    }
}

/// Moves `cursor` to the previous (`Up`) or next (`Down`) enabled entry,
/// staying in place at the ends of the menu.
fn move_cursor(cursor: usize, enabled: &[bool], code: KeyCode) -> usize {
    let candidate = match code {
        KeyCode::Up => (0..cursor).rev().find(|&index| enabled[index]),
        KeyCode::Down => (cursor + 1..enabled.len()).find(|&index| enabled[index]),
        _ => None,
    };
    candidate.unwrap_or(cursor)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(state: &mut State, code: KeyCode) -> Option<Effect> {
        state.handle_key(KeyEvent::from(code))
    }

    fn state() -> State {
        State::new(Language::English, None)
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
        let mut state = State::new(Language::English, Some(Timestamp::UNIX_EPOCH));
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
        let mut state = State::new(Language::French, None);
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

        assert_eq!(move_cursor(0, &enabled, KeyCode::Down), 2);
        assert_eq!(move_cursor(2, &enabled, KeyCode::Up), 0);
        assert_eq!(move_cursor(0, &[true, false], KeyCode::Down), 0);
    }
}

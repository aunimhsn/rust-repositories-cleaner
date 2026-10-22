//! Keyboard shortcuts: the only place where keys are mapped to commands.
//!
//! Screens never see keys, only the [`Command`] they mean, so changing a
//! shortcut only changes this module.

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// What the user asks for, whatever the key used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Up,
    Down,
    /// Jump a few rows up (`Shift + ↑`, or `PgUp`).
    JumpUp,
    /// Jump a few rows down (`Shift + ↓`, or `PgDn`).
    JumpDown,
    /// Go to the first row (`Ctrl + ↑`, or `Home`).
    First,
    /// Go to the last row (`Ctrl + ↓`, or `End`).
    Last,
    /// Toggle the mark of the current row (`Space`).
    Toggle,
    /// Validate (`Enter`).
    Confirm,
    /// Go back or cancel (`Esc`).
    Back,
    /// Quit from any screen (`Ctrl + C`).
    Quit,
    /// Any other key, which only closes messages.
    Other,
}

/// Maps a key press to its command.
#[must_use]
pub fn command(key: KeyEvent) -> Command {
    let control = key.modifiers.contains(KeyModifiers::CONTROL);
    let shift = key.modifiers.contains(KeyModifiers::SHIFT);

    match key.code {
        KeyCode::Char('c') if control => Command::Quit,
        KeyCode::Up if control => Command::First,
        KeyCode::Down if control => Command::Last,
        KeyCode::Up if shift => Command::JumpUp,
        KeyCode::Down if shift => Command::JumpDown,
        KeyCode::Up => Command::Up,
        KeyCode::Down => Command::Down,
        KeyCode::PageUp => Command::JumpUp,
        KeyCode::PageDown => Command::JumpDown,
        KeyCode::Home => Command::First,
        KeyCode::End => Command::Last,
        KeyCode::Char(' ') => Command::Toggle,
        KeyCode::Enter => Command::Confirm,
        KeyCode::Esc => Command::Back,
        _ => Command::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode, modifiers: KeyModifiers) -> Command {
        command(KeyEvent::new(code, modifiers))
    }

    #[test]
    fn arrows_move_by_one() {
        assert_eq!(key(KeyCode::Up, KeyModifiers::NONE), Command::Up);
        assert_eq!(key(KeyCode::Down, KeyModifiers::NONE), Command::Down);
    }

    #[test]
    fn shift_arrows_and_page_keys_jump() {
        assert_eq!(key(KeyCode::Up, KeyModifiers::SHIFT), Command::JumpUp);
        assert_eq!(key(KeyCode::Down, KeyModifiers::SHIFT), Command::JumpDown);
        assert_eq!(key(KeyCode::PageUp, KeyModifiers::NONE), Command::JumpUp);
        assert_eq!(
            key(KeyCode::PageDown, KeyModifiers::NONE),
            Command::JumpDown
        );
    }

    #[test]
    fn control_arrows_and_home_end_go_to_the_ends() {
        assert_eq!(key(KeyCode::Up, KeyModifiers::CONTROL), Command::First);
        assert_eq!(key(KeyCode::Down, KeyModifiers::CONTROL), Command::Last);
        assert_eq!(key(KeyCode::Home, KeyModifiers::NONE), Command::First);
        assert_eq!(key(KeyCode::End, KeyModifiers::NONE), Command::Last);
    }

    #[test]
    fn control_wins_over_shift() {
        let both = KeyModifiers::CONTROL | KeyModifiers::SHIFT;

        assert_eq!(key(KeyCode::Down, both), Command::Last);
    }

    #[test]
    fn action_keys() {
        assert_eq!(key(KeyCode::Char(' '), KeyModifiers::NONE), Command::Toggle);
        assert_eq!(key(KeyCode::Enter, KeyModifiers::NONE), Command::Confirm);
        assert_eq!(key(KeyCode::Esc, KeyModifiers::NONE), Command::Back);
        assert_eq!(
            key(KeyCode::Char('c'), KeyModifiers::CONTROL),
            Command::Quit
        );
        assert_eq!(key(KeyCode::Char('c'), KeyModifiers::NONE), Command::Other);
    }
}

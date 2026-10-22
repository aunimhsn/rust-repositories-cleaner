//! Scrollable list of repositories, shared by the protection and deletion screens.
//!
//! The cursor stays in the middle of the visible rows, so the repositories
//! around it, before and after, are always visible. Only at the ends of the
//! list does the cursor move towards the edge.

use jiff::Timestamp;

use super::keymap::Command;
use super::state::Scope;
use crate::github::Repository;

/// Number of rows moved by a jump (`Shift + ↑/↓`).
const JUMP_SIZE: usize = 5;

/// Repositories with the position of the cursor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryList {
    repositories: Vec<Repository>,
    /// Index of the selected repository, `None` when the list is empty.
    cursor: Option<usize>,
}

impl RepositoryList {
    #[must_use]
    pub fn new(repositories: Vec<Repository>) -> Self {
        let cursor = (!repositories.is_empty()).then_some(0);
        Self {
            repositories,
            cursor,
        }
    }

    #[must_use]
    pub fn repositories(&self) -> &[Repository] {
        &self.repositories
    }

    #[must_use]
    pub fn cursor(&self) -> Option<usize> {
        self.cursor
    }

    /// Repository under the cursor, if the list is not empty.
    #[must_use]
    pub fn selected(&self) -> Option<&Repository> {
        self.cursor.and_then(|index| self.repositories.get(index))
    }

    /// Moves the cursor for a navigation command; other commands are ignored.
    pub fn navigate(&mut self, command: Command) {
        let (Some(cursor), Some(last)) = (self.cursor, self.repositories.len().checked_sub(1))
        else {
            return;
        };
        let target = match command {
            Command::Up => cursor.saturating_sub(1),
            Command::Down => cursor + 1,
            Command::JumpUp => cursor.saturating_sub(JUMP_SIZE),
            Command::JumpDown => cursor + JUMP_SIZE,
            Command::First => 0,
            Command::Last => last,
            _ => return,
        };
        self.cursor = Some(target.min(last));
    }
}

/// Index of the first visible row that keeps `cursor` in the middle of
/// `height` visible rows, without scrolling past the ends of `len` rows.
#[must_use]
pub fn centered_offset(cursor: usize, len: usize, height: usize) -> usize {
    let max_offset = len.saturating_sub(height);
    cursor.saturating_sub(height / 2).min(max_offset)
}

/// Keeps the repositories of `scope`: all of them, or those created after the
/// last deletion.
#[must_use]
pub fn in_scope(
    repositories: Vec<Repository>,
    scope: Scope,
    last_deletion: Option<Timestamp>,
) -> Vec<Repository> {
    match (scope, last_deletion) {
        (Scope::SinceLastDeletion, Some(last_deletion)) => repositories
            .into_iter()
            .filter(|repository| repository.created_at > last_deletion)
            .collect(),
        // The scope menu only offers "since last deletion" after a deletion.
        _ => repositories,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repository(name: &str, created_at: &str) -> Repository {
        Repository {
            name: name.to_owned(),
            private: false,
            fork: false,
            archived: false,
            size_kb: 0,
            created_at: created_at.parse().unwrap(),
            pushed_at: None,
        }
    }

    fn list(count: usize) -> RepositoryList {
        RepositoryList::new(
            (0..count)
                .map(|index| repository(&format!("repo-{index}"), "2021-01-01T00:00:00Z"))
                .collect(),
        )
    }

    fn cursor(list: &RepositoryList) -> usize {
        list.cursor().unwrap()
    }

    #[test]
    fn cursor_starts_on_the_first_repository() {
        assert_eq!(list(3).selected().unwrap().name, "repo-0");
    }

    #[test]
    fn cursor_stays_inside_the_list() {
        let mut list = list(3);

        list.navigate(Command::Up);
        assert_eq!(cursor(&list), 0);

        for _ in 0..5 {
            list.navigate(Command::Down);
        }
        assert_eq!(cursor(&list), 2);
    }

    #[test]
    fn jumps_move_by_five_and_stop_at_the_ends() {
        let mut list = list(12);

        list.navigate(Command::JumpDown);
        assert_eq!(cursor(&list), 5);
        list.navigate(Command::JumpDown);
        list.navigate(Command::JumpDown);
        assert_eq!(cursor(&list), 11);
        list.navigate(Command::JumpUp);
        assert_eq!(cursor(&list), 6);
    }

    #[test]
    fn first_and_last_go_to_the_ends() {
        let mut list = list(25);

        list.navigate(Command::Last);
        assert_eq!(cursor(&list), 24);
        list.navigate(Command::First);
        assert_eq!(cursor(&list), 0);
    }

    #[test]
    fn empty_list_has_no_selection() {
        let mut list = list(0);

        list.navigate(Command::Down);

        assert!(list.selected().is_none());
    }

    #[test]
    fn other_commands_do_not_move() {
        let mut list = list(3);

        list.navigate(Command::Toggle);

        assert_eq!(cursor(&list), 0);
    }

    #[test]
    fn offset_keeps_the_cursor_in_the_middle() {
        assert_eq!(centered_offset(50, 332, 17), 42);
        assert_eq!(centered_offset(51, 332, 17), 43);
    }

    #[test]
    fn offset_stops_at_the_ends_of_the_list() {
        assert_eq!(centered_offset(3, 332, 17), 0);
        assert_eq!(centered_offset(331, 332, 17), 315);
    }

    #[test]
    fn short_lists_never_scroll() {
        assert_eq!(centered_offset(4, 5, 17), 0);
    }

    #[test]
    fn since_last_deletion_keeps_newer_repositories() {
        let repositories = vec![
            repository("old", "2025-01-01T00:00:00Z"),
            repository("new", "2026-06-01T00:00:00Z"),
        ];
        let last_deletion = "2026-01-01T00:00:00Z".parse().ok();

        let kept = in_scope(repositories, Scope::SinceLastDeletion, last_deletion);

        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].name, "new");
    }

    #[test]
    fn all_scope_keeps_every_repository() {
        let repositories = vec![repository("old", "2025-01-01T00:00:00Z")];

        assert_eq!(in_scope(repositories, Scope::All, None).len(), 1);
    }
}

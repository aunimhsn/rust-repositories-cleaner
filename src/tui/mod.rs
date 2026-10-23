//! Terminal user interface: event loop running the menus.

mod deletion;
mod format;
mod keymap;
mod list;
mod render;
mod state;

use std::collections::BTreeSet;
use std::fs::TryLockError;
use std::io;
use std::path::Path;
use std::time::Duration;

use i18n_embed::fluent::FluentLanguageLoader;
use jiff::Timestamp;
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event, KeyEventKind};

use self::deletion::MissingLine;
use self::state::{Action, Effect, Scope, State};
use crate::config::Config;
use crate::deletion_log::{self, LOG_FILE};
use crate::github::{GitHub, GitHubError};
use crate::i18n::{self, fl, github_error_message};
use crate::language::Language;

/// Runs the interface until the user quits.
///
/// # Errors
///
/// Returns an error if the terminal cannot be drawn or read.
pub fn run(
    config: Config,
    config_path: &Path,
    loader: FluentLanguageLoader,
    github: &GitHub,
) -> io::Result<()> {
    // The last deletion date enables the "since last deletion" scope.
    let last_deletion = deletion_log::read_last_deletion(Path::new(LOG_FILE));
    let mut state = State::new(
        config.language,
        i18n::confirmation_key(&loader),
        last_deletion.as_ref().ok().copied().flatten(),
    );
    if let Err(error) = last_deletion {
        state.show_message(fl!(loader, "error-log", details = error.to_string()));
    }
    // A deletion session cut short (application killed, terminal closed…) is
    // recorded as interrupted, and the user is told at once.
    match deletion_log::close_unfinished_session(Path::new(LOG_FILE), Timestamp::now()) {
        Ok(Some(session)) => {
            let mut message = fl!(loader, "previous-session-interrupted", path = LOG_FILE);
            if let Some(repository) = session.pending {
                message.push('\n');
                message.push_str(&fl!(
                    loader,
                    "previous-session-pending",
                    repository = repository
                ));
            }
            state.show_message(message);
        }
        Ok(None) => {}
        Err(error) => state.show_message(fl!(loader, "error-log", details = error.to_string())),
    }
    let mut app = App {
        state,
        config,
        config_path,
        loader,
        github,
    };
    ratatui::run(|terminal| app.run(terminal))
}

/// Everything the event loop needs to run the effects.
struct App<'a> {
    state: State,
    config: Config,
    config_path: &'a Path,
    loader: FluentLanguageLoader,
    github: &'a GitHub,
}

impl App<'_> {
    fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        loop {
            self.draw(terminal)?;

            let Event::Key(key) = event::read()? else {
                continue;
            };
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match self.state.handle_key(key) {
                None => {}
                Some(Effect::Quit) => return Ok(()),
                Some(Effect::CheckDeleteScope) => {
                    self.state.show_loading(fl!(self.loader, "checking-access"));
                    self.draw(terminal)?;
                    self.check_delete_scope();
                    discard_pending_events()?;
                }
                Some(Effect::ChangeLanguage(language)) => self.change_language(language),
                Some(Effect::OpenList(action, scope)) => {
                    self.state
                        .show_loading(fl!(self.loader, "loading-repositories"));
                    self.draw(terminal)?;
                    self.open_list(action, scope);
                    discard_pending_events()?;
                }
                Some(Effect::DeleteRepositories(names)) => {
                    self.delete(&names, terminal);
                    discard_pending_events()?;
                }
                Some(Effect::SaveProtections(protected)) => self.save_protections(protected),
            }
        }
    }

    fn draw(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        terminal.draw(|frame| render::draw(frame, &self.state, &self.loader))?;
        Ok(())
    }

    fn check_delete_scope(&mut self) {
        match self.github.check_delete_scope() {
            Ok(()) => self.state.open_scope_menu(Action::Delete),
            Err(error) => self
                .state
                .show_message(github_error_message(&self.loader, &error)),
        }
    }

    fn open_list(&mut self, action: Action, scope: Scope) {
        match self.github.repositories() {
            Ok(repositories) => {
                let protected = self.config.protected.clone();
                match action {
                    Action::Protect => {
                        self.state
                            .open_protection_list(repositories, protected, scope);
                    }
                    Action::Delete => self
                        .state
                        .open_deletion_list(repositories, protected, scope),
                }
            }
            Err(error) => self
                .state
                .show_message(github_error_message(&self.loader, &error)),
        }
    }

    /// Deletes the repositories confirmed by the user, showing the progress,
    /// then shows the summary. `Esc` or `Ctrl + C` stops after the current one.
    fn delete(&mut self, names: &[String], terminal: &mut DefaultTerminal) {
        let mut log = match deletion_log::open(Path::new(LOG_FILE)) {
            Ok(log) => log,
            Err(error) => {
                let message = fl!(self.loader, "error-log", details = error.to_string());
                self.state.show_message(message);
                return;
            }
        };
        // The lock, released when `log` is dropped, keeps another instance
        // from writing to the log or closing this session as interrupted.
        match log.try_lock() {
            Ok(()) => {}
            Err(TryLockError::WouldBlock) => {
                self.state.show_message(fl!(self.loader, "log-in-use"));
                return;
            }
            Err(TryLockError::Error(error)) => {
                let message = fl!(self.loader, "error-log", details = error.to_string());
                self.state.show_message(message);
                return;
            }
        }

        let github = self.github;
        let loader = &self.loader;
        let state = &mut self.state;
        let result = deletion::delete_all(
            names,
            github.login(),
            |name| github.delete_repository(name).map_err(failure_reason),
            &mut log,
            Timestamp::now,
            |done, total| {
                state.show_deletion_progress(done, total);
                // A progress frame that cannot be drawn must not interrupt the
                // deletions: the next frame or the summary will be drawn again.
                let _ = terminal.draw(|frame| render::draw(frame, state, loader));
            },
            stop_requested,
        );

        let (summary, error) = match result {
            Ok(summary) => (summary, None),
            Err(error) => (error.summary, Some((error.missing, error.source))),
        };
        let mut lines = Vec::new();
        if let Some((_, source)) = &error {
            lines.push(fl!(self.loader, "error-log", details = source.to_string()));
        }
        lines.push(fl!(
            self.loader,
            "deletion-summary",
            deleted = summary.deleted,
            failed = summary.failed,
            path = LOG_FILE
        ));
        if summary.not_processed > 0 {
            lines.push(fl!(
                self.loader,
                "deletion-untouched",
                count = summary.not_processed
            ));
        }
        match error.map(|(missing, _)| missing) {
            Some(MissingLine::Result(repository)) => {
                lines.push(fl!(
                    self.loader,
                    "log-missing-result",
                    repository = repository
                ));
            }
            Some(MissingLine::End) => lines.push(fl!(self.loader, "log-missing-end")),
            // The other repositories were not deleted: they are counted as untouched.
            _ => {}
        }

        if let Some(last_deleted_at) = summary.last_deleted_at {
            self.state.set_last_deletion(last_deleted_at);
        }
        self.state.show_message(lines.join("\n"));
    }

    /// Saves the protected repositories. Nothing changes if saving fails.
    fn save_protections(&mut self, protected: BTreeSet<String>) {
        match with_protections(&self.config, self.config_path, protected) {
            Ok(config) => self.config = config,
            Err(details) => {
                let message = fl!(self.loader, "error-config-save", details = details);
                self.state.show_message(message);
            }
        }
    }

    /// Applies and saves a new language. Nothing changes if it fails.
    fn change_language(&mut self, language: Language) {
        match switch_language(&self.config, self.config_path, language) {
            Ok((config, loader)) => {
                self.config = config;
                self.loader = loader;
                self.state
                    .set_language(language, i18n::confirmation_key(&self.loader));
            }
            Err(details) => {
                let message = fl!(self.loader, "error-config-save", details = details);
                self.state.show_message(message);
            }
        }
    }
}

/// Loads the translations of `language`, then saves it to `path`. Loading
/// first means the file never names a language that cannot be displayed.
fn switch_language(
    config: &Config,
    path: &Path,
    language: Language,
) -> Result<(Config, FluentLanguageLoader), String> {
    let loader = i18n::load(language).map_err(|error| error.to_string())?;
    let config = Config {
        language,
        ..config.clone()
    };
    config.save(path).map_err(|error| error.to_string())?;
    Ok((config, loader))
}

/// Returns `config` with the `protected` repositories, saved to `path` only
/// if they changed, so that an unchanged file is never rewritten.
fn with_protections(
    config: &Config,
    path: &Path,
    protected: BTreeSet<String>,
) -> Result<Config, String> {
    if protected == config.protected {
        return Ok(config.clone());
    }
    let config = Config {
        protected,
        ..config.clone()
    };
    config.save(path).map_err(|error| error.to_string())?;
    Ok(config)
}

/// Reason of a failed deletion for the log: GitHub's message, without prefix.
fn failure_reason(error: GitHubError) -> String {
    match error {
        GitHubError::Request(details) => details,
        error => error.to_string(),
    }
}

/// Returns whether the user pressed `Esc` or `Ctrl + C` to stop the deletions.
/// Other keys pressed meanwhile are dropped.
fn stop_requested() -> bool {
    let mut stop = false;
    while let Ok(true) = event::poll(Duration::ZERO) {
        if let Ok(Event::Key(key)) = event::read()
            && key.kind == KeyEventKind::Press
        {
            stop |= matches!(
                keymap::command(key),
                keymap::Command::Back | keymap::Command::Quit
            );
        }
    }
    stop
}

/// Drops the keys pressed while the interface was blocked, so that they do not
/// act on the next screen.
fn discard_pending_events() -> io::Result<()> {
    while event::poll(Duration::ZERO)? {
        event::read()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;
    use crate::config::CONFIG_FILE;

    #[test]
    fn switching_language_saves_it() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(CONFIG_FILE);

        let (config, _) = switch_language(&Config::default(), &path, Language::French).unwrap();

        assert_eq!(config.language, Language::French);
        assert_eq!(Config::load_or_create(&path).unwrap(), config);
    }

    #[test]
    fn changed_protections_are_saved() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(CONFIG_FILE);
        let protected = BTreeSet::from(["alpha".to_owned()]);

        let config = with_protections(&Config::default(), &path, protected.clone()).unwrap();

        assert_eq!(config.protected, protected);
        assert_eq!(Config::load_or_create(&path).unwrap(), config);
    }

    #[test]
    fn unchanged_protections_are_not_rewritten() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(CONFIG_FILE);
        let config = Config::default();

        with_protections(&config, &path, config.protected.clone()).unwrap();

        assert!(!path.exists());
    }

    #[test]
    fn protections_are_unchanged_when_the_config_cannot_be_saved() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("missing").join(CONFIG_FILE);
        let protected = BTreeSet::from(["alpha".to_owned()]);

        assert!(with_protections(&Config::default(), &path, protected).is_err());
    }

    #[test]
    fn language_is_unchanged_when_the_config_cannot_be_saved() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("missing").join(CONFIG_FILE);

        assert!(switch_language(&Config::default(), &path, Language::French).is_err());
        assert!(!path.exists());
    }
}

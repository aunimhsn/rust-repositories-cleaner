//! Terminal user interface: event loop running the menus.

mod render;
mod state;

use std::io;
use std::path::Path;
use std::time::Duration;

use i18n_embed::fluent::FluentLanguageLoader;
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event, KeyEventKind};

use self::state::{Action, Effect, State};
use crate::config::Config;
use crate::github::GitHub;
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
    let mut app = App {
        state: State::new(config.language, None),
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
            terminal.draw(|frame| render::draw(frame, &self.state, &self.loader))?;

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
                    terminal.draw(|frame| render::draw(frame, &self.state, &self.loader))?;
                    self.check_delete_scope();
                    discard_pending_events()?;
                }
                Some(Effect::ChangeLanguage(language)) => self.change_language(language),
                // The lists of repositories come with the next roadmap steps.
                Some(Effect::OpenList(..)) => self.state.show_main_menu(),
            }
        }
    }

    fn check_delete_scope(&mut self) {
        match self.github.check_delete_scope() {
            Ok(()) => self.state.open_scope_menu(Action::Delete),
            Err(error) => self
                .state
                .show_message(github_error_message(&self.loader, &error)),
        }
    }

    /// Applies and saves a new language. Nothing changes if it fails.
    fn change_language(&mut self, language: Language) {
        match switch_language(&self.config, self.config_path, language) {
            Ok((config, loader)) => {
                self.config = config;
                self.loader = loader;
                self.state.set_language(language);
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
    fn language_is_unchanged_when_the_config_cannot_be_saved() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("missing").join(CONFIG_FILE);

        assert!(switch_language(&Config::default(), &path, Language::French).is_err());
        assert!(!path.exists());
    }
}

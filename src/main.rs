use std::path::Path;
use std::process::ExitCode;

use rust_repositories_cleaner::config::{CONFIG_FILE, Config};
use rust_repositories_cleaner::github::GitHub;
use rust_repositories_cleaner::i18n::{self, github_error_message};
use rust_repositories_cleaner::tui;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

/// Loads the configuration and checks the GitHub access before showing the
/// menus: these errors are printed and end the application.
fn run() -> Result<(), String> {
    let config_path = Path::new(CONFIG_FILE);
    let config = Config::load_or_create(config_path).map_err(|error| format!("Error: {error}"))?;
    let loader = i18n::load(config.language).map_err(|error| format!("Error: {error}"))?;
    let github = GitHub::connect().map_err(|error| github_error_message(&loader, &error))?;

    tui::run(config, config_path, loader, &github).map_err(|error| format!("Error: {error}"))
}

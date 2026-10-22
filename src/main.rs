use std::path::Path;
use std::process::ExitCode;

use rust_repositories_cleaner::config::{CONFIG_FILE, Config};
use rust_repositories_cleaner::github::GitHub;
use rust_repositories_cleaner::i18n::{self, fl, github_error_message};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

/// Temporary entry point until the menus exist: checks the GitHub access and
/// prints the number of repositories.
fn run() -> Result<(), String> {
    let config = Config::load_or_create(Path::new(CONFIG_FILE))
        .map_err(|error| format!("Error: {error}"))?;
    let loader = i18n::load(config.language).map_err(|error| format!("Error: {error}"))?;

    let github = GitHub::connect().map_err(|error| github_error_message(&loader, &error))?;
    println!("{}", fl!(loader, "loading-repositories"));
    let repositories = github
        .repositories()
        .map_err(|error| github_error_message(&loader, &error))?;

    let protected = repositories
        .iter()
        .filter(|repository| config.protected.contains(&repository.name))
        .count();
    println!(
        "{} · {}",
        fl!(loader, "repository-count", count = repositories.len()),
        fl!(loader, "protected-count", count = protected)
    );
    Ok(())
}

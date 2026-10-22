use std::path::Path;
use std::process::ExitCode;

use rust_repositories_cleaner::config::{CONFIG_FILE, Config};

fn main() -> ExitCode {
    match Config::load_or_create(Path::new(CONFIG_FILE)) {
        Ok(_config) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Error: {error}");
            ExitCode::FAILURE
        }
    }
}

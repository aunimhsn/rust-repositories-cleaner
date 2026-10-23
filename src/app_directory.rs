//! Directory of the local files (`config.toml` and `logs/deletions.log`),
//! inside the user configuration directory of the system:
//!
//! | System  | Application directory                                           |
//! |---------|-----------------------------------------------------------------|
//! | Linux   | `$XDG_CONFIG_HOME/rust-repositories-cleaner`, by default `~/.config/rust-repositories-cleaner` |
//! | macOS   | `~/Library/Application Support/rust-repositories-cleaner`       |
//! | Windows | `%APPDATA%\rust-repositories-cleaner`                           |

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::config::CONFIG_FILE;
use crate::deletion_log;

/// Name of the application directory, inside the user configuration directory.
const NAME: &str = "rust-repositories-cleaner";

/// Paths of the local files, inside the application directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalFiles {
    /// The configuration, `config.toml`.
    pub config: PathBuf,
    /// The log of the deletions, `logs/deletions.log`.
    pub log: PathBuf,
}

impl LocalFiles {
    fn in_directory(directory: &Path) -> Self {
        Self {
            config: directory.join(CONFIG_FILE),
            log: deletion_log::path_in(directory),
        }
    }
}

/// Error raised while finding or creating the application directory.
#[derive(Debug, Error)]
pub enum AppDirectoryError {
    #[error("cannot find the user configuration directory")]
    NotFound,
    #[error("cannot create {path}: {source}")]
    Create { path: PathBuf, source: io::Error },
}

/// Creates the application directory if needed and returns the paths of the
/// local files in it.
///
/// # Errors
///
/// Returns an error if the system has no user configuration directory, or if
/// the application directory cannot be created.
pub fn create() -> Result<LocalFiles, AppDirectoryError> {
    let base = dirs::config_dir().ok_or(AppDirectoryError::NotFound)?;
    create_in(&base)
}

/// Creates the application directory inside `base` if needed and returns the
/// paths of the local files in it.
fn create_in(base: &Path) -> Result<LocalFiles, AppDirectoryError> {
    let path = base.join(NAME);
    fs::create_dir_all(&path).map_err(|source| AppDirectoryError::Create {
        path: path.clone(),
        source,
    })?;
    Ok(LocalFiles::in_directory(&path))
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn the_directory_is_created_once() {
        let base = TempDir::new().unwrap();

        let files = create_in(base.path()).unwrap();

        let directory = base.path().join("rust-repositories-cleaner");
        assert!(directory.is_dir());
        assert_eq!(files.config, directory.join("config.toml"));
        assert_eq!(files.log, directory.join("logs").join("deletions.log"));
        assert_eq!(create_in(base.path()).unwrap(), files);
    }

    #[test]
    fn a_directory_that_cannot_be_created_is_an_error() {
        let base = TempDir::new().unwrap();
        let file = base.path().join("file");
        fs::write(&file, "").unwrap();

        // A file cannot contain a directory.
        assert!(matches!(
            create_in(&file),
            Err(AppDirectoryError::Create { .. })
        ));
    }
}

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

/// Name of the application directory, inside the user configuration directory.
const NAME: &str = "rust-repositories-cleaner";

/// Error raised while finding or creating the application directory.
#[derive(Debug, Error)]
pub enum AppDirectoryError {
    #[error("cannot find the user configuration directory")]
    NotFound,
    #[error("cannot create {path}: {source}")]
    Create { path: PathBuf, source: io::Error },
}

/// Returns the application directory, creating it if needed.
///
/// # Errors
///
/// Returns an error if the system has no user configuration directory, or if
/// the application directory cannot be created.
pub fn create() -> Result<PathBuf, AppDirectoryError> {
    let base = dirs::config_dir().ok_or(AppDirectoryError::NotFound)?;
    create_in(&base)
}

/// Returns the application directory inside `base`, creating it if needed.
fn create_in(base: &Path) -> Result<PathBuf, AppDirectoryError> {
    let path = base.join(NAME);
    fs::create_dir_all(&path).map_err(|source| AppDirectoryError::Create {
        path: path.clone(),
        source,
    })?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn the_directory_is_created_once() {
        let base = TempDir::new().unwrap();

        let path = create_in(base.path()).unwrap();

        assert_eq!(path, base.path().join("rust-repositories-cleaner"));
        assert!(path.is_dir());
        assert_eq!(create_in(base.path()).unwrap(), path);
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

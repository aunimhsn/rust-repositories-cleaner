//! User configuration stored in `config.toml`.

use std::collections::BTreeSet;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;
use thiserror::Error;

use crate::language::Language;

/// Path of the configuration file, relative to the application directory
/// (see [`crate::app_directory`]).
pub const CONFIG_FILE: &str = "config.toml";

/// Repository protected by default on first run: this project itself.
const DEFAULT_PROTECTED_REPOSITORY: &str = "rust-repositories-cleaner";

/// User configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    /// Language of the user interface.
    pub language: Language,
    /// Names of the repositories that can never be marked for deletion.
    pub protected: BTreeSet<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            language: Language::default(),
            protected: BTreeSet::from([DEFAULT_PROTECTED_REPOSITORY.to_owned()]),
        }
    }
}

/// Error raised while reading or writing the configuration file.
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("cannot read {path}: {source}")]
    Read { path: PathBuf, source: io::Error },
    #[error("invalid configuration in {path}: {source}")]
    Parse {
        path: PathBuf,
        source: toml::de::Error,
    },
    #[error("cannot serialize the configuration: {0}")]
    Serialize(#[from] toml::ser::Error),
    #[error("cannot write {path}: {source}")]
    Write { path: PathBuf, source: io::Error },
}

impl Config {
    /// Loads the configuration from `path`, or creates the file with default
    /// values if it does not exist yet.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read, is not valid, or cannot
    /// be created.
    pub fn load_or_create(path: &Path) -> Result<Self, ConfigError> {
        match fs::read_to_string(path) {
            Ok(content) => toml::from_str(&content).map_err(|source| ConfigError::Parse {
                path: path.to_path_buf(),
                source,
            }),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                let config = Self::default();
                config.save(path)?;
                Ok(config)
            }
            Err(source) => Err(ConfigError::Read {
                path: path.to_path_buf(),
                source,
            }),
        }
    }

    /// Saves the configuration to `path`, replacing its content atomically:
    /// the file holds either the old or the new content, never a truncated one.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration cannot be serialized or the file
    /// cannot be written.
    pub fn save(&self, path: &Path) -> Result<(), ConfigError> {
        let content = toml::to_string_pretty(self)?;
        let write_error = |source| ConfigError::Write {
            path: path.to_path_buf(),
            source,
        };

        // The temporary file must be in the same directory for the rename to be atomic.
        let directory = match path.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => parent,
            _ => Path::new("."),
        };
        let mut file = NamedTempFile::new_in(directory).map_err(write_error)?;
        file.write_all(content.as_bytes()).map_err(write_error)?;
        file.as_file().sync_all().map_err(write_error)?;
        file.persist(path)
            .map_err(|error| write_error(error.error))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn config_path(dir: &TempDir) -> std::path::PathBuf {
        dir.path().join(CONFIG_FILE)
    }

    #[test]
    fn default_config_is_english_and_protects_this_project() {
        let config = Config::default();

        assert_eq!(config.language, Language::English);
        assert!(config.protected.contains("rust-repositories-cleaner"));
    }

    #[test]
    fn creates_default_file_when_missing() {
        let dir = TempDir::new().unwrap();
        let path = config_path(&dir);

        let config = Config::load_or_create(&path).unwrap();

        assert_eq!(config, Config::default());
        assert!(path.exists());
    }

    #[test]
    fn saved_config_is_loaded_back() {
        let dir = TempDir::new().unwrap();
        let path = config_path(&dir);
        let config = Config {
            language: Language::French,
            protected: BTreeSet::from(["first".to_owned(), "second".to_owned()]),
        };

        config.save(&path).unwrap();

        assert_eq!(Config::load_or_create(&path).unwrap(), config);
    }

    #[test]
    fn missing_keys_use_default_values() {
        let config: Config = toml::from_str(r#"language = "fr""#).unwrap();

        assert_eq!(config.language, Language::French);
        assert_eq!(config.protected, Config::default().protected);
    }

    #[test]
    fn invalid_file_is_rejected() {
        let dir = TempDir::new().unwrap();
        let path = config_path(&dir);
        fs::write(&path, r#"language = "de""#).unwrap();

        let error = Config::load_or_create(&path).unwrap_err();

        assert!(matches!(error, ConfigError::Parse { .. }));
    }

    #[test]
    fn invalid_file_is_left_untouched() {
        let dir = TempDir::new().unwrap();
        let path = config_path(&dir);
        let content = r#"language = "de""#;
        fs::write(&path, content).unwrap();

        Config::load_or_create(&path).unwrap_err();

        assert_eq!(fs::read_to_string(&path).unwrap(), content);
    }

    #[test]
    fn unreadable_path_is_a_read_error() {
        let dir = TempDir::new().unwrap();

        // Reading a directory fails with an error other than NotFound.
        let error = Config::load_or_create(dir.path()).unwrap_err();

        assert!(matches!(error, ConfigError::Read { .. }));
    }

    #[test]
    fn missing_parent_directory_is_a_write_error() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("missing").join(CONFIG_FILE);

        let error = Config::load_or_create(&path).unwrap_err();

        assert!(matches!(error, ConfigError::Write { .. }));
    }

    #[test]
    fn save_leaves_no_temporary_file() {
        let dir = TempDir::new().unwrap();
        let path = config_path(&dir);

        Config::default().save(&path).unwrap();

        let entries: Vec<_> = fs::read_dir(dir.path()).unwrap().collect();
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn unknown_keys_are_rejected() {
        let result = toml::from_str::<Config>(r#"protect = ["typo"]"#);

        assert!(result.is_err());
    }

    #[test]
    fn example_file_is_valid() {
        let example = include_str!("../config.example.toml");

        let config: Config = toml::from_str(example).unwrap();

        assert_eq!(config, Config::default());
    }
}

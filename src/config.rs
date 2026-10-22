//! User configuration stored in `config.toml`.

use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::language::Language;

/// Path of the configuration file, relative to the current directory.
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
    Read { path: String, source: io::Error },
    #[error("invalid configuration in {path}: {source}")]
    Parse {
        path: String,
        source: toml::de::Error,
    },
    #[error("cannot serialize the configuration: {0}")]
    Serialize(#[from] toml::ser::Error),
    #[error("cannot write {path}: {source}")]
    Write { path: String, source: io::Error },
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
                path: path.display().to_string(),
                source,
            }),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                let config = Self::default();
                config.save(path)?;
                Ok(config)
            }
            Err(source) => Err(ConfigError::Read {
                path: path.display().to_string(),
                source,
            }),
        }
    }

    /// Saves the configuration to `path`, replacing its content.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration cannot be serialized or the file
    /// cannot be written.
    pub fn save(&self, path: &Path) -> Result<(), ConfigError> {
        let content = toml::to_string_pretty(self)?;
        fs::write(path, content).map_err(|source| ConfigError::Write {
            path: path.display().to_string(),
            source,
        })
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

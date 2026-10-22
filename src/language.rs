//! Interface languages.

use serde::{Deserialize, Serialize};

/// Language of the user interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Language {
    #[default]
    #[serde(rename = "en")]
    English,
    #[serde(rename = "fr")]
    French,
}

impl Language {
    /// All available languages, in the order shown in the language menu.
    ///
    /// A test checks that it matches the translation files in `i18n/`.
    pub const ALL: [Self; 2] = [Self::English, Self::French];

    /// Language code, also used as the name of its directory in `i18n/`.
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::French => "fr",
        }
    }

    /// Name of the language, written in the language itself.
    ///
    /// Not translated on purpose, as an exception to the rule that interface
    /// texts live in the Fluent files: a language is always shown under its own
    /// name, so users can find theirs whatever the current language.
    #[must_use]
    pub fn native_name(self) -> &'static str {
        match self {
            Self::English => "English",
            Self::French => "Français",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_match_the_configuration_values() {
        for language in Language::ALL {
            let serialized = toml::Value::try_from(language).unwrap();

            assert_eq!(serialized.as_str(), Some(language.code()));
        }
    }

    #[test]
    fn native_names_are_written_in_their_own_language() {
        assert_eq!(Language::English.native_name(), "English");
        assert_eq!(Language::French.native_name(), "Français");
    }
}

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

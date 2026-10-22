//! User interface translations.
//!
//! Messages are stored as [Fluent](https://projectfluent.org/) files in
//! `i18n/<language>/messages.ftl` and embedded in the binary. They are read
//! with the [`fl!`] macro, which checks at compile time that the message and
//! its arguments exist in the English reference file.
//!
//! `i18n-embed` does not report every problem in the other languages: a missing
//! file makes it panic and a syntax error is only logged. The tests of this
//! module are the real guard: every language has a file, every file is valid,
//! and every language has the same messages with the same variables.
//!
//! The log file does not use these messages: it is always written in English.

use i18n_embed::fluent::{FluentLanguageLoader, fluent_language_loader};
use i18n_embed::unic_langid::LanguageIdentifier;
use i18n_embed::{I18nEmbedError, LanguageLoader};
use rust_embed::RustEmbed;

use crate::language::Language;

pub use i18n_embed_fl::fl;

/// Translation files, embedded in the binary at compile time.
#[derive(RustEmbed)]
#[folder = "i18n/"]
struct Localizations;

/// Loads the translations of `language`, with English as the fallback for
/// any missing message.
///
/// # Errors
///
/// Returns an error if the English reference file is missing or is not valid
/// UTF-8. Fluent syntax errors are only logged by `i18n-embed`: the tests parse
/// every file to catch them.
pub fn load(language: Language) -> Result<FluentLanguageLoader, I18nEmbedError> {
    let loader = fluent_language_loader!();
    loader.load_languages(&Localizations, &[identifier(language)])?;
    // Unicode isolation marks around arguments are useless in a terminal.
    loader.set_use_isolating(false);
    Ok(loader)
}

/// Returns whether `key` confirms the deletion, in either case.
#[must_use]
pub fn confirms_deletion(loader: &FluentLanguageLoader, key: char) -> bool {
    let confirmation_key = fl!(loader, "confirmation-key");
    key.to_lowercase().eq(confirmation_key.chars())
}

/// Language identifier of `language`, used to find its Fluent file.
fn identifier(language: Language) -> LanguageIdentifier {
    language
        .code()
        .parse()
        .expect("language codes are valid identifiers, which a test checks")
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use fluent_syntax::ast::{
        Entry, Expression, InlineExpression, Pattern, PatternElement, Resource,
    };
    use fluent_syntax::parser;

    use super::*;

    fn english() -> FluentLanguageLoader {
        load(Language::English).unwrap()
    }

    fn french() -> FluentLanguageLoader {
        load(Language::French).unwrap()
    }

    /// Source of every translation file read by the loader, by language code.
    fn sources() -> BTreeMap<String, String> {
        Localizations::iter()
            .filter(|path| path.ends_with("/messages.ftl"))
            .map(|path| {
                let file = Localizations::get(&path).unwrap();
                let source = String::from_utf8(file.data.into_owned()).unwrap();
                let language = path.split('/').next().unwrap().to_owned();
                (language, source)
            })
            .collect()
    }

    /// Parses a Fluent file, panicking on syntax errors, which `i18n-embed`
    /// would only log.
    fn parse<'a>(language: &str, source: &'a str) -> Resource<&'a str> {
        parser::parse(source)
            .unwrap_or_else(|(_, errors)| panic!("syntax errors in {language}: {errors:?}"))
    }

    /// Variables (`$name`) used by each message and attribute of a Fluent
    /// resource, by identifier. Terms are prefixed with `-`.
    fn variables_by_identifier(resource: &Resource<&str>) -> BTreeMap<String, BTreeSet<String>> {
        let mut result = BTreeMap::new();
        for entry in &resource.body {
            let (name, value, attributes) = match entry {
                Entry::Message(message) => (
                    message.id.name.to_owned(),
                    message.value.as_ref(),
                    &message.attributes,
                ),
                Entry::Term(term) => (
                    format!("-{}", term.id.name),
                    Some(&term.value),
                    &term.attributes,
                ),
                _ => continue,
            };
            for attribute in attributes {
                let mut variables = BTreeSet::new();
                pattern_variables(&attribute.value, &mut variables);
                result.insert(format!("{name}.{}", attribute.id.name), variables);
            }
            let mut variables = BTreeSet::new();
            if let Some(pattern) = value {
                pattern_variables(pattern, &mut variables);
            }
            result.insert(name, variables);
        }
        result
    }

    fn pattern_variables(pattern: &Pattern<&str>, variables: &mut BTreeSet<String>) {
        for element in &pattern.elements {
            if let PatternElement::Placeable { expression } = element {
                expression_variables(expression, variables);
            }
        }
    }

    fn expression_variables(expression: &Expression<&str>, variables: &mut BTreeSet<String>) {
        match expression {
            Expression::Select { selector, variants } => {
                inline_variables(selector, variables);
                for variant in variants {
                    pattern_variables(&variant.value, variables);
                }
            }
            Expression::Inline(inline) => inline_variables(inline, variables),
        }
    }

    fn inline_variables(inline: &InlineExpression<&str>, variables: &mut BTreeSet<String>) {
        match inline {
            InlineExpression::VariableReference { id } => {
                variables.insert(id.name.to_owned());
            }
            InlineExpression::FunctionReference { arguments, .. } => {
                for argument in &arguments.positional {
                    inline_variables(argument, variables);
                }
                for argument in &arguments.named {
                    inline_variables(&argument.value, variables);
                }
            }
            InlineExpression::Placeable { expression } => {
                expression_variables(expression, variables);
            }
            // Literals and references to other messages or terms hold no
            // variable of the current message.
            _ => {}
        }
    }

    #[test]
    fn every_language_has_a_translation_file() {
        let files: BTreeSet<_> = sources().into_keys().collect();
        let languages: BTreeSet<_> = Language::ALL
            .iter()
            .map(|language| language.code().to_owned())
            .collect();

        assert_eq!(files, languages);
    }

    #[test]
    fn language_codes_are_valid_identifiers() {
        for language in Language::ALL {
            assert_eq!(identifier(language).to_string(), language.code());
        }
    }

    #[test]
    fn every_language_has_the_same_messages_and_variables() {
        let sources = sources();
        let reference_code = Language::default().code();
        let reference = variables_by_identifier(&parse(reference_code, &sources[reference_code]));

        for (language, source) in &sources {
            let messages = variables_by_identifier(&parse(language, source));

            let missing: Vec<_> = reference
                .keys()
                .filter(|id| !messages.contains_key(*id))
                .collect();
            let unexpected: Vec<_> = messages
                .keys()
                .filter(|id| !reference.contains_key(*id))
                .collect();
            assert!(
                missing.is_empty() && unexpected.is_empty(),
                "{language}: missing {missing:?}, not in English {unexpected:?}"
            );

            for (id, variables) in &messages {
                assert_eq!(
                    variables, &reference[id],
                    "{language}: `{id}` does not use the same variables as in English"
                );
            }
        }
    }

    #[test]
    fn english_uses_plural_for_zero() {
        let loader = english();

        assert_eq!(fl!(loader, "repository-count", count = 0), "0 repositories");
        assert_eq!(fl!(loader, "repository-count", count = 1), "1 repository");
        assert_eq!(
            fl!(loader, "repository-count", count = 330),
            "330 repositories"
        );
    }

    #[test]
    fn french_uses_singular_for_zero_and_one() {
        let loader = french();

        assert_eq!(fl!(loader, "repository-count", count = 0), "0 dépôt");
        assert_eq!(fl!(loader, "repository-count", count = 1), "1 dépôt");
        assert_eq!(fl!(loader, "repository-count", count = 330), "330 dépôts");
        assert_eq!(fl!(loader, "protected-count", count = 1), "1 protégé");
        assert_eq!(fl!(loader, "protected-count", count = 12), "12 protégés");
    }

    #[test]
    fn new_repository_count_includes_the_date() {
        assert_eq!(
            fl!(
                english(),
                "new-repository-count",
                count = 8,
                since = "2026-10-22"
            ),
            "8 new repositories since 2026-10-22"
        );
        assert_eq!(
            fl!(
                french(),
                "new-repository-count",
                count = 1,
                since = "2026-10-22"
            ),
            "1 nouveau dépôt depuis le 2026-10-22"
        );
    }

    #[test]
    fn confirmation_has_a_singular_form() {
        assert_eq!(
            fl!(english(), "confirm-deletion", count = 1),
            "Do you really want to delete this repository? [y/N]"
        );
        assert_eq!(
            fl!(english(), "confirm-deletion", count = 3),
            "Do you really want to delete these 3 repositories? [y/N]"
        );
        assert_eq!(
            fl!(french(), "confirm-deletion", count = 1),
            "Voulez-vous vraiment supprimer ce dépôt\u{a0}? [o/N]"
        );
        assert_eq!(
            fl!(french(), "confirm-deletion", count = 3),
            "Voulez-vous vraiment supprimer ces 3 dépôts\u{a0}? [o/N]"
        );
    }

    #[test]
    fn confirmation_accepts_both_cases_only() {
        assert!(confirms_deletion(&english(), 'y'));
        assert!(confirms_deletion(&english(), 'Y'));
        assert!(!confirms_deletion(&english(), 'o'));
        assert!(!confirms_deletion(&english(), 'n'));
        assert!(confirms_deletion(&french(), 'o'));
        assert!(confirms_deletion(&french(), 'O'));
        assert!(!confirms_deletion(&french(), 'y'));
    }

    #[test]
    fn deletion_summary_agrees_in_french() {
        assert_eq!(
            fl!(
                english(),
                "deletion-summary",
                deleted = 3,
                failed = 1,
                path = "logs/deletions.log"
            ),
            "3 deleted, 1 failed. See log: logs/deletions.log"
        );
        assert_eq!(
            fl!(
                french(),
                "deletion-summary",
                deleted = 1,
                failed = 0,
                path = "logs/deletions.log"
            ),
            "1 supprimé, 0 échec. Voir le journal\u{a0}: logs/deletions.log"
        );
        assert_eq!(
            fl!(
                french(),
                "deletion-summary",
                deleted = 3,
                failed = 2,
                path = "logs/deletions.log"
            ),
            "3 supprimés, 2 échecs. Voir le journal\u{a0}: logs/deletions.log"
        );
    }
}

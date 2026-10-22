//! Access to GitHub through the GitHub CLI (`gh`).
//!
//! `gh` handles the authentication token, pagination and HTTP errors, so this
//! module only runs `gh` commands and parses their output.

use std::io;
use std::process::{Command, Output};

use jiff::Timestamp;
use serde::Deserialize;
use thiserror::Error;

/// Token scope required to delete repositories.
const DELETE_SCOPE: &str = "delete_repo";

/// Exit code of `gh` when a command requires authentication and the user is
/// not logged in (see `gh help exit-codes`).
const NOT_LOGGED_IN_EXIT_CODE: i32 = 4;

/// Fields of each repository kept from the GitHub API response, one JSON object per line.
const REPOSITORY_FIELDS_FILTER: &str =
    ".[] | {name, private, fork, archived, size, created_at, pushed_at}";

/// Error raised while talking to GitHub.
#[derive(Debug, Error)]
pub enum GitHubError {
    #[error("GitHub CLI (gh) is not installed")]
    GhNotInstalled,
    #[error("not logged in to GitHub CLI")]
    NotLoggedIn,
    #[error("the GitHub CLI token is missing the {DELETE_SCOPE} scope")]
    MissingDeleteScope,
    #[error("GitHub request failed: {0}")]
    Request(String),
    #[error("unexpected response from GitHub CLI: {0}")]
    UnexpectedResponse(String),
}

/// A repository owned by the authenticated user.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Repository {
    pub name: String,
    pub private: bool,
    pub fork: bool,
    pub archived: bool,
    /// Size in kilobytes, as reported by GitHub: approximate, and it can lag
    /// behind the latest pushes.
    #[serde(rename = "size")]
    pub size_kb: u64,
    pub created_at: Timestamp,
    /// Date of the last push, empty for some repositories that never received one.
    pub pushed_at: Option<Timestamp>,
}

impl Repository {
    /// Date of the last update: the last push, or the creation if there was none.
    #[must_use]
    pub fn last_update(&self) -> Timestamp {
        self.pushed_at.unwrap_or(self.created_at)
    }
}

/// Connection to GitHub, checked to be able to list repositories.
#[derive(Debug)]
pub struct GitHub {
    login: String,
}

impl GitHub {
    /// Checks that `gh` is installed and logged in.
    ///
    /// The `delete_repo` scope is not required here: it is only checked by
    /// [`GitHub::check_delete_scope`] before deleting.
    ///
    /// # Errors
    ///
    /// Returns an error if one of these checks fails or GitHub cannot be reached.
    pub fn connect() -> Result<Self, GitHubError> {
        let output = user_response()?;
        let response = parse_include_output(&output)?;
        let user: User = serde_json::from_str(response.body)
            .map_err(|error| GitHubError::UnexpectedResponse(error.to_string()))?;
        Ok(Self { login: user.login })
    }

    /// Checks that the token has the `delete_repo` scope.
    ///
    /// # Errors
    ///
    /// Returns [`GitHubError::MissingDeleteScope`] if the scope is missing, or
    /// another error if GitHub cannot be reached.
    pub fn check_delete_scope(&self) -> Result<(), GitHubError> {
        let output = user_response()?;
        let response = parse_include_output(&output)?;
        if has_delete_scope(response.headers) {
            Ok(())
        } else {
            Err(GitHubError::MissingDeleteScope)
        }
    }

    /// Login of the authenticated user.
    #[must_use]
    pub fn login(&self) -> &str {
        &self.login
    }

    /// Lists the repositories owned by the authenticated user, sorted
    /// alphabetically by name, ignoring case.
    ///
    /// # Errors
    ///
    /// Returns an error if GitHub cannot be reached or its response cannot be parsed.
    pub fn repositories(&self) -> Result<Vec<Repository>, GitHubError> {
        let output = successful(gh(&[
            "api",
            "--paginate",
            "user/repos?affiliation=owner&per_page=100",
            "--jq",
            REPOSITORY_FIELDS_FILTER,
        ])?)?;
        let mut repositories = parse_repositories(&output)?;
        sort_by_name(&mut repositories);
        Ok(repositories)
    }
}

/// Body of the `user` API endpoint, reduced to the needed field.
#[derive(Deserialize)]
struct User {
    login: String,
}

/// Fetches the authenticated user with its HTTP headers, which list the token scopes.
fn user_response() -> Result<String, GitHubError> {
    let output = gh(&["api", "--include", "user"])?;
    if output.status.code() == Some(NOT_LOGGED_IN_EXIT_CODE) {
        return Err(GitHubError::NotLoggedIn);
    }
    successful(output)
}

/// Runs `gh` with `arguments` and returns its output, whatever its exit status.
///
/// Colors and terminal mode are disabled, so that the output can be parsed even
/// if the user's environment forces them.
fn gh(arguments: &[&str]) -> Result<Output, GitHubError> {
    Command::new("gh")
        .args(arguments)
        .env("NO_COLOR", "1")
        .env_remove("CLICOLOR_FORCE")
        .env_remove("GH_FORCE_TTY")
        .output()
        .map_err(|error| match error.kind() {
            io::ErrorKind::NotFound => GitHubError::GhNotInstalled,
            _ => GitHubError::Request(error.to_string()),
        })
}

/// Returns the standard output of a successful command, or its error message,
/// falling back to the exit status when `gh` printed nothing.
fn successful(output: Output) -> Result<String, GitHubError> {
    if output.status.success() {
        return String::from_utf8(output.stdout)
            .map_err(|error| GitHubError::UnexpectedResponse(error.to_string()));
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let message = match stderr.trim() {
        "" => output.status.to_string(),
        message => message.to_owned(),
    };
    Err(GitHubError::Request(message))
}

/// HTTP response printed by `gh api --include`: headers, a blank line, then the body.
#[derive(Debug, PartialEq, Eq)]
struct IncludeOutput<'a> {
    headers: &'a str,
    body: &'a str,
}

fn parse_include_output(output: &str) -> Result<IncludeOutput<'_>, GitHubError> {
    ["\r\n\r\n", "\n\n"]
        .iter()
        .find_map(|separator| output.split_once(separator))
        .map(|(headers, body)| IncludeOutput { headers, body })
        .ok_or_else(|| GitHubError::UnexpectedResponse("missing HTTP headers".to_owned()))
}

/// Returns whether the token has the `delete_repo` scope, according to the
/// `X-OAuth-Scopes` response header.
///
/// Tokens without this header (fine-grained or app tokens) have no scopes to
/// check: they are accepted, and a missing permission will make the deletion
/// fail with GitHub's reason.
fn has_delete_scope(headers: &str) -> bool {
    headers
        .lines()
        .filter_map(|line| line.split_once(':'))
        .find(|(name, _)| name.trim().eq_ignore_ascii_case("x-oauth-scopes"))
        .is_none_or(|(_, scopes)| scopes.split(',').any(|scope| scope.trim() == DELETE_SCOPE))
}

/// Parses one JSON repository per line, as produced by [`REPOSITORY_FIELDS_FILTER`].
fn parse_repositories(output: &str) -> Result<Vec<Repository>, GitHubError> {
    output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            serde_json::from_str(line)
                .map_err(|error| GitHubError::UnexpectedResponse(error.to_string()))
        })
        .collect()
}

fn sort_by_name(repositories: &mut [Repository]) {
    repositories.sort_by_cached_key(|repository| repository.name.to_lowercase());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repository(name: &str) -> Repository {
        Repository {
            name: name.to_owned(),
            private: false,
            fork: false,
            archived: false,
            size_kb: 0,
            created_at: "2021-01-01T00:00:00Z".parse().unwrap(),
            pushed_at: None,
        }
    }

    #[cfg(unix)]
    fn output(code: i32, stderr: &str) -> Output {
        use std::os::unix::process::ExitStatusExt;

        Output {
            status: std::process::ExitStatus::from_raw(code << 8),
            stdout: Vec::new(),
            stderr: stderr.as_bytes().to_vec(),
        }
    }

    #[cfg(unix)]
    #[test]
    fn failure_reports_the_gh_message() {
        let error = successful(output(1, "HTTP 401: Bad credentials\n")).unwrap_err();

        assert!(
            matches!(error, GitHubError::Request(message) if message == "HTTP 401: Bad credentials")
        );
    }

    #[cfg(unix)]
    #[test]
    fn failure_without_message_reports_the_exit_status() {
        let error = successful(output(1, "")).unwrap_err();

        assert!(matches!(error, GitHubError::Request(message) if message == "exit status: 1"));
    }

    #[test]
    fn include_output_is_split_into_headers_and_body() {
        let output = "HTTP/2.0 200 OK\nX-Oauth-Scopes: repo\n\n{\"login\":\"octocat\"}";

        let response = parse_include_output(output).unwrap();

        assert_eq!(response.headers, "HTTP/2.0 200 OK\nX-Oauth-Scopes: repo");
        assert_eq!(response.body, "{\"login\":\"octocat\"}");
    }

    #[test]
    fn include_output_accepts_crlf_line_endings() {
        let output = "HTTP/2.0 200 OK\r\nX-Oauth-Scopes: repo\r\n\r\n{}";

        assert_eq!(parse_include_output(output).unwrap().body, "{}");
    }

    #[test]
    fn include_output_accepts_the_gh_format() {
        // gh ends the status line with "\n" and the headers with "\r\n".
        let output = "HTTP/2.0 200 OK\nX-Oauth-Scopes: repo\r\n\r\n{}";

        assert_eq!(
            parse_include_output(output).unwrap(),
            IncludeOutput {
                headers: "HTTP/2.0 200 OK\nX-Oauth-Scopes: repo",
                body: "{}"
            }
        );
    }

    #[test]
    fn include_output_without_headers_is_rejected() {
        assert!(parse_include_output("{}").is_err());
    }

    #[test]
    fn delete_scope_is_found_in_the_scope_list() {
        assert!(has_delete_scope(
            "HTTP/2.0 200 OK\nX-Oauth-Scopes: gist, delete_repo, repo"
        ));
    }

    #[test]
    fn missing_delete_scope_is_detected() {
        assert!(!has_delete_scope(
            "HTTP/2.0 200 OK\nX-Oauth-Scopes: gist, read:org, repo"
        ));
        assert!(!has_delete_scope("HTTP/2.0 200 OK\nX-Oauth-Scopes: "));
    }

    #[test]
    fn scope_header_name_ignores_case() {
        // false proves the lowercase header was found: a missing header would be accepted.
        assert!(!has_delete_scope("HTTP/2.0 200 OK\nx-oauth-scopes: repo"));
    }

    #[test]
    fn tokens_without_scope_header_are_accepted() {
        assert!(has_delete_scope(
            "HTTP/2.0 200 OK\nContent-Type: application/json"
        ));
    }

    #[test]
    fn repositories_are_parsed_one_per_line() {
        let output = concat!(
            r#"{"name":"cleaner","private":false,"fork":false,"archived":false,"size":1234,"created_at":"2021-03-01T10:00:00Z","pushed_at":"2026-10-22T19:25:38Z"}"#,
            "\n",
            r#"{"name":"empty","private":true,"fork":true,"archived":true,"size":0,"created_at":"2022-05-01T08:00:00Z","pushed_at":null}"#,
            "\n",
        );

        let repositories = parse_repositories(output).unwrap();

        assert_eq!(repositories.len(), 2);
        assert_eq!(repositories[0].name, "cleaner");
        assert_eq!(repositories[0].size_kb, 1234);
        assert_eq!(
            repositories[0].last_update(),
            "2026-10-22T19:25:38Z".parse().unwrap()
        );
        assert!(repositories[1].private && repositories[1].fork && repositories[1].archived);
        assert_eq!(repositories[1].pushed_at, None);
    }

    #[test]
    fn empty_output_means_no_repository() {
        assert_eq!(parse_repositories("").unwrap(), Vec::new());
    }

    #[test]
    fn invalid_repository_is_rejected() {
        let error = parse_repositories(r#"{"name":"missing-fields"}"#).unwrap_err();

        assert!(matches!(error, GitHubError::UnexpectedResponse(_)));
    }

    #[test]
    fn last_update_falls_back_to_creation_date() {
        let repository = repository("never-pushed");

        assert_eq!(repository.last_update(), repository.created_at);
    }

    #[test]
    fn repositories_are_sorted_by_name_ignoring_case() {
        let mut repositories = vec![repository("zeta"), repository("Beta"), repository("alpha")];

        sort_by_name(&mut repositories);

        let names: Vec<_> = repositories
            .iter()
            .map(|repository| repository.name.as_str())
            .collect();
        assert_eq!(names, ["alpha", "Beta", "zeta"]);
    }
}

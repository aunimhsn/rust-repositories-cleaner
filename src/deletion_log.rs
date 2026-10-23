//! Log of the deletions, appended to `logs/deletions.log`.
//!
//! The log is always written in English, one line per event, so that it can
//! be read back reliably. Each deletion session starts with `STARTED` and ends
//! with `FINISHED`. Each repository gets a `DELETING` line before GitHub is
//! asked to delete it, then its result; repositories left untouched after a
//! stop are `SKIPPED`:
//!
//! ```text
//! 2026-10-23T10:15:00Z STARTED 3 repositories
//! 2026-10-23T10:15:00Z DELETING owner/first-repository
//! 2026-10-23T10:15:01Z DELETED owner/first-repository
//! 2026-10-23T10:15:01Z DELETING owner/second-repository
//! 2026-10-23T10:15:02Z FAILED owner/second-repository: HTTP 403: Must have admin rights
//! 2026-10-23T10:15:02Z SKIPPED owner/third-repository: stopped by the user
//! 2026-10-23T10:15:02Z FINISHED 1 deleted, 1 failed, 1 skipped
//! ```
//!
//! A session that never reached `FINISHED` (the application was killed, the
//! terminal closed…) is closed with an `INTERRUPTED` line at the next start,
//! naming the repository whose deletion may have been cut short.
//!
//! The date of the last deletion is read back from the last `DELETED` line.

use std::fs::{self, File, OpenOptions, TryLockError};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;

use jiff::{RoundMode, Timestamp, TimestampRound, Unit};

/// Path of the log file, relative to the current directory.
pub const LOG_FILE: &str = "logs/deletions.log";

/// Something to record in the log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event<'a> {
    /// A deletion session starts, for `count` repositories.
    Started { count: usize },
    /// GitHub is about to be asked to delete `repository` (`owner/name`).
    Deleting { repository: &'a str },
    /// `repository` was deleted.
    Deleted { repository: &'a str },
    /// The deletion of `repository` failed, with the reason given by GitHub.
    Failed {
        repository: &'a str,
        reason: &'a str,
    },
    /// `repository` was left untouched because the user stopped the deletions.
    Skipped { repository: &'a str },
    /// The deletion session ended normally.
    Finished {
        deleted: usize,
        failed: usize,
        skipped: usize,
    },
    /// The previous session ended without `FINISHED`, found at the next start,
    /// possibly while `pending` was being deleted.
    Interrupted { pending: Option<&'a str> },
}

/// A deletion session that started but never ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnfinishedSession {
    /// Repository whose deletion was requested but has no result: it may or
    /// may not have been deleted.
    pub pending: Option<String>,
}

/// Opens the log at `path` for appending, creating it and its directory if
/// needed. If the last line was cut short, a line break is added first so that
/// the next line starts cleanly.
///
/// # Errors
///
/// Returns an error if the directory or the file cannot be created or opened.
pub fn open(path: &Path) -> io::Result<File> {
    if let Some(directory) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(directory)?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .read(true)
        .append(true)
        .open(path)?;
    if file.metadata()?.len() > 0 {
        let mut last = [0];
        file.seek(SeekFrom::End(-1))?;
        file.read_exact(&mut last)?;
        if last != *b"\n" {
            file.write_all(b"\n")?;
        }
    }
    Ok(file)
}

/// Truncates `timestamp` to the second, the precision of the log. The date of
/// the last deletion kept in memory uses it too, so that it equals the one
/// read back from the log.
#[must_use]
pub fn truncate(timestamp: Timestamp) -> Timestamp {
    let whole_seconds = TimestampRound::new()
        .smallest(Unit::Second)
        .mode(RoundMode::Trunc);
    timestamp.round(whole_seconds).unwrap_or(timestamp)
}

/// Log line of `event` at `timestamp`, truncated to the second, without the
/// final line break.
#[must_use]
pub fn entry(timestamp: Timestamp, event: Event) -> String {
    let timestamp = truncate(timestamp);
    match event {
        Event::Started { count } => format!("{timestamp} STARTED {count} repositories"),
        Event::Deleting { repository } => format!("{timestamp} DELETING {repository}"),
        Event::Deleted { repository } => format!("{timestamp} DELETED {repository}"),
        Event::Failed { repository, reason } => {
            // The reason must stay on the line of its repository.
            let reason = reason.split_whitespace().collect::<Vec<_>>().join(" ");
            format!("{timestamp} FAILED {repository}: {reason}")
        }
        Event::Skipped { repository } => {
            format!("{timestamp} SKIPPED {repository}: stopped by the user")
        }
        Event::Finished {
            deleted,
            failed,
            skipped,
        } => format!("{timestamp} FINISHED {deleted} deleted, {failed} failed, {skipped} skipped"),
        Event::Interrupted { pending: None } => {
            format!("{timestamp} INTERRUPTED the previous session ended before FINISHED")
        }
        Event::Interrupted {
            pending: Some(repository),
        } => format!(
            "{timestamp} INTERRUPTED the previous session ended before FINISHED, {repository} may have been deleted"
        ),
    }
}

/// Returns the last session of `log` if it started but never ended.
#[must_use]
pub fn unfinished_session(log: &str) -> Option<UnfinishedSession> {
    let mut session = None;
    for line in log.lines() {
        let mut words = line.split_whitespace().skip(1);
        let kind = words.next();
        let repository = words.next().map(|word| word.trim_end_matches(':'));
        match (kind, &mut session) {
            (Some("STARTED"), _) => session = Some(UnfinishedSession { pending: None }),
            (Some("FINISHED" | "INTERRUPTED"), _) => session = None,
            (Some("DELETING"), Some(open)) => open.pending = repository.map(str::to_owned),
            (Some("DELETED" | "FAILED"), Some(open)) => open.pending = None,
            _ => {}
        }
    }
    session
}

/// Date of the last deletion of `log`: the timestamp of its last `DELETED`
/// line. A `DELETED` line whose timestamp cannot be read is ignored.
#[must_use]
pub fn last_deletion(log: &str) -> Option<Timestamp> {
    log.lines().rev().find_map(|line| {
        let mut words = line.split_whitespace();
        let timestamp = words.next()?;
        if words.next() == Some("DELETED") {
            timestamp.parse().ok()
        } else {
            None
        }
    })
}

/// Reads the log at `path`. A missing log reads as empty.
///
/// # Errors
///
/// Returns an error if the log exists but cannot be read.
pub fn read(path: &Path) -> io::Result<String> {
    match fs::read_to_string(path) {
        Ok(log) => Ok(log),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(String::new()),
        Err(error) => Err(error),
    }
}

/// Closes the last session of `log`, the content of the log at `path`, with an
/// `INTERRUPTED` line if it started but never ended, and returns it.
///
/// Nothing is done while another instance of the application holds the log,
/// since its session is still running.
///
/// # Errors
///
/// Returns an error if the log cannot be written.
pub fn close_unfinished_session(
    path: &Path,
    log: &str,
    now: Timestamp,
) -> io::Result<Option<UnfinishedSession>> {
    let Some(session) = unfinished_session(log) else {
        return Ok(None);
    };

    let mut file = open(path)?;
    match file.try_lock() {
        Ok(()) => {}
        Err(TryLockError::WouldBlock) => return Ok(None),
        Err(TryLockError::Error(error)) => return Err(error),
    }
    let event = Event::Interrupted {
        pending: session.pending.as_deref(),
    };
    file.write_all(format!("{}\n", entry(now, event)).as_bytes())?;
    Ok(Some(session))
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    fn timestamp() -> Timestamp {
        "2026-10-23T10:15:00.678Z".parse().unwrap()
    }

    #[test]
    fn entries_have_the_truncated_timestamp() {
        assert_eq!(
            entry(timestamp(), Event::Started { count: 3 }),
            "2026-10-23T10:15:00Z STARTED 3 repositories"
        );
        assert_eq!(
            entry(
                timestamp(),
                Event::Deleting {
                    repository: "owner/repository"
                }
            ),
            "2026-10-23T10:15:00Z DELETING owner/repository"
        );
        assert_eq!(
            entry(
                timestamp(),
                Event::Deleted {
                    repository: "owner/repository"
                }
            ),
            "2026-10-23T10:15:00Z DELETED owner/repository"
        );
        assert_eq!(
            entry(
                timestamp(),
                Event::Skipped {
                    repository: "owner/repository"
                }
            ),
            "2026-10-23T10:15:00Z SKIPPED owner/repository: stopped by the user"
        );
        assert_eq!(
            entry(
                timestamp(),
                Event::Finished {
                    deleted: 1,
                    failed: 2,
                    skipped: 3
                }
            ),
            "2026-10-23T10:15:00Z FINISHED 1 deleted, 2 failed, 3 skipped"
        );
    }

    #[test]
    fn failed_entry_keeps_the_reason_on_one_line() {
        let reason = "HTTP 403: Must have admin rights\n(https://docs.github.com)";

        assert_eq!(
            entry(
                timestamp(),
                Event::Failed {
                    repository: "owner/repository",
                    reason
                }
            ),
            "2026-10-23T10:15:00Z FAILED owner/repository: HTTP 403: Must have admin rights (https://docs.github.com)"
        );
    }

    #[test]
    fn interrupted_entry_names_the_pending_repository() {
        assert_eq!(
            entry(
                timestamp(),
                Event::Interrupted {
                    pending: Some("owner/repository")
                }
            ),
            "2026-10-23T10:15:00Z INTERRUPTED the previous session ended before FINISHED, owner/repository may have been deleted"
        );
    }

    #[test]
    fn a_session_cut_during_a_deletion_has_a_pending_repository() {
        let log = "\
2026-10-23T10:15:00Z STARTED 2 repositories
2026-10-23T10:15:00Z DELETING owner/alpha
2026-10-23T10:15:01Z DELETED owner/alpha
2026-10-23T10:15:01Z DELETING owner/beta
";

        assert_eq!(
            unfinished_session(log),
            Some(UnfinishedSession {
                pending: Some("owner/beta".to_owned())
            })
        );
    }

    #[test]
    fn a_session_cut_between_deletions_has_no_pending_repository() {
        let log = "\
2026-10-23T10:15:00Z STARTED 2 repositories
2026-10-23T10:15:00Z DELETING owner/alpha
2026-10-23T10:15:01Z FAILED owner/alpha: HTTP 500
";

        assert_eq!(
            unfinished_session(log),
            Some(UnfinishedSession { pending: None })
        );
    }

    #[test]
    fn finished_or_interrupted_sessions_are_closed() {
        let finished = "\
2026-10-23T10:15:00Z STARTED 1 repositories
2026-10-23T10:15:01Z FINISHED 0 deleted, 0 failed, 1 skipped
";
        let interrupted = "\
2026-10-23T10:15:00Z STARTED 1 repositories
2026-10-23T11:00:00Z INTERRUPTED the previous session ended before FINISHED
";

        assert_eq!(unfinished_session(finished), None);
        assert_eq!(unfinished_session(interrupted), None);
        assert_eq!(unfinished_session(""), None);
    }

    #[test]
    fn an_unfinished_session_is_closed_once() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("deletions.log");
        fs::write(
            &path,
            "2026-10-23T10:15:00Z STARTED 1 repositories\n2026-10-23T10:15:00Z DELETING owner/alpha\n",
        )
        .unwrap();

        let session = close_unfinished_session(&path, &read(&path).unwrap(), timestamp()).unwrap();

        assert_eq!(session.unwrap().pending.as_deref(), Some("owner/alpha"));
        assert_eq!(
            close_unfinished_session(&path, &read(&path).unwrap(), timestamp()).unwrap(),
            None
        );
    }

    #[test]
    fn a_cut_last_line_does_not_glue_the_next_one() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("deletions.log");
        fs::write(
            &path,
            "2026-10-23T10:15:00Z STARTED 1 repositories\n2026-10-23T10:1",
        )
        .unwrap();

        close_unfinished_session(&path, &read(&path).unwrap(), timestamp()).unwrap();

        let log = fs::read_to_string(&path).unwrap();
        assert!(log.ends_with("\n2026-10-23T10:1\n2026-10-23T10:15:00Z INTERRUPTED the previous session ended before FINISHED\n"));
        assert_eq!(unfinished_session(&log), None);
    }

    #[test]
    fn a_session_held_by_another_instance_is_not_closed() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("deletions.log");
        fs::write(&path, "2026-10-23T10:15:00Z STARTED 1 repositories\n").unwrap();
        let running = open(&path).unwrap();
        running.try_lock().unwrap();

        assert_eq!(
            close_unfinished_session(&path, &read(&path).unwrap(), timestamp()).unwrap(),
            None
        );
        assert!(unfinished_session(&fs::read_to_string(&path).unwrap()).is_some());
    }

    #[test]
    fn a_missing_log_has_no_session_to_close() {
        let dir = TempDir::new().unwrap();

        let path = dir.path().join(LOG_FILE);

        assert_eq!(read(&path).unwrap(), "");
        assert_eq!(
            close_unfinished_session(&path, "", timestamp()).unwrap(),
            None
        );
        assert!(!path.exists());
    }

    #[test]
    fn last_deletion_is_read_from_the_last_deleted_line() {
        let log = "\
2026-10-22T09:00:00Z STARTED 1 repositories
2026-10-22T09:00:00Z DELETING owner/alpha
2026-10-22T09:00:01Z DELETED owner/alpha
2026-10-22T09:00:01Z FINISHED 1 deleted, 0 failed, 0 skipped
2026-10-23T10:15:00Z STARTED 2 repositories
2026-10-23T10:15:00Z DELETING owner/beta
2026-10-23T10:15:01Z DELETED owner/beta
2026-10-23T10:15:01Z DELETING owner/gamma
2026-10-23T10:15:02Z FAILED owner/gamma: HTTP 403: Must have admin rights
2026-10-23T10:15:02Z FINISHED 1 deleted, 1 failed, 0 skipped
";

        assert_eq!(
            last_deletion(log),
            Some("2026-10-23T10:15:01Z".parse().unwrap())
        );
    }

    #[test]
    fn a_log_without_deleted_line_has_no_last_deletion() {
        let log = "\
2026-10-23T10:15:00Z STARTED 1 repositories
2026-10-23T10:15:00Z DELETING owner/alpha
2026-10-23T10:15:01Z FAILED owner/alpha: HTTP 403: Must have admin rights
2026-10-23T10:15:01Z FINISHED 0 deleted, 1 failed, 0 skipped
";

        assert_eq!(last_deletion(log), None);
        assert_eq!(last_deletion(""), None);
    }

    #[test]
    fn a_deleted_line_with_an_unreadable_date_is_ignored() {
        let log = "\
2026-10-22T09:00:01Z DELETED owner/alpha
2026-10-2 DELETED owner/beta
";

        assert_eq!(
            last_deletion(log),
            Some("2026-10-22T09:00:01Z".parse().unwrap())
        );
    }

    #[test]
    fn a_log_that_cannot_be_read_is_an_error() {
        let dir = TempDir::new().unwrap();

        // A directory exists but cannot be read as a file.
        assert!(read(dir.path()).is_err());
    }

    #[test]
    fn open_creates_the_directory_and_appends() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(LOG_FILE);

        writeln!(open(&path).unwrap(), "first").unwrap();
        writeln!(open(&path).unwrap(), "second").unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), "first\nsecond\n");
    }
}

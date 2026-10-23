//! Deletion of the confirmed repositories, one by one, with a log entry for each.

use std::io::{self, Write};

use jiff::Timestamp;

use crate::deletion_log::{self, Event};

/// What the deletions did, so far or in total.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Summary {
    pub deleted: usize,
    pub failed: usize,
    /// Repositories left untouched because the user stopped the deletions.
    pub not_processed: usize,
    /// Date of the last successful deletion, as written in the log.
    pub last_deleted_at: Option<Timestamp>,
}

/// Log line that could not be written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MissingLine {
    /// The start of the session: nothing was deleted.
    Start,
    /// The announcement of the deletion of a repository: it was not deleted.
    Deleting(String),
    /// The result of a repository (`owner/name`): it was processed, deleted
    /// or not, but the log does not say.
    Result(String),
    /// A repository left untouched after a stop.
    Skipped(String),
    /// The end of the session: every repository was processed and logged.
    End,
}

/// The log could not be written: the deletions stopped, since every deletion
/// must be recorded.
#[derive(Debug)]
pub struct LogError {
    /// What was done before stopping, with the repositories never attempted
    /// counted as not processed.
    pub summary: Summary,
    pub missing: MissingLine,
    pub source: io::Error,
}

/// Deletes the repositories `names` of `owner` with `delete`, which returns
/// GitHub's reason on failure, and records the session in `log`: a `STARTED`
/// line, a `DELETING` line before each deletion then its result, and a
/// `FINISHED` line.
///
/// - `progress` is called with the number of repositories processed and the
///   total, before the first deletion and after each one.
/// - `stop_requested` is called before each deletion: when it returns `true`,
///   the remaining repositories are left untouched and logged as `SKIPPED`.
/// - A failed deletion does not stop the next ones.
///
/// # Errors
///
/// Returns a [`LogError`] with what was done so far if the log cannot be
/// written: the deletions stop at once.
pub fn delete_all(
    names: &[String],
    owner: &str,
    mut delete: impl FnMut(&str) -> Result<(), String>,
    log: &mut impl Write,
    now: impl Fn() -> Timestamp,
    mut progress: impl FnMut(usize, usize),
    mut stop_requested: impl FnMut() -> bool,
) -> Result<Summary, LogError> {
    let mut summary = Summary::default();
    // Repositories processed so far, to count the others when stopping.
    let mut processed = 0;
    let mut record = |timestamp: Timestamp,
                      event: Event,
                      summary: Summary,
                      processed: usize,
                      missing: MissingLine| {
        // One write per line, so that a line is never split.
        let line = format!("{}\n", deletion_log::entry(timestamp, event));
        log.write_all(line.as_bytes())
            .and_then(|()| log.flush())
            .map_err(|source| LogError {
                summary: Summary {
                    not_processed: names.len() - processed,
                    ..summary
                },
                missing,
                source,
            })
    };

    let count = names.len();
    record(
        now(),
        Event::Started { count },
        summary,
        0,
        MissingLine::Start,
    )?;
    progress(0, count);

    for (index, name) in names.iter().enumerate() {
        let repository = format!("{owner}/{name}");
        if stop_requested() {
            summary.not_processed = count - index;
            for skipped in &names[index..] {
                let skipped = format!("{owner}/{skipped}");
                let event = Event::Skipped {
                    repository: &skipped,
                };
                let missing = MissingLine::Skipped(skipped.clone());
                record(now(), event, summary, count, missing)?;
            }
            break;
        }

        let deleting = Event::Deleting {
            repository: &repository,
        };
        let missing = MissingLine::Deleting(repository.clone());
        record(now(), deleting, summary, processed, missing)?;

        let result = delete(name);
        // The clock is read once, so that the date kept in memory is the one logged.
        let timestamp = now();
        processed += 1;
        let event = match &result {
            Ok(()) => {
                summary.deleted += 1;
                summary.last_deleted_at = Some(deletion_log::truncate(timestamp));
                Event::Deleted {
                    repository: &repository,
                }
            }
            Err(reason) => {
                summary.failed += 1;
                Event::Failed {
                    repository: &repository,
                    reason,
                }
            }
        };
        let missing = MissingLine::Result(repository.clone());
        record(timestamp, event, summary, processed, missing)?;
        progress(index + 1, count);
    }

    let finished = Event::Finished {
        deleted: summary.deleted,
        failed: summary.failed,
        skipped: summary.not_processed,
    };
    record(now(), finished, summary, count, MissingLine::End)?;
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| (*name).to_owned()).collect()
    }

    fn now() -> Timestamp {
        "2026-10-23T10:15:00.900Z".parse().unwrap()
    }

    fn never() -> bool {
        false
    }

    #[test]
    fn every_repository_is_deleted_and_logged() {
        let mut deleted = Vec::new();
        let mut log = Vec::new();

        let summary = delete_all(
            &names(&["alpha", "beta"]),
            "owner",
            |name| {
                deleted.push(name.to_owned());
                Ok(())
            },
            &mut log,
            now,
            |_, _| {},
            never,
        )
        .unwrap();

        assert_eq!((summary.deleted, summary.failed), (2, 0));
        assert_eq!(deleted, ["alpha", "beta"]);
        assert_eq!(
            String::from_utf8(log).unwrap(),
            "\
2026-10-23T10:15:00Z STARTED 2 repositories
2026-10-23T10:15:00Z DELETING owner/alpha
2026-10-23T10:15:00Z DELETED owner/alpha
2026-10-23T10:15:00Z DELETING owner/beta
2026-10-23T10:15:00Z DELETED owner/beta
2026-10-23T10:15:00Z FINISHED 2 deleted, 0 failed, 0 skipped
"
        );
    }

    #[test]
    fn last_deletion_date_is_the_one_written_in_the_log() {
        use std::cell::Cell;

        // A clock that advances one second at each reading.
        let seconds = Cell::new(0);
        let clock = || {
            seconds.set(seconds.get() + 1);
            Timestamp::from_second(seconds.get()).unwrap()
        };
        let mut log = Vec::new();

        let summary = delete_all(
            &names(&["alpha"]),
            "owner",
            |_| Ok(()),
            &mut log,
            clock,
            |_, _| {},
            never,
        )
        .unwrap();

        let log = String::from_utf8(log).unwrap();
        let deleted = log.lines().find(|line| line.contains(" DELETED ")).unwrap();
        let logged: Timestamp = deleted.split(' ').next().unwrap().parse().unwrap();
        assert_eq!(summary.last_deleted_at, Some(logged));
    }

    #[test]
    fn a_failure_is_logged_and_does_not_stop_the_next_deletions() {
        let mut log = Vec::new();

        let summary = delete_all(
            &names(&["alpha", "beta"]),
            "owner",
            |name| match name {
                "alpha" => Err("HTTP 403: Must have admin rights".to_owned()),
                _ => Ok(()),
            },
            &mut log,
            now,
            |_, _| {},
            never,
        )
        .unwrap();

        assert_eq!((summary.deleted, summary.failed), (1, 1));
        assert!(
            String::from_utf8(log)
                .unwrap()
                .contains("\n2026-10-23T10:15:00Z FAILED owner/alpha: HTTP 403")
        );
    }

    #[test]
    fn progress_is_reported_before_and_after_each_deletion() {
        let mut steps = Vec::new();

        delete_all(
            &names(&["alpha", "beta"]),
            "owner",
            |_| Ok(()),
            &mut Vec::new(),
            now,
            |done, total| steps.push((done, total)),
            never,
        )
        .unwrap();

        assert_eq!(steps, [(0, 2), (1, 2), (2, 2)]);
    }

    #[test]
    fn a_stop_leaves_the_remaining_repositories_untouched_and_logged() {
        let mut deleted = Vec::new();
        let mut log = Vec::new();
        let mut checks = 0;

        let summary = delete_all(
            &names(&["alpha", "beta", "gamma"]),
            "owner",
            |name| {
                deleted.push(name.to_owned());
                Ok(())
            },
            &mut log,
            now,
            |_, _| {},
            || {
                checks += 1;
                checks > 1
            },
        )
        .unwrap();

        assert_eq!(deleted, ["alpha"]);
        assert_eq!((summary.deleted, summary.not_processed), (1, 2));
        assert!(String::from_utf8(log).unwrap().ends_with(
            "\
2026-10-23T10:15:00Z SKIPPED owner/beta: stopped by the user
2026-10-23T10:15:00Z SKIPPED owner/gamma: stopped by the user
2026-10-23T10:15:00Z FINISHED 1 deleted, 0 failed, 2 skipped
"
        ));
    }

    #[test]
    fn a_log_that_cannot_be_written_prevents_any_deletion() {
        let mut deleted = 0;

        let error = delete_all(
            &names(&["alpha", "beta"]),
            "owner",
            |_| {
                deleted += 1;
                Ok(())
            },
            &mut FailingLog { lines: 0 },
            now,
            |_, _| {},
            never,
        )
        .unwrap_err();

        assert_eq!(deleted, 0);
        assert_eq!(error.missing, MissingLine::Start);
        assert_eq!(error.summary.not_processed, 2);
    }

    /// A log that accepts `lines` lines, then refuses every write.
    struct FailingLog {
        lines: usize,
    }

    impl Write for FailingLog {
        fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
            if self.lines == 0 {
                return Err(io::Error::other("disk full"));
            }
            self.lines -= 1;
            Ok(buffer.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    fn fail_after(lines: usize, deleted: &mut usize) -> LogError {
        delete_all(
            &names(&["alpha", "beta"]),
            "owner",
            |_| {
                *deleted += 1;
                Ok(())
            },
            &mut FailingLog { lines },
            now,
            |_, _| {},
            never,
        )
        .unwrap_err()
    }

    #[test]
    fn a_missing_deleting_line_prevents_that_deletion() {
        let mut deleted = 0;

        let error = fail_after(1, &mut deleted);

        assert_eq!(deleted, 0);
        assert_eq!(
            error.missing,
            MissingLine::Deleting("owner/alpha".to_owned())
        );
    }

    #[test]
    fn a_missing_result_names_the_processed_repository() {
        let mut deleted = 0;

        let error = fail_after(2, &mut deleted);

        assert_eq!(deleted, 1);
        assert_eq!(error.missing, MissingLine::Result("owner/alpha".to_owned()));
        assert_eq!((error.summary.deleted, error.summary.not_processed), (1, 1));
        assert!(error.summary.last_deleted_at.is_some());
    }

    #[test]
    fn a_missing_end_comes_after_every_deletion() {
        let mut deleted = 0;

        let error = fail_after(5, &mut deleted);

        assert_eq!(deleted, 2);
        assert_eq!(error.missing, MissingLine::End);
        assert_eq!(error.summary.not_processed, 0);
    }
}

//! `standby timeout`: run a command and stop it if it outlives a time limit.

#[cfg(unix)]
mod unix;
#[cfg(unix)]
use unix as sys;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows as sys;

use crate::cli::TimeoutArgs;
use crate::exit;
use standby::time::Duration;
use std::io;
use std::time::Duration as StdDuration;

pub fn run(args: TimeoutArgs) -> i32 {
    match sys::run(&args) {
        Ok(status) => status,
        Err(Failure::Spawn(err)) => {
            let command = args.command[0].to_string_lossy();
            report!("standby: failed to run command '{command}': {err}");
            if err.kind() == io::ErrorKind::NotFound {
                exit::NOT_FOUND
            } else {
                exit::CANNOT_INVOKE
            }
        }
        Err(Failure::Supervise(err)) => {
            report!("standby: {err}");
            exit::FAILURE
        }
    }
}

/// Why `timeout` could not report the command's status.
enum Failure {
    /// The command could not be started.
    Spawn(io::Error),
    /// The command started, but watching over it failed.
    Supervise(io::Error),
}

impl From<io::Error> for Failure {
    fn from(err: io::Error) -> Self {
        Self::Supervise(err)
    }
}

/// How the command ended.
struct Outcome {
    /// The command's own status: its exit code, or 128 plus the signal that
    /// ended it.
    status: i32,
    /// The time limit was reached.
    timed_out: bool,
    /// The command was killed outright rather than asked to stop.
    killed: bool,
}

impl Outcome {
    /// The status standby exits with.
    fn exit_status(&self, preserve_status: bool) -> i32 {
        if !self.timed_out || preserve_status {
            self.status
        } else if self.killed {
            exit::KILLED
        } else {
            exit::TIMED_OUT
        }
    }
}

/// The length of a time limit, or `None` when there is none: zero and
/// infinity both switch a limit off, as in GNU `timeout`.
fn limit(duration: Duration) -> Option<StdDuration> {
    duration.to_std().filter(|limit| !limit.is_zero())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn outcome(status: i32, timed_out: bool, killed: bool) -> Outcome {
        Outcome {
            status,
            timed_out,
            killed,
        }
    }

    #[test]
    fn a_command_that_finishes_in_time_keeps_its_status() {
        assert_eq!(outcome(0, false, false).exit_status(false), 0);
        assert_eq!(outcome(3, false, false).exit_status(false), 3);
        assert_eq!(outcome(143, false, false).exit_status(true), 143);
    }

    #[test]
    fn a_timeout_reports_124() {
        assert_eq!(outcome(143, true, false).exit_status(false), 124);
        // Even when the command caught the signal and exited cleanly.
        assert_eq!(outcome(0, true, false).exit_status(false), 124);
    }

    #[test]
    fn a_timeout_that_needed_kill_reports_137() {
        assert_eq!(outcome(137, true, true).exit_status(false), 137);
    }

    #[test]
    fn preserve_status_reports_the_command_status_after_a_timeout() {
        assert_eq!(outcome(143, true, false).exit_status(true), 143);
        assert_eq!(outcome(0, true, false).exit_status(true), 0);
        assert_eq!(outcome(7, true, true).exit_status(true), 7);
    }

    #[test]
    fn zero_and_infinity_mean_no_limit() {
        assert_eq!(limit(Duration::ZERO), None);
        assert_eq!(limit(Duration::Infinite), None);
        assert_eq!(
            limit(Duration::from_secs(5)),
            Some(StdDuration::from_secs(5))
        );
    }
}

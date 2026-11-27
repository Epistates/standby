//! Command-line definitions.

use clap::{Args, Parser, Subcommand};
use clap_complete::Shell;
use standby::StandbyError;
use standby::signals::Signal;
use standby::time::Duration;
use std::ffi::OsString;

/// The duration format, shared by the help of every subcommand that takes one.
macro_rules! duration_help {
    () => {
        "\
Durations are seconds by default and take the units ns, us, ms, s, m, h and d.
They may be fractional and may be combined: 90, 2.5s, 1m30s, 500ms, infinity."
    };
}

const SLEEP_HELP: &str = duration_help!();

const TIMEOUT_HELP: &str = concat!(
    duration_help!(),
    "

At the time limit the signal goes to the command and to every process it
started. A signal sent to standby is passed on to the command.

Exit status:
  124  the time limit was reached (unless --preserve-status)
  125  standby itself failed
  126  the command was found but could not be run
  127  the command was not found
  137  the command was sent KILL
  otherwise the status of the command, or 128+N if signal N ended it"
);

const WAIT_HELP: &str = concat!(
    duration_help!(),
    "

Exit status:
  0    every process has exited
  124  the timeout was reached first
  1    a process could not be watched"
);

#[derive(Parser)]
#[command(
    name = "standby",
    version,
    about = "Sleep, run commands under a time limit, and wait for processes to exit"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Pause for a length of time
    Sleep(SleepArgs),

    /// Run a command and stop it if it outlives a time limit
    Timeout(TimeoutArgs),

    /// Wait until processes have exited
    Wait(WaitArgs),

    /// Print a shell completion script
    Completions(CompletionsArgs),
}

#[derive(Args)]
#[command(after_help = SLEEP_HELP)]
pub struct SleepArgs {
    /// How long to pause; several durations add up
    #[arg(required = true, value_name = "DURATION", value_parser = duration)]
    pub durations: Vec<Duration>,
}

#[derive(Args)]
// A repeated option overrides the earlier one, as in GNU `timeout`, so that
// an alias can set defaults.
#[command(after_help = TIMEOUT_HELP, args_override_self = true)]
pub struct TimeoutArgs {
    /// Signal to send at the time limit, by name or number
    #[arg(short, long, value_name = "SIGNAL", default_value = "TERM", value_parser = signal)]
    pub signal: Signal,

    /// Send KILL to whatever the command started that is still running this
    /// long after the first signal, and wait until none of it is left
    #[arg(short, long, value_name = "DURATION", value_parser = duration)]
    pub kill_after: Option<Duration>,

    /// Exit with the command's own status even when the time limit is reached
    #[arg(short, long)]
    pub preserve_status: bool,

    /// Leave the command in standby's process group so that it can read from
    /// the terminal; its children are then not stopped at the time limit
    #[arg(short, long)]
    pub foreground: bool,

    /// Report on stderr each signal that is sent
    #[arg(short, long)]
    pub verbose: bool,

    /// Time limit; 0 or infinity means no limit
    #[arg(value_name = "DURATION", value_parser = duration)]
    pub duration: Duration,

    /// Command to run, followed by its arguments
    #[arg(
        required = true,
        trailing_var_arg = true,
        allow_hyphen_values = true,
        value_name = "COMMAND"
    )]
    pub command: Vec<OsString>,
}

#[derive(Args)]
#[command(after_help = WAIT_HELP)]
pub struct WaitArgs {
    /// Give up after this long; 0 checks once without waiting
    #[arg(short, long, value_name = "DURATION", value_parser = duration)]
    pub timeout: Option<Duration>,

    /// IDs of the processes to wait for; one that does not exist counts as
    /// already exited
    #[arg(
        required = true,
        value_name = "PID",
        value_parser = clap::value_parser!(u32).range(1..=i32::MAX as i64)
    )]
    pub pids: Vec<u32>,
}

#[derive(Args)]
pub struct CompletionsArgs {
    /// Shell to generate the script for
    pub shell: Shell,
}

// clap already names the argument and quotes the value, so these pass on only
// what is wrong with it.

fn duration(value: &str) -> Result<Duration, String> {
    value.parse().map_err(|err| match err {
        StandbyError::InvalidDuration { reason, .. } => reason,
        other => other.to_string(),
    })
}

fn signal(value: &str) -> Result<Signal, String> {
    value
        .parse()
        .map_err(|_| "no such signal on this platform".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn definition_is_consistent() {
        Cli::command().debug_assert();
    }

    fn timeout(args: &[&str]) -> TimeoutArgs {
        let argv = ["standby", "timeout"].iter().chain(args);
        match Cli::try_parse_from(argv).map(|cli| cli.command) {
            Ok(Command::Timeout(args)) => args,
            Ok(_) => panic!("parsed as another subcommand"),
            Err(err) => panic!("{err}"),
        }
    }

    #[test]
    fn options_after_the_command_belong_to_the_command() {
        let args = timeout(&["5", "grep", "-s", "-k", "--verbose", "--help", "x"]);
        assert_eq!(
            args.command,
            ["grep", "-s", "-k", "--verbose", "--help", "x"]
        );
        assert_eq!(args.signal, Signal::TERM);
        assert_eq!(args.kill_after, None);
        assert!(!args.verbose);
    }

    #[test]
    fn options_before_the_duration_belong_to_standby() {
        let args = timeout(&[
            "-s", "KILL", "-k", "2s", "-p", "-f", "-v", "1m", "make", "-j8",
        ]);
        assert_eq!(args.signal, Signal::KILL);
        assert_eq!(args.kill_after, Some(Duration::from_secs(2)));
        assert!(args.preserve_status && args.foreground && args.verbose);
        assert_eq!(args.duration, Duration::from_secs(60));
        assert_eq!(args.command, ["make", "-j8"]);
    }

    #[test]
    fn a_repeated_option_overrides_the_earlier_one() {
        let args = timeout(&[
            "-k", "5", "-s", "INT", "-v", "-k", "9", "-s", "KILL", "-v", "1", "x",
        ]);
        assert_eq!(args.kill_after, Some(Duration::from_secs(9)));
        assert_eq!(args.signal, Signal::KILL);
        assert!(args.verbose);
    }

    #[test]
    fn a_double_dash_inside_the_command_is_kept() {
        let args = timeout(&["5", "git", "log", "--", "file"]);
        assert_eq!(args.command, ["git", "log", "--", "file"]);
    }

    #[test]
    fn a_command_is_required() {
        assert!(Cli::try_parse_from(["standby", "timeout", "5"]).is_err());
    }

    #[test]
    fn sleep_takes_several_durations() {
        let cli = Cli::try_parse_from(["standby", "sleep", "1m", "30s"]).unwrap();
        let Command::Sleep(args) = cli.command else {
            panic!("parsed as another subcommand");
        };
        assert_eq!(
            args.durations,
            [Duration::from_secs(60), Duration::from_secs(30)]
        );
    }

    #[test]
    fn wait_rejects_ids_that_cannot_be_a_process() {
        for pid in ["0", "-1", "2147483648", "abc"] {
            assert!(
                Cli::try_parse_from(["standby", "wait", pid]).is_err(),
                "{pid}"
            );
        }
        assert!(Cli::try_parse_from(["standby", "wait"]).is_err());
        assert!(Cli::try_parse_from(["standby", "wait", "1", "2147483647"]).is_ok());
    }
}

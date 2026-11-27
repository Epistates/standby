//! The `standby` command-line tool: `sleep`, `timeout` and `wait`.

#[cfg(not(any(unix, windows)))]
compile_error!("standby runs on Unix and Windows only");

mod cli;
mod sleep;
#[cfg(unix)]
mod terminal;
mod timeout;
mod wait;
#[cfg(windows)]
mod windows;

use clap::{CommandFactory, Parser};
use cli::{Cli, Command};
use std::io::{self, Write};

/// Exit statuses that standby itself produces.
///
/// `timeout` follows GNU `timeout`: anything else is the command's own status,
/// or 128 plus the number of the signal that ended it.
mod exit {
    /// The time limit was reached.
    pub const TIMED_OUT: i32 = 124;
    /// `timeout` itself failed.
    pub const FAILURE: i32 = 125;
    /// The command was found but could not be run.
    pub const CANNOT_INVOKE: i32 = 126;
    /// The command was not found.
    pub const NOT_FOUND: i32 = 127;
    /// The command was killed outright.
    pub const KILLED: i32 = 128 + 9;
}

fn main() {
    let cli = Cli::try_parse().unwrap_or_else(|err| {
        // Printing fails only when the stream is closed; nothing to add then.
        let _ = err.print();
        std::process::exit(if err.use_stderr() {
            usage_error_status()
        } else {
            0
        });
    });

    let status = match cli.command {
        Command::Sleep(args) => sleep::run(args),
        Command::Timeout(args) => timeout::run(args),
        Command::Wait(args) => wait::run(args),
        Command::Completions(args) => completions(args),
    };
    std::process::exit(status);
}

/// The exit status for a malformed command line.
///
/// `timeout` reserves 125 for its own failures, so that a usage error is not
/// mistaken for the command's status. `sleep` exits 1 as GNU `sleep` does.
/// Everything else uses clap's conventional 2.
fn usage_error_status() -> i32 {
    match std::env::args_os()
        .nth(1)
        .as_deref()
        .and_then(|arg| arg.to_str())
    {
        Some("timeout") => exit::FAILURE,
        Some("sleep") => 1,
        _ => 2,
    }
}

fn completions(args: cli::CompletionsArgs) -> i32 {
    // Rendered to memory first: the generator panics on a write error, and a
    // closed pipe (`standby completions zsh | head`) is not worth a panic.
    let mut script = Vec::new();
    clap_complete::generate(args.shell, &mut Cli::command(), "standby", &mut script);
    match io::stdout().write_all(&script) {
        Ok(()) => 0,
        Err(err) if err.kind() == io::ErrorKind::BrokenPipe => 0,
        Err(err) => {
            eprintln!("standby: {err}");
            1
        }
    }
}

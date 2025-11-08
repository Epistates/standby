//! Timeout subcommand implementation.
//!
//! Runs a command with a time limit and signal handling.

use crate::errors::{Result, StandbyError};
use crate::signals::{Signal, SignalHandler};
use crate::time::parse_duration;
use clap::Parser;
use std::process::Command;
use std::time::Duration;
use std::thread;

/// Arguments for the timeout subcommand.
#[derive(Parser)]
pub struct TimeoutArgs {
    /// Duration before timeout (e.g., "5", "5s", "1m30s")
    pub duration: String,

    /// Command to run
    pub command: String,

    /// Command arguments
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub args: Vec<String>,

    /// Signal to send on timeout (default: SIGTERM)
    #[arg(short = 's', long, default_value = "TERM")]
    pub signal: String,

    /// Time to wait before sending SIGKILL after initial signal
    #[arg(short = 'k', long)]
    pub kill_after: Option<String>,

    /// Preserve status of command
    #[arg(long)]
    pub preserve_status: bool,
}

/// Execute the timeout command.
pub fn execute(args: TimeoutArgs) -> Result<()> {
    let timeout_duration = parse_duration(&args.duration)?;
    let timeout_std = timeout_duration.to_std_duration();

    let mut child = Command::new(&args.command)
        .args(&args.args)
        .spawn()
        .map_err(|e| {
            StandbyError::ProcessError(format!(
                "Failed to spawn command '{}': {}",
                args.command, e
            ))
        })?;

    let child_id = child.id();
    let signal = parse_signal(&args.signal)?;
    let kill_after = if let Some(k) = args.kill_after {
        Some(parse_duration(&k)?.to_std_duration())
    } else {
        None
    };

    // Wait for the process with timeout
    let start = std::time::Instant::now();

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                // Process completed successfully
                if !args.preserve_status {
                    std::process::exit(status.code().unwrap_or(0));
                }
                return Ok(());
            }
            Ok(None) => {
                // Process still running
                if start.elapsed() >= timeout_std {
                    // Timeout reached - send signal
                    eprintln!(
                        "timeout: sending signal {} to pid {}",
                        args.signal, child_id
                    );

                    SignalHandler::send_signal(&child, signal)
                        .map_err(|_| {
                            // If we can't send signal via handler, process still running
                        })
                        .ok();

                    // If kill_after specified, wait and send KILL signal
                    if let Some(kill_duration) = kill_after {
                        let kill_start = std::time::Instant::now();

                        loop {
                            match child.try_wait() {
                                Ok(Some(status)) => {
                                    if !args.preserve_status {
                                        std::process::exit(status.code().unwrap_or(1));
                                    }
                                    return Ok(());
                                }
                                Ok(None) => {
                                    if kill_start.elapsed() >= kill_duration {
                                        eprintln!("timeout: sending SIGKILL to pid {}", child_id);
                                        SignalHandler::send_signal(&child, Signal::Kill).ok();
                                        // Give a moment for SIGKILL to take effect
                                        thread::sleep(Duration::from_millis(100));
                                    }

                                    thread::sleep(Duration::from_millis(10));
                                }
                                Err(e) => {
                                    return Err(StandbyError::ProcessError(format!(
                                        "Error waiting for process: {}",
                                        e
                                    )))
                                }
                            }
                        }
                    } else {
                        // Just wait for process to die
                        match child.wait() {
                            Ok(status) => {
                                if !args.preserve_status {
                                    std::process::exit(status.code().unwrap_or(1));
                                }
                                return Ok(());
                            }
                            Err(e) => {
                                return Err(StandbyError::ProcessError(format!(
                                    "Error waiting for process: {}",
                                    e
                                )))
                            }
                        }
                    }
                }

                thread::sleep(Duration::from_millis(10));
            }
            Err(e) => {
                return Err(StandbyError::ProcessError(format!(
                    "Failed to wait for process: {}",
                    e
                )))
            }
        }
    }
}

/// Parse a signal name or number into a Signal enum.
fn parse_signal(signal_str: &str) -> Result<Signal> {
    match signal_str.to_uppercase().as_str() {
        "TERM" | "15" => Ok(Signal::Term),
        "KILL" | "9" => Ok(Signal::Kill),
        "INT" | "2" => Ok(Signal::Int),
        _ => Err(StandbyError::InvalidArgument(format!(
            "Unknown signal: {}",
            signal_str
        ))),
    }
}

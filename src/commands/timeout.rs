//! Timeout subcommand implementation.
//!
//! Runs a command with a time limit and signal handling.

use crate::debug::debug;
use crate::errors::{Result, StandbyError};
use crate::signals::{Signal, SignalHandler};
use crate::terminal::TerminalGuard;
use crate::time::parse_duration;
use clap::Parser;
use std::process::Command;
use std::thread;
use std::time::Duration;

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

    /// Run command in foreground (same process group)
    #[arg(long)]
    pub foreground: bool,

    /// Enable verbose output for debugging timeout behavior
    #[arg(short = 'v', long)]
    pub verbose: bool,
}

/// Execute the timeout command.
pub fn execute(args: TimeoutArgs) -> Result<()> {
    // Initialize verbose mode
    crate::debug::init_verbose(args.verbose);

    // RAII guard ensures terminal restoration on all code paths
    let _terminal = TerminalGuard::new();

    debug!("Timeout command started");
    debug!("Duration: {}", args.duration);
    debug!("Command: {} {:?}", args.command, args.args);

    let timeout_duration = parse_duration(&args.duration)?;
    let timeout_std = timeout_duration.to_std_duration();

    debug!("Parsed timeout: {:.3}s", timeout_std.as_secs_f64());

    // On Unix, ignore SIGTTIN and SIGTTOU to allow background child to access terminal
    // This matches GNU timeout behavior and prevents child from being suspended
    // when it tries to read/write the terminal while in a background process group
    #[cfg(unix)]
    if !args.foreground {
        setup_tty_signals()?;
    }

    // Build command with optional foreground mode
    let mut command = Command::new(&args.command);
    command.args(&args.args);

    // On Unix, set up process group based on foreground flag
    // Using process_group(0) is safer and faster than pre_exec with setpgid:
    // - No unsafe code required
    // - Uses fast posix_spawn path instead of fork+exec
    // - No async-signal-safety concerns
    // - Stabilized in Rust 1.64.0, recommended by RFC 3228
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        if !args.foreground {
            // Run in separate process group (default behavior, like GNU timeout)
            // process_group(0) sets PGID to the child's own PID
            // With SIGTTIN/SIGTTOU ignored above, child can still access terminal
            command.process_group(0);
        }
        // If foreground is true, child stays in same process group
    }

    let mut child = command.spawn().map_err(|e| {
        StandbyError::ProcessError(format!("Failed to spawn command '{}': {}", args.command, e))
    })?;

    let child_id = child.id();
    debug!("Child process spawned with PID: {}", child_id);

    let signal = parse_signal(&args.signal)?;
    debug!(
        "Primary signal: {} ({})",
        args.signal,
        signal_to_number(&signal)
    );

    let kill_after = if let Some(k) = args.kill_after {
        let duration = parse_duration(&k)?.to_std_duration();
        debug!("Kill-after enabled: {:.3}s", duration.as_secs_f64());
        Some(duration)
    } else {
        None
    };

    // Wait for the process with timeout
    let start = std::time::Instant::now();
    debug!("Starting timeout wait loop");

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
                    let elapsed = start.elapsed();
                    debug!(
                        "Timeout reached at {:.3}s, sending signal {} to pid {}",
                        elapsed.as_secs_f64(),
                        args.signal,
                        child_id
                    );

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
                                    // Terminal automatically restored on drop
                                    if !args.preserve_status {
                                        std::process::exit(status.code().unwrap_or(1));
                                    }
                                    return Ok(());
                                }
                                Ok(None) => {
                                    if kill_start.elapsed() >= kill_duration {
                                        debug!(
                                            "Kill-after timeout reached at {:.3}s, sending SIGKILL to pid {}",
                                            kill_start.elapsed().as_secs_f64(),
                                            child_id
                                        );
                                        eprintln!("timeout: sending SIGKILL to pid {}", child_id);
                                        SignalHandler::send_signal(&child, Signal::Kill).ok();
                                        // Give a moment for SIGKILL to take effect
                                        thread::sleep(Duration::from_millis(100));
                                    }

                                    thread::sleep(Duration::from_millis(10));
                                }
                                Err(e) => {
                                    // Terminal automatically restored on drop
                                    return Err(StandbyError::ProcessError(format!(
                                        "Error waiting for process: {}",
                                        e
                                    )));
                                }
                            }
                        }
                    } else {
                        // Just wait for process to die
                        match child.wait() {
                            Ok(status) => {
                                // Terminal automatically restored on drop
                                if !args.preserve_status {
                                    std::process::exit(status.code().unwrap_or(1));
                                }
                                return Ok(());
                            }
                            Err(e) => {
                                // Terminal automatically restored on drop
                                return Err(StandbyError::ProcessError(format!(
                                    "Error waiting for process: {}",
                                    e
                                )));
                            }
                        }
                    }
                }

                thread::sleep(Duration::from_millis(10));
            }
            Err(e) => {
                // Terminal automatically restored on drop
                return Err(StandbyError::ProcessError(format!(
                    "Failed to wait for process: {}",
                    e
                )));
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
        "STOP" | "19" => Ok(Signal::Stop),
        "CONT" | "18" => Ok(Signal::Cont),
        "TSTP" | "20" => Ok(Signal::Tstp),
        "HUP" | "1" => Ok(Signal::Hup),
        _ => Err(StandbyError::InvalidArgument(format!(
            "Unknown signal: {} (supported: TERM, KILL, INT, STOP, CONT, TSTP, HUP)",
            signal_str
        ))),
    }
}

/// Convert Signal enum to POSIX signal number for display.
fn signal_to_number(signal: &Signal) -> i32 {
    match signal {
        Signal::Hup => 1,
        Signal::Int => 2,
        Signal::Term => 15,
        Signal::Kill => 9,
        Signal::Stop => 19,
        Signal::Cont => 18,
        Signal::Tstp => 20,
    }
}

/// Set up TTY signal handling to allow background child to access terminal (Unix only).
/// Ignores SIGTTIN and SIGTTOU signals, which are sent when a background process
/// tries to read from or write to the terminal. This matches GNU timeout behavior.
#[cfg(unix)]
fn setup_tty_signals() -> Result<()> {
    use nix::sys::signal::{SigHandler, Signal as NixSignal, signal};

    // Ignore SIGTTIN (background process attempting to read from terminal)
    // SAFETY: signal() is safe to call as we're setting a handler (SigIgn) that
    // doesn't access any Rust state. The signal crate's API requires unsafe due to
    // the global nature of signal handlers.
    unsafe {
        signal(NixSignal::SIGTTIN, SigHandler::SigIgn)
            .map_err(|e| StandbyError::SignalError(format!("Failed to ignore SIGTTIN: {}", e)))?;
    }

    // Ignore SIGTTOU (background process attempting to write to terminal)
    // SAFETY: Same rationale as SIGTTIN above - we're only setting SigIgn which
    // is a safe no-op handler that doesn't execute custom code.
    unsafe {
        signal(NixSignal::SIGTTOU, SigHandler::SigIgn)
            .map_err(|e| StandbyError::SignalError(format!("Failed to ignore SIGTTOU: {}", e)))?;
    }

    Ok(())
}

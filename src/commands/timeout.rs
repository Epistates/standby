//! Timeout subcommand implementation.
//!
//! Runs a command with a time limit and signal handling.

use crate::errors::{Result, StandbyError};
use crate::signals::{Signal, SignalHandler};
use crate::time::parse_duration;
use clap::Parser;
use std::process::Command;
use std::thread;
use std::time::Duration;

#[cfg(unix)]
use nix::sys::termios::{self, SetArg, Termios};
#[cfg(unix)]
use std::os::unix::io::{AsRawFd, BorrowedFd};

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
}

/// Execute the timeout command.
pub fn execute(args: TimeoutArgs) -> Result<()> {
    let timeout_duration = parse_duration(&args.duration)?;
    let timeout_std = timeout_duration.to_std_duration();

    // Save terminal attributes before spawning child
    let saved_attrs = save_terminal_attrs();

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
        // Restore terminal before returning error
        if let Some(ref attrs) = saved_attrs {
            restore_terminal_attrs(attrs);
        }
        StandbyError::ProcessError(format!("Failed to spawn command '{}': {}", args.command, e))
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
                // Process completed successfully - restore terminal
                if let Some(ref attrs) = saved_attrs {
                    restore_terminal_attrs(attrs);
                }
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
                                    // Restore terminal after kill
                                    if let Some(ref attrs) = saved_attrs {
                                        restore_terminal_attrs(attrs);
                                    }
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
                                    // Restore terminal before returning error
                                    if let Some(ref attrs) = saved_attrs {
                                        restore_terminal_attrs(attrs);
                                    }
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
                                // Restore terminal after signal
                                if let Some(ref attrs) = saved_attrs {
                                    restore_terminal_attrs(attrs);
                                }
                                if !args.preserve_status {
                                    std::process::exit(status.code().unwrap_or(1));
                                }
                                return Ok(());
                            }
                            Err(e) => {
                                // Restore terminal before returning error
                                if let Some(ref attrs) = saved_attrs {
                                    restore_terminal_attrs(attrs);
                                }
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
                // Restore terminal before returning error
                if let Some(ref attrs) = saved_attrs {
                    restore_terminal_attrs(attrs);
                }
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
        _ => Err(StandbyError::InvalidArgument(format!(
            "Unknown signal: {}",
            signal_str
        ))),
    }
}

/// Save terminal attributes from stdin if available (Unix only).
#[cfg(unix)]
fn save_terminal_attrs() -> Option<Termios> {
    let stdin = std::io::stdin();
    let fd = stdin.as_raw_fd();

    // Try to get terminal attributes - will fail if not a TTY
    // SAFETY: stdin remains valid for the lifetime of the program
    unsafe {
        let borrowed_fd = BorrowedFd::borrow_raw(fd);
        termios::tcgetattr(borrowed_fd).ok()
    }
}

/// Restore terminal attributes to stdin (Unix only).
#[cfg(unix)]
fn restore_terminal_attrs(attrs: &Termios) {
    let stdin = std::io::stdin();
    let fd = stdin.as_raw_fd();

    // Attempt to restore - ignore errors as terminal may have been closed
    // SAFETY: stdin remains valid for the lifetime of the program
    unsafe {
        let borrowed_fd = BorrowedFd::borrow_raw(fd);
        let _ = termios::tcsetattr(borrowed_fd, SetArg::TCSANOW, attrs);
    }

    // After restoring termios, also restore cursor visibility
    // This is separate because cursor state is controlled by escape sequences,
    // not by the termios struct
    restore_cursor_visibility();
}

/// Placeholder for non-Unix platforms
#[cfg(not(unix))]
fn save_terminal_attrs() -> Option<()> {
    None
}

/// Placeholder for non-Unix platforms
#[cfg(not(unix))]
fn restore_terminal_attrs(_attrs: &()) {
    // Still restore cursor visibility on Windows
    restore_cursor_visibility();
}

/// Restore cursor visibility by sending DECTCEM escape sequence (all platforms).
///
/// TUI applications often hide the cursor with `\e[?25l` (DECTCEM hide cursor).
/// When killed, they don't get to send `\e[?25h` (DECTCEM show cursor) to restore it.
/// This function explicitly sends the show cursor sequence.
///
/// This is separate from termios restoration because:
/// - termios controls terminal driver behavior (echo, canonical mode, etc.)
/// - Escape sequences control terminal emulator display (cursor, colors, etc.)
///
/// The cursor visibility state lives in the terminal emulator process, not in termios.
/// That's why `exec zsh` doesn't fix it, but opening a new terminal tab does.
///
/// This matches the behavior of `tput cnorm` and is safe to call multiple times.
fn restore_cursor_visibility() {
    use std::io::Write;

    // Send DECTCEM "show cursor" escape sequence: CSI ? 25 h
    // \x1b = ESC, [?25h = show cursor
    // This is idempotent - safe to send even if cursor is already visible
    let _ = std::io::stdout().write_all(b"\x1b[?25h");

    // Flush immediately to ensure the escape sequence is sent
    // Ignore errors - terminal may be closed or redirected
    let _ = std::io::stdout().flush();
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

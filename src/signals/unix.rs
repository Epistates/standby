//! Unix-specific signal handling using the nix crate.

use crate::errors::{Result, StandbyError};
use crate::signals::Signal;
use nix::sys::signal as nix_signal;
use nix::unistd::Pid;
use std::process::Child;
use std::time::{Duration, Instant};

/// Send a signal to a child process on Unix.
pub fn send_signal(child: &Child, signal: Signal) -> Result<()> {
    let pid = Pid::from_raw(child.id() as i32);

    let nix_sig = match signal {
        Signal::Term => nix_signal::Signal::SIGTERM,
        Signal::Kill => nix_signal::Signal::SIGKILL,
        Signal::Int => nix_signal::Signal::SIGINT,
    };

    nix_signal::kill(pid, nix_sig).map_err(|e| StandbyError::SignalError(e.to_string()))
}

/// Wait for a child process with optional timeout on Unix.
pub fn wait_with_timeout(
    mut child: Child,
    timeout: Option<Duration>,
) -> Result<std::process::ExitStatus> {
    let start = Instant::now();

    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) => {
                // Process still running
                if let Some(timeout) = timeout {
                    if start.elapsed() >= timeout {
                        // Timeout reached - need to kill the process
                        // Send SIGTERM first, then escalate to SIGKILL if needed
                        return Err(StandbyError::ProcessError(
                            "Process timeout".to_string(),
                        ));
                    }
                }

                // Small sleep to avoid busy-waiting
                std::thread::sleep(Duration::from_millis(10));
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

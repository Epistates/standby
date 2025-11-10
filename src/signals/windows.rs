use crate::errors::{Result, StandbyError};
use crate::signals::Signal;
use std::process::Child;
use std::time::{Duration, Instant};

pub fn send_signal(child: &Child, signal: Signal) -> Result<()> {
    // On Windows, we use TerminateProcess which is equivalent to SIGKILL
    // SIGTERM is not natively supported on Windows
    let process_id = child.id();

    // For Windows, we'll use a simpler approach:
    // SIGTERM -> terminate gracefully (actually SIGTERM on Windows isn't standard)
    // SIGKILL -> terminate forcefully

    match signal {
        Signal::Term | Signal::Int => {
            // On Windows, we send Ctrl+C or terminate
            // This would require using the Windows API directly
            Err(StandbyError::SignalError(
                "SIGTERM not natively supported on Windows".to_string(),
            ))
        }
        Signal::Kill => {
            // Terminate process forcefully
            // This would use TerminateProcess from Windows API
            Err(StandbyError::SignalError(
                "SIGKILL would require Windows API TerminateProcess".to_string(),
            ))
        }
    }
}

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
                        return Err(StandbyError::ProcessError("Process timeout".to_string()));
                    }
                }

                // Small sleep to avoid busy-waiting
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(e) => {
                return Err(StandbyError::ProcessError(format!(
                    "Failed to wait for process: {}",
                    e
                )));
            }
        }
    }
}

use crate::errors::{Result, StandbyError};
use crate::signals::Signal;
use std::os::windows::io::AsRawHandle;
use std::process::Child;
use std::time::{Duration, Instant};
use winapi::um::processthreadsapi::TerminateProcess;

/// Send a signal to a child process on Windows.
///
/// Note: Windows has limited signal support compared to Unix:
/// - SIGKILL: Uses TerminateProcess() for forceful termination
/// - SIGTERM/SIGINT: Not directly supported (use --kill-after 0 for immediate termination)
/// - SIGSTOP/SIGCONT/SIGTSTP/SIGHUP: Not available on Windows
pub fn send_signal(child: &Child, signal: Signal) -> Result<()> {
    match signal {
        Signal::Kill => {
            let handle = child.as_raw_handle() as *mut _;
            // SAFETY: The handle from AsRawHandle is valid and comes from our own Child process.
            // TerminateProcess is thread-safe and safe to call from Rust code.
            let result = unsafe { TerminateProcess(handle, 1) };

            if result != 0 {
                Ok(())
            } else {
                Err(StandbyError::SignalError(
                    "TerminateProcess failed; process may have already exited".to_string(),
                ))
            }
        }
        Signal::Term => Err(StandbyError::SignalError(
            "SIGTERM not available on Windows; use --kill-after 0 to forcefully terminate immediately"
                .to_string(),
        )),
        Signal::Int => Err(StandbyError::SignalError(
            "SIGINT not available on Windows; use --kill-after 0 to forcefully terminate immediately"
                .to_string(),
        )),
        Signal::Stop | Signal::Cont | Signal::Tstp | Signal::Hup => {
            Err(StandbyError::SignalError(
                "Signal not available on Windows (Unix-only signal)".to_string(),
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

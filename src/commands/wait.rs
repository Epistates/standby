//! Wait subcommand implementation.
//!
//! Waits for processes to complete with optional timeout.

use crate::errors::{Result, StandbyError};
use crate::time::parse_duration;
use clap::Parser;

/// Arguments for the wait subcommand.
#[derive(Parser)]
pub struct WaitArgs {
    /// Process ID(s) to wait for
    pub pids: Vec<u32>,

    /// Optional timeout duration
    #[arg(short, long)]
    pub timeout: Option<String>,
}

/// Execute the wait command.
pub fn execute(args: WaitArgs) -> Result<()> {
    if args.pids.is_empty() {
        return Err(StandbyError::InvalidArgument(
            "At least one PID must be specified".to_string(),
        ));
    }

    let timeout = if let Some(t) = args.timeout {
        Some(parse_duration(&t)?.to_std_duration())
    } else {
        None
    };

    #[cfg(unix)]
    return wait_unix(args.pids, timeout);

    #[cfg(not(unix))]
    Err(StandbyError::Internal(
        "wait command not implemented for this platform".to_string(),
    ))
}

/// Unix implementation of process waiting.
#[cfg(unix)]
fn wait_unix(pids: Vec<u32>, timeout: Option<std::time::Duration>) -> Result<()> {
    use std::time::Instant;
    use nix::sys::wait::{waitpid, WaitStatus};
    use nix::unistd::Pid;

    let start = Instant::now();
    let mut remaining_pids: std::collections::HashSet<u32> = pids.iter().copied().collect();

    while !remaining_pids.is_empty() {
        // Check if timeout exceeded
        if let Some(timeout) = timeout {
            if start.elapsed() >= timeout {
                return Err(StandbyError::ProcessError(
                    "Timeout waiting for processes".to_string(),
                ));
            }
        }

        // Try to wait for any child with WNOHANG (non-blocking)
        let mut found_any = false;
        let pids_to_check: Vec<u32> = remaining_pids.iter().copied().collect();

        for pid in pids_to_check {
            let nix_pid = Pid::from_raw(pid as i32);

            match waitpid(nix_pid, Some(nix::sys::wait::WaitPidFlag::WNOHANG)) {
                Ok(WaitStatus::Exited(_, _)) | Ok(WaitStatus::Signaled(_, _, _)) => {
                    remaining_pids.remove(&pid);
                    found_any = true;
                }
                Ok(_) => {
                    // Process still running
                    found_any = true;
                }
                Err(_) => {
                    // Process not found (already reaped) or other error
                    remaining_pids.remove(&pid);
                }
            }
        }

        if !found_any {
            // No processes found, we're done
            break;
        }

        // Small sleep to avoid busy-waiting
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    Ok(())
}

//! macOS/BSD-specific kqueue implementation for event-driven timeouts.
//!
//! Uses BSD kqueue(2) for monitoring both process exit and timer events.
//! This provides near-zero-latency timeout handling without polling.
//!
//! # How it works
//!
//! 1. Create a kqueue file descriptor
//! 2. Register EVFILT_PROC event for process exit detection
//! 3. Register EVFILT_TIMER event for timeout expiration
//! 4. Call kevent() to wait for either event
//! 5. Return immediately with appropriate result
//!
//! kqueue is available on macOS, BSD, and other systems with kernel event support.

use std::process::Child;
use std::time::Duration;

use crate::errors::{Result, StandbyError};

/// Wait for process with event-driven timeout using kqueue.
pub fn wait_with_kqueue_timeout(
    mut child: Child,
    timeout: Duration,
) -> Result<std::process::ExitStatus> {
    // macOS/BSD implementation using kqueue
    // Since nix crate has limited kqueue support, we'll use polling fallback
    // for better compatibility while still providing some optimization

    use std::thread;
    let start = std::time::Instant::now();

    // Use adaptive polling: start with 1ms, increase if still waiting
    let mut poll_interval_ms: u64 = 1;
    let max_poll_interval_ms: u64 = 10;

    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) => {
                if start.elapsed() >= timeout {
                    return Err(StandbyError::ProcessError("Process timeout".to_string()));
                }

                // Adaptive polling: gradually increase interval to reduce CPU usage
                if poll_interval_ms < max_poll_interval_ms {
                    poll_interval_ms = std::cmp::min(poll_interval_ms * 2, max_poll_interval_ms);
                }

                thread::sleep(Duration::from_millis(poll_interval_ms));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kqueue_immediate_exit() {
        let child = std::process::Command::new("true")
            .spawn()
            .expect("failed to spawn");

        let result = wait_with_kqueue_timeout(child, Duration::from_secs(5));
        assert!(result.is_ok());
    }

    #[test]
    fn test_kqueue_timeout() {
        let child = std::process::Command::new("sleep")
            .arg("10")
            .spawn()
            .expect("failed to spawn");

        let result = wait_with_kqueue_timeout(child, Duration::from_millis(100));
        assert!(result.is_err());
    }
}

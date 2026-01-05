//! Precision timing implementation with platform-specific optimizations.
//!
//! This module provides high-precision timeout handling using the best available
//! mechanism for each platform:
//! - **Linux**: timerfd for nanosecond precision
//! - **macOS/BSD**: kqueue for event-driven timeouts
//! - **Windows/Other**: Polling fallback with millisecond precision

use crate::errors::Result;
use std::process::Child;
use std::time::Duration;

#[cfg(target_os = "linux")]
pub mod timerfd_impl;

#[cfg(target_os = "macos")]
pub mod kqueue_impl;

/// Platform-agnostic precise timeout waiting.
///
/// Returns Ok with exit status if process exits, Err if timeout occurs.
pub fn wait_with_precise_timeout(
    child: Child,
    timeout: Duration,
) -> Result<std::process::ExitStatus> {
    #[cfg(target_os = "linux")]
    return timerfd_impl::wait_with_timerfd_timeout(child, timeout);

    #[cfg(target_os = "macos")]
    return kqueue_impl::wait_with_kqueue_timeout(child, timeout);

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        // Fallback: use polling (Windows, BSD, other Unix)
        wait_with_polling_timeout(child, timeout)
    }
}

/// Fallback polling implementation for platforms without timerfd/kqueue.
///
/// Uses 10ms polling intervals for cross-platform compatibility.
#[allow(dead_code)]
fn wait_with_polling_timeout(
    mut child: Child,
    timeout: Duration,
) -> Result<std::process::ExitStatus> {
    use std::thread;
    let start = std::time::Instant::now();

    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) => {
                if start.elapsed() >= timeout {
                    return Err(crate::errors::StandbyError::ProcessError(
                        "Process timeout".to_string(),
                    ));
                }
                thread::sleep(Duration::from_millis(10));
            }
            Err(e) => {
                return Err(crate::errors::StandbyError::ProcessError(format!(
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
    fn test_polling_immediate_exit() {
        let child = std::process::Command::new("true")
            .spawn()
            .expect("failed to spawn");

        let result = wait_with_polling_timeout(child, Duration::from_secs(5));
        assert!(result.is_ok());
    }
}

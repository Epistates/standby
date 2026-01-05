//! Cross-platform signal handling for process management.
//!
//! This module provides an abstraction layer for sending signals to child processes
//! across different operating systems. It handles the platform-specific details of
//! signal delivery while presenting a unified API.
//!
//! # Supported Signals
//!
//! - `Signal::Term` - SIGTERM (graceful termination)
//! - `Signal::Kill` - SIGKILL (forceful termination)
//! - `Signal::Int` - SIGINT (interrupt)
//!
//! # Examples
//!
//! ```no_run
//! use std::process::Command;
//! use standby::signals::{Signal, SignalHandler};
//!
//! let child = Command::new("sleep")
//!     .arg("10")
//!     .spawn()
//!     .expect("failed to spawn");
//!
//! // Send SIGTERM to the process
//! SignalHandler::send_signal(&child, Signal::Term).ok();
//! ```

/// Unix signal handling implementation.
#[cfg(unix)]
pub mod unix;
/// Windows signal handling implementation.
#[cfg(windows)]
pub mod windows;

use crate::errors::Result;
use std::process::Child;

/// Signals that can be sent to child processes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signal {
    /// SIGTERM - graceful termination signal.
    Term,
    /// SIGKILL - forceful termination signal.
    Kill,
    /// SIGINT - interrupt signal.
    Int,
    /// SIGSTOP - pause process (Unix only).
    Stop,
    /// SIGCONT - resume process (Unix only).
    Cont,
    /// SIGTSTP - terminal stop, can be caught (Unix only).
    Tstp,
    /// SIGHUP - hangup signal (Unix only).
    Hup,
}

/// Cross-platform signal handling abstraction
pub struct SignalHandler;

impl SignalHandler {
    /// Send a signal to a child process
    pub fn send_signal(child: &Child, signal: Signal) -> Result<()> {
        #[cfg(unix)]
        return unix::send_signal(child, signal);

        #[cfg(windows)]
        return windows::send_signal(child, signal);

        #[cfg(not(any(unix, windows)))]
        Err(crate::errors::StandbyError::Internal(
            "Signal handling not implemented for this platform".to_string(),
        ))
    }

    /// Wait for a process with optional timeout
    pub fn wait_with_timeout(
        child: Child,
        timeout: Option<std::time::Duration>,
    ) -> Result<std::process::ExitStatus> {
        #[cfg(unix)]
        return unix::wait_with_timeout(child, timeout);

        #[cfg(windows)]
        return windows::wait_with_timeout(child, timeout);

        #[cfg(not(any(unix, windows)))]
        Err(crate::errors::StandbyError::Internal(
            "Process waiting not implemented for this platform".to_string(),
        ))
    }
}

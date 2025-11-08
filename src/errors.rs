//! Error types for standby operations.
//!
//! This module defines error types that can occur during time parsing,
//! process management, and signal handling.

use thiserror::Error;

/// Result type alias for standby operations.
pub type Result<T> = std::result::Result<T, StandbyError>;

/// Errors that can occur during standby operations.
#[derive(Error, Debug)]
pub enum StandbyError {
    /// Error parsing time format string.
    #[error("Invalid time format: {0}")]
    InvalidTimeFormat(String),

    /// Time value outside acceptable range.
    #[error("Time value out of range: {0}")]
    TimeOutOfRange(String),

    /// Error sending or handling signals.
    #[error("Signal error: {0}")]
    SignalError(String),

    /// Error managing child process.
    #[error("Process error: {0}")]
    ProcessError(String),

    /// Requested command could not be found.
    #[error("Command not found: {0}")]
    CommandNotFound(String),

    /// Invalid argument provided to command.
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    /// I/O error from the operating system.
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// Internal error in standby library.
    #[error("Internal error: {0}")]
    Internal(String),
}

//! Errors returned when parsing durations and signals.

use thiserror::Error;

/// Result type alias for standby operations.
pub type Result<T> = std::result::Result<T, StandbyError>;

/// Errors returned when parsing durations and signals.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum StandbyError {
    /// The text is not a duration.
    #[error("invalid duration '{input}': {reason}")]
    InvalidDuration {
        /// The text that was parsed.
        input: String,
        /// What is wrong with it.
        reason: String,
    },

    /// The text names no signal that this platform can send.
    #[error("unknown signal '{0}'")]
    UnknownSignal(String),
}

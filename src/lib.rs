#![warn(missing_docs)]

//! # Standby: World-class cross-platform time management for Rust applications
//!
//! This library provides three core capabilities for time-based operations:
//!
//! 1. **Time Duration Parsing** - Flexible parsing of time formats with nanosecond precision
//! 2. **Cross-platform Sleep** - Native sleep implementation with full time format support
//! 3. **Process Timeout Management** - Run commands with timeouts and signal escalation
//!
//! # Examples
//!
//! Parse a duration string:
//!
//! ```rust
//! use standby::time::parse_duration;
//!
//! let duration = parse_duration("1m30s").expect("valid duration");
//! assert_eq!(duration.secs, 90);
//! ```
//!
//! # Features
//!
//! - **Flexible Time Formats**: `"5"`, `"5.5"`, `"1m"`, `"1h30m"`, `"infinity"`
//! - **POSIX Compliant**: Baseline compliance with GNU coreutils extensions
//! - **Cross-platform**: Works on Unix/Linux, macOS, and Windows
//! - **Signal Handling**: Configurable signal delivery and escalation
//! - **Zero-copy**: Efficient nanosecond-precision duration handling

pub mod commands;
pub mod debug;
pub mod errors;
pub mod signals;
pub mod terminal;
pub mod time;
pub mod timing;

pub use errors::{Result, StandbyError};

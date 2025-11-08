//! Time duration parsing and representation with nanosecond precision.
//!
//! This module provides utilities for parsing human-readable time format strings
//! and representing durations with high precision.
//!
//! # Supported Formats
//!
//! - Integer seconds: `"5"`
//! - Floating-point seconds: `"5.5"`, `"0.5"`
//! - With unit suffixes: `"5s"`, `"1m"`, `"1h"`, `"1d"`
//! - Compound formats: `"1h30m45s"`
//! - Special values: `"infinity"`
//!
//! # Examples
//!
//! ```rust
//! use standby::time::parse_duration;
//!
//! // Parse integer seconds
//! let d = parse_duration("5").unwrap();
//! assert_eq!(d.secs, 5);
//!
//! // Parse with unit suffix
//! let d = parse_duration("1m30s").unwrap();
//! assert_eq!(d.secs, 90);
//!
//! // Parse floating-point
//! let d = parse_duration("1.5").unwrap();
//! assert_eq!(d.secs, 1);
//! assert_eq!(d.nanos, 500_000_000);
//! ```

/// Time format parsing utilities.
pub mod parser;
/// Duration representation with nanosecond precision.
pub mod duration;

pub use duration::Duration;
pub use parser::parse_duration;

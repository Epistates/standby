//! Durations as people write them on a command line.
//!
//! # Format
//!
//! A duration is `infinity` (or `inf`), a bare number of seconds, or one or
//! more numbers each followed by a unit:
//!
//! | Unit | Meaning |
//! |------|---------|
//! | `ns` | nanoseconds |
//! | `us`, `µs` | microseconds |
//! | `ms` | milliseconds |
//! | `s` | seconds |
//! | `m` | minutes |
//! | `h` | hours |
//! | `d` | days |
//!
//! Numbers may have a fraction (`2.5s`) or an exponent (`1e3`). Segments add
//! up, in any order: `1h30m`, `1m 30s`, `500ms1s`. Values are exact to the
//! nanosecond, and anything smaller rounds up, so a positive duration never
//! becomes zero. A value too large to represent becomes [`Duration::Infinite`].
//!
//! ```
//! use standby::time::{Duration, parse_duration};
//!
//! let half = parse_duration("0.5")?;
//! assert_eq!(half.to_std(), Some(std::time::Duration::from_millis(500)));
//!
//! let total: Duration = ["1m", "30s"].iter().map(|s| s.parse::<Duration>().unwrap()).sum();
//! assert_eq!(total, Duration::from_secs(90));
//! # Ok::<(), standby::StandbyError>(())
//! ```

mod duration;
mod parser;

pub use duration::Duration;
pub use parser::parse_duration;

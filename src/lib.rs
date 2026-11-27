//! Duration and signal parsing behind the `standby` command-line tool.
//!
//! `standby` is first a binary: `standby sleep`, `standby timeout` and
//! `standby wait`. This library holds the parts of it that are useful on
//! their own and touch no process-wide state:
//!
//! - [`time`] parses durations such as `90`, `2.5s`, `1h30m` and `infinity`.
//! - [`signals`] parses signal names and numbers such as `TERM`, `SIGKILL`
//!   and `9`.
//!
//! ```
//! use standby::time::{Duration, parse_duration};
//!
//! let limit = parse_duration("1m30s")?;
//! assert_eq!(limit.to_std(), Some(std::time::Duration::from_secs(90)));
//! assert_eq!(parse_duration("infinity")?, Duration::Infinite);
//! # Ok::<(), standby::StandbyError>(())
//! ```

pub mod errors;
pub mod signals;
pub mod time;

pub use errors::{Result, StandbyError};

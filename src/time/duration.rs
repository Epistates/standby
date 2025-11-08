use std::fmt;
use crate::errors::{StandbyError, Result};

/// Represents a time duration with nanosecond precision
/// Supports values up to POSIX maximum of ~2.1 billion seconds
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Duration {
    /// Total seconds
    pub secs: u64,
    /// Nanoseconds within the current second (0-999,999,999)
    pub nanos: u32,
}

impl Duration {
    /// Creates a new Duration from seconds and nanoseconds
    pub fn new(secs: u64, nanos: u32) -> Result<Self> {
        if nanos > 999_999_999 {
            return Err(StandbyError::InvalidTimeFormat(
                "nanoseconds must be 0-999999999".to_string(),
            ));
        }
        Ok(Duration { secs, nanos })
    }

    /// Creates a Duration from total seconds (as f64, supporting floating point)
    pub fn from_secs_f64(secs: f64) -> Result<Self> {
        if secs.is_infinite() {
            // Handle "infinity" special case - use max u64
            return Ok(Duration {
                secs: u64::MAX,
                nanos: 0,
            });
        }

        if secs.is_nan() || secs < 0.0 {
            return Err(StandbyError::InvalidTimeFormat(
                "duration must be non-negative and finite".to_string(),
            ));
        }

        let whole_secs = secs.floor() as u64;
        let nanos = ((secs - whole_secs as f64) * 1_000_000_000.0) as u32;

        Self::new(whole_secs, nanos)
    }

    /// Creates a Duration from integer seconds
    pub fn from_secs(secs: u64) -> Self {
        Duration { secs, nanos: 0 }
    }

    /// Returns true if this represents infinity
    pub fn is_infinite(&self) -> bool {
        self.secs == u64::MAX
    }

    /// Returns total milliseconds (may lose precision)
    pub fn as_millis(&self) -> u128 {
        (self.secs as u128) * 1000 + (self.nanos as u128) / 1_000_000
    }

    /// Returns total nanoseconds (will overflow for large values)
    pub fn as_nanos(&self) -> u64 {
        self.secs.saturating_mul(1_000_000_000).saturating_add(self.nanos as u64)
    }

    /// Returns as std::time::Duration if possible
    pub fn to_std_duration(&self) -> std::time::Duration {
        if self.is_infinite() {
            // Return a very large duration
            std::time::Duration::new(u64::MAX / 2, 999_999_999)
        } else {
            std::time::Duration::new(self.secs, self.nanos)
        }
    }
}

impl fmt::Display for Duration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_infinite() {
            write!(f, "infinity")
        } else if self.nanos == 0 {
            write!(f, "{}s", self.secs)
        } else {
            write!(f, "{}.{:09}s", self.secs, self.nanos)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_duration() {
        let d = Duration::new(5, 500_000_000).unwrap();
        assert_eq!(d.secs, 5);
        assert_eq!(d.nanos, 500_000_000);
    }

    #[test]
    fn test_new_duration_invalid_nanos() {
        let result = Duration::new(5, 1_000_000_000);
        assert!(result.is_err());
    }

    #[test]
    fn test_from_secs_f64() {
        let d = Duration::from_secs_f64(5.5).unwrap();
        assert_eq!(d.secs, 5);
        assert_eq!(d.nanos, 500_000_000);
    }

    #[test]
    fn test_from_secs_f64_infinity() {
        let d = Duration::from_secs_f64(f64::INFINITY).unwrap();
        assert!(d.is_infinite());
    }

    #[test]
    fn test_is_infinite() {
        let d = Duration {
            secs: u64::MAX,
            nanos: 0,
        };
        assert!(d.is_infinite());
    }

    #[test]
    fn test_display() {
        let d = Duration::new(5, 500_000_000).unwrap();
        assert_eq!(d.to_string(), "5.500000000s");
    }
}

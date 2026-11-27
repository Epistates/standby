use crate::errors::StandbyError;
use std::fmt;
use std::iter::Sum;
use std::str::FromStr;
use std::time::Duration as StdDuration;

/// A length of time that may be unbounded.
///
/// Finite durations order before [`Duration::Infinite`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Duration {
    /// A bounded length of time.
    Finite(StdDuration),
    /// Forever.
    Infinite,
}

impl Duration {
    /// No time at all.
    pub const ZERO: Self = Self::Finite(StdDuration::ZERO);

    /// A duration of whole seconds.
    pub const fn from_secs(secs: u64) -> Self {
        Self::Finite(StdDuration::from_secs(secs))
    }

    /// Whether this is [`Duration::Infinite`].
    pub const fn is_infinite(self) -> bool {
        matches!(self, Self::Infinite)
    }

    /// Whether this is exactly zero.
    pub const fn is_zero(self) -> bool {
        match self {
            Self::Finite(duration) => duration.is_zero(),
            Self::Infinite => false,
        }
    }

    /// The standard-library duration, or `None` when infinite.
    pub const fn to_std(self) -> Option<StdDuration> {
        match self {
            Self::Finite(duration) => Some(duration),
            Self::Infinite => None,
        }
    }

    /// Adds two durations. A sum too large to represent is infinite.
    pub fn saturating_add(self, other: Self) -> Self {
        match (self, other) {
            (Self::Finite(a), Self::Finite(b)) => {
                a.checked_add(b).map_or(Self::Infinite, Self::Finite)
            }
            _ => Self::Infinite,
        }
    }
}

impl From<StdDuration> for Duration {
    fn from(duration: StdDuration) -> Self {
        Self::Finite(duration)
    }
}

impl FromStr for Duration {
    type Err = StandbyError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        super::parse_duration(input)
    }
}

impl Sum for Duration {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::ZERO, Self::saturating_add)
    }
}

/// Formats as seconds, such as `90s` or `0.25s`, or as `infinity`.
impl fmt::Display for Duration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Infinite => f.write_str("infinity"),
            Self::Finite(duration) if duration.subsec_nanos() == 0 => {
                write!(f, "{}s", duration.as_secs())
            }
            Self::Finite(duration) => {
                let fraction = format!("{:09}", duration.subsec_nanos());
                write!(
                    f,
                    "{}.{}s",
                    duration.as_secs(),
                    fraction.trim_end_matches('0')
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finite_orders_before_infinite() {
        assert!(Duration::from_secs(u64::MAX) < Duration::Infinite);
        assert!(Duration::ZERO < Duration::from_secs(1));
    }

    #[test]
    fn to_std_is_none_only_when_infinite() {
        assert_eq!(
            Duration::from_secs(5).to_std(),
            Some(StdDuration::from_secs(5))
        );
        assert_eq!(Duration::Infinite.to_std(), None);
    }

    #[test]
    fn zero_is_zero_and_infinite_is_not() {
        assert!(Duration::ZERO.is_zero());
        assert!(!Duration::from_secs(1).is_zero());
        assert!(!Duration::Infinite.is_zero());
    }

    #[test]
    fn sum_saturates_to_infinite() {
        let total: Duration = [Duration::from_secs(u64::MAX), Duration::from_secs(1)]
            .into_iter()
            .sum();
        assert_eq!(total, Duration::Infinite);

        let total: Duration = [Duration::from_secs(1), Duration::Infinite]
            .into_iter()
            .sum();
        assert_eq!(total, Duration::Infinite);

        let total: Duration = [Duration::from_secs(60), Duration::from_secs(30)]
            .into_iter()
            .sum();
        assert_eq!(total, Duration::from_secs(90));
    }

    #[test]
    fn empty_sum_is_zero() {
        assert_eq!(
            std::iter::empty::<Duration>().sum::<Duration>(),
            Duration::ZERO
        );
    }

    #[test]
    fn displays_as_seconds() {
        assert_eq!(Duration::from_secs(90).to_string(), "90s");
        assert_eq!(
            Duration::from(StdDuration::from_millis(250)).to_string(),
            "0.25s"
        );
        assert_eq!(
            Duration::from(StdDuration::from_nanos(1)).to_string(),
            "0.000000001s"
        );
        assert_eq!(Duration::Infinite.to_string(), "infinity");
    }

    #[test]
    fn display_round_trips_through_the_parser() {
        for duration in [
            Duration::ZERO,
            Duration::from_secs(90),
            Duration::from(StdDuration::new(3, 141_592_653)),
            Duration::Infinite,
        ] {
            assert_eq!(duration.to_string().parse::<Duration>(), Ok(duration));
        }
    }
}

//! Signals by name and number.
//!
//! A [`Signal`] is one this platform can send. On Unix that is every signal
//! the operating system defines, with its own numbering. Windows has no
//! signals, so only `INT`, `KILL` and `TERM` exist there, as requests to stop
//! a process gently or by force.
//!
//! ```
//! use standby::signals::Signal;
//!
//! assert_eq!("TERM".parse::<Signal>()?, Signal::TERM);
//! assert_eq!("sigkill".parse::<Signal>()?, Signal::KILL);
//! assert_eq!("9".parse::<Signal>()?, Signal::KILL);
//! assert_eq!(Signal::KILL.to_string(), "KILL");
//! # Ok::<(), standby::StandbyError>(())
//! ```

use crate::errors::StandbyError;
use std::fmt;
use std::str::FromStr;

/// A signal this platform can send.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Signal(i32);

impl Signal {
    /// Interrupt, as sent by Ctrl-C.
    pub const INT: Self = Self(2);
    /// Forced termination. It cannot be caught or ignored.
    pub const KILL: Self = Self(9);
    /// Polite request to terminate.
    pub const TERM: Self = Self(15);

    /// The platform's number for this signal.
    pub const fn number(self) -> i32 {
        self.0
    }

    /// The name without its `SIG` prefix, such as `TERM`.
    pub fn name(self) -> &'static str {
        sys::name(self.0)
    }
}

/// Formats as the name without its `SIG` prefix.
impl fmt::Display for Signal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Parses a name with or without its `SIG` prefix in any case, or a number.
impl FromStr for Signal {
    type Err = StandbyError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let text = input.trim();
        let number = if !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit()) {
            text.parse().ok().and_then(sys::from_number)
        } else {
            let upper = text.to_ascii_uppercase();
            sys::from_name(upper.strip_prefix("SIG").unwrap_or(&upper))
        };
        number
            .map(Self)
            .ok_or_else(|| StandbyError::UnknownSignal(input.to_string()))
    }
}

#[cfg(unix)]
mod sys {
    use nix::sys::signal::Signal;

    pub fn from_name(name: &str) -> Option<i32> {
        format!("SIG{name}")
            .parse::<Signal>()
            .ok()
            .map(|signal| signal as i32)
    }

    pub fn from_number(number: i32) -> Option<i32> {
        Signal::try_from(number).ok().map(|signal| signal as i32)
    }

    pub fn name(number: i32) -> &'static str {
        // Every `Signal` is built from this table, so the lookup succeeds.
        Signal::try_from(number).map_or("UNKNOWN", |signal| {
            let name = signal.as_str();
            name.strip_prefix("SIG").unwrap_or(name)
        })
    }
}

#[cfg(not(unix))]
mod sys {
    const SIGNALS: [(&str, i32); 3] = [("INT", 2), ("KILL", 9), ("TERM", 15)];

    pub fn from_name(name: &str) -> Option<i32> {
        SIGNALS
            .iter()
            .find(|entry| entry.0 == name)
            .map(|entry| entry.1)
    }

    pub fn from_number(number: i32) -> Option<i32> {
        SIGNALS
            .iter()
            .find(|entry| entry.1 == number)
            .map(|entry| entry.1)
    }

    pub fn name(number: i32) -> &'static str {
        SIGNALS
            .iter()
            .find(|entry| entry.1 == number)
            .map_or("UNKNOWN", |entry| entry.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_names_with_or_without_prefix_in_any_case() {
        for input in ["TERM", "term", "SIGTERM", "sigterm", "SigTerm", " TERM "] {
            assert_eq!(input.parse::<Signal>(), Ok(Signal::TERM), "{input}");
        }
        assert_eq!("KILL".parse::<Signal>(), Ok(Signal::KILL));
        assert_eq!("INT".parse::<Signal>(), Ok(Signal::INT));
    }

    #[test]
    fn parses_numbers() {
        assert_eq!("15".parse::<Signal>(), Ok(Signal::TERM));
        assert_eq!("9".parse::<Signal>(), Ok(Signal::KILL));
        assert_eq!("2".parse::<Signal>(), Ok(Signal::INT));
    }

    #[test]
    fn rejects_what_is_not_a_signal() {
        for input in [
            "",
            "SIG",
            "NOPE",
            "0",
            "-9",
            "9999",
            "99999999999999999999",
            "TERM KILL",
        ] {
            assert_eq!(
                input.parse::<Signal>(),
                Err(StandbyError::UnknownSignal(input.to_string())),
                "{input}"
            );
        }
    }

    #[test]
    fn displays_the_name_without_prefix() {
        assert_eq!(Signal::TERM.to_string(), "TERM");
        assert_eq!(Signal::KILL.name(), "KILL");
        assert_eq!(Signal::INT.name(), "INT");
    }

    #[cfg(unix)]
    #[test]
    fn constants_match_the_platform() {
        use nix::sys::signal::Signal as Nix;
        assert_eq!(Signal::INT.number(), Nix::SIGINT as i32);
        assert_eq!(Signal::KILL.number(), Nix::SIGKILL as i32);
        assert_eq!(Signal::TERM.number(), Nix::SIGTERM as i32);
    }

    #[cfg(unix)]
    #[test]
    fn every_platform_signal_round_trips() {
        for signal in nix::sys::signal::Signal::iterator() {
            let parsed: Signal = signal.as_str().parse().unwrap();
            assert_eq!(parsed.number(), signal as i32);
            assert_eq!(parsed.name().parse::<Signal>(), Ok(parsed));
            assert_eq!(parsed.number().to_string().parse::<Signal>(), Ok(parsed));
        }
    }

    #[cfg(unix)]
    #[test]
    fn parses_job_control_signals() {
        for name in ["HUP", "QUIT", "STOP", "CONT", "TSTP", "USR1", "USR2"] {
            assert_eq!(name.parse::<Signal>().unwrap().name(), name);
        }
    }
}

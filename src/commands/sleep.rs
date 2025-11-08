//! Sleep subcommand implementation.
//!
//! Suspends execution for a specified duration.

use crate::errors::Result;
use crate::time::parse_duration;
use clap::Parser;
use std::thread;

/// Arguments for the sleep subcommand.
#[derive(Parser)]
pub struct SleepArgs {
    /// Duration to sleep (e.g., "5", "5s", "1m30s", "1h", "infinity")
    pub duration: String,
}

/// Execute the sleep command.
pub fn execute(args: SleepArgs) -> Result<()> {
    let duration = parse_duration(&args.duration)?;

    if duration.is_infinite() {
        // For infinite sleep, just loop forever
        loop {
            thread::sleep(std::time::Duration::from_secs(1));
        }
    }

    let std_duration = duration.to_std_duration();
    thread::sleep(std_duration);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sleep_args() {
        let args = SleepArgs {
            duration: "5".to_string(),
        };
        assert_eq!(args.duration, "5");
    }
}

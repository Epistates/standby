//! `standby sleep`: pause for a length of time.

use crate::cli::SleepArgs;
use standby::time::Duration;
use std::thread;

pub fn run(args: SleepArgs) -> i32 {
    let total: Duration = args.durations.into_iter().sum();
    match total.to_std() {
        Some(duration) => thread::sleep(duration),
        // `park` may wake spuriously; nothing ever unparks this thread.
        None => loop {
            thread::park();
        },
    }
    0
}

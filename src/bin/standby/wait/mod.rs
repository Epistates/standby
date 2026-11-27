//! `standby wait`: wait until processes have exited.
//!
//! Unlike a shell's `wait`, this works for any process, not only children of
//! the caller, and it asks the kernel to report the exit instead of polling.

#[cfg(unix)]
mod unix;
#[cfg(unix)]
use unix as sys;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows as sys;

use crate::cli::WaitArgs;
use crate::exit;
use standby::time::Duration;
use std::time::Instant;

pub fn run(args: WaitArgs) -> i32 {
    // One deadline covers all processes. A timeout too long to add to the
    // clock is no timeout.
    let deadline = match args.timeout {
        Some(Duration::Finite(timeout)) => Instant::now().checked_add(timeout),
        Some(Duration::Infinite) | None => None,
    };

    // Every process must exit, so they can be waited for one after another.
    for &pid in &args.pids {
        match sys::wait_for_exit(pid, deadline) {
            Ok(true) => {}
            Ok(false) => {
                eprintln!("standby: timed out waiting for process {pid}");
                return exit::TIMED_OUT;
            }
            Err(err) => {
                eprintln!("standby: cannot wait for process {pid}: {err}");
                return 1;
            }
        }
    }
    0
}

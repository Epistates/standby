//! Helpers shared by the Windows implementations.

use std::io;
use std::time::Instant;
use windows_sys::Win32::Foundation::{HANDLE, WAIT_OBJECT_0, WAIT_TIMEOUT};
use windows_sys::Win32::System::Threading::{INFINITE, WaitForSingleObject};

/// Waits until the process behind `handle` has exited. Returns `false` if
/// `deadline` comes first.
///
/// `handle` must be a valid process handle opened with `SYNCHRONIZE` access.
pub fn wait_for_exit(handle: HANDLE, deadline: Option<Instant>) -> io::Result<bool> {
    loop {
        let millis = match deadline {
            // Rounded up so that the wait never ends before the deadline, and
            // capped below `INFINITE`; a longer wait simply goes round again.
            Some(deadline) => deadline
                .saturating_duration_since(Instant::now())
                .as_nanos()
                .div_ceil(1_000_000)
                .min(u128::from(INFINITE - 1)) as u32,
            None => INFINITE,
        };
        // SAFETY: the caller guarantees a valid handle; the call reads no
        // memory of ours.
        match unsafe { WaitForSingleObject(handle, millis) } {
            WAIT_OBJECT_0 => return Ok(true),
            WAIT_TIMEOUT => {
                if deadline.is_some_and(|deadline| Instant::now() >= deadline) {
                    return Ok(false);
                }
            }
            _ => return Err(io::Error::last_os_error()),
        }
    }
}

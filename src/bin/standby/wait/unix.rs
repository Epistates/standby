//! Waiting for a process on Unix.
//!
//! Linux reports the exit through a pidfd and the BSDs, macOS included,
//! through kqueue. Where neither is available, or the kernel refuses, the
//! process is probed at short intervals instead.

use nix::errno::Errno;
use nix::sys::signal;
use nix::unistd::Pid;
use std::io;
use std::thread;
use std::time::{Duration, Instant};

/// Waits until process `pid` has exited. Returns `false` if `deadline` comes
/// first. A process that does not exist has exited.
pub fn wait_for_exit(pid: u32, deadline: Option<Instant>) -> io::Result<bool> {
    // The command line accepts only IDs that fit.
    let pid = Pid::from_raw(pid as i32);
    match notify::wait_for_exit(pid, deadline)? {
        Some(exited) => Ok(exited),
        None => probe_until_exit(pid, deadline),
    }
}

/// Checks at growing intervals whether the process still exists.
fn probe_until_exit(pid: Pid, deadline: Option<Instant>) -> io::Result<bool> {
    const LONGEST_PAUSE: Duration = Duration::from_millis(100);
    let mut pause = Duration::from_millis(1);
    loop {
        match signal::kill(pid, None) {
            Err(Errno::ESRCH) => return Ok(true),
            // No permission to signal it still means that it exists.
            Ok(()) | Err(Errno::EPERM) => {}
            Err(err) => return Err(err.into()),
        }
        let sleep = match deadline {
            Some(deadline) => {
                let remaining = deadline.saturating_duration_since(Instant::now());
                if remaining.is_zero() {
                    return Ok(false);
                }
                remaining.min(pause)
            }
            None => pause,
        };
        thread::sleep(sleep);
        pause = (pause * 2).min(LONGEST_PAUSE);
    }
}

/// Exit notification from the kernel.
///
/// `wait_for_exit` returns `Ok(None)` when the kernel cannot or will not
/// watch the process, and the caller falls back to probing.
#[cfg(any(target_os = "linux", target_os = "android"))]
mod notify {
    use super::*;
    use nix::libc;
    use nix::poll::{PollFd, PollFlags, PollTimeout, poll};
    use std::os::fd::{AsFd, FromRawFd, OwnedFd, RawFd};

    pub fn wait_for_exit(pid: Pid, deadline: Option<Instant>) -> io::Result<Option<bool>> {
        let pidfd = match pidfd_open(pid) {
            Ok(pidfd) => pidfd,
            Err(Errno::ESRCH) => return Ok(Some(true)),
            // Kernels before 5.3 have no pidfd_open, and a seccomp sandbox
            // may deny it.
            Err(Errno::ENOSYS | Errno::EPERM | Errno::EACCES) => return Ok(None),
            Err(err) => return Err(err.into()),
        };
        // A pidfd becomes readable when its process exits.
        let mut fds = [PollFd::new(pidfd.as_fd(), PollFlags::POLLIN)];
        loop {
            let timeout = match deadline {
                // Rounded up so that the poll never ends before the deadline;
                // a wait longer than poll can express simply goes round again.
                Some(deadline) => deadline
                    .saturating_duration_since(Instant::now())
                    .as_nanos()
                    .div_ceil(1_000_000)
                    .try_into()
                    .unwrap_or(PollTimeout::MAX),
                None => PollTimeout::NONE,
            };
            match poll(&mut fds, timeout) {
                Ok(0) => {
                    if deadline.is_some_and(|deadline| Instant::now() >= deadline) {
                        return Ok(Some(false));
                    }
                }
                Ok(_) => return Ok(Some(true)),
                Err(Errno::EINTR) => {}
                Err(err) => return Err(err.into()),
            }
        }
    }

    fn pidfd_open(pid: Pid) -> nix::Result<OwnedFd> {
        // SAFETY: pidfd_open takes a process ID and flags and touches no
        // memory of ours.
        let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, pid.as_raw(), 0) };
        // SAFETY: on success the kernel returned a new descriptor that
        // nothing else owns. It is close-on-exec by definition.
        Errno::result(fd).map(|fd| unsafe { OwnedFd::from_raw_fd(fd as RawFd) })
    }
}

#[cfg(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "netbsd",
    target_os = "openbsd"
))]
mod notify {
    use super::*;
    use nix::sys::event::{EvFlags, EventFilter, FilterFlag, KEvent, Kqueue};
    use nix::sys::time::TimeSpec;

    pub fn wait_for_exit(pid: Pid, deadline: Option<Instant>) -> io::Result<Option<bool>> {
        let queue = Kqueue::new()?;
        let watch = KEvent::new(
            // A validated process ID is positive.
            pid.as_raw() as usize,
            EventFilter::EVFILT_PROC,
            EvFlags::EV_ADD | EvFlags::EV_ONESHOT,
            FilterFlag::NOTE_EXIT,
            0,
            0,
        );
        // With no room for events, this only registers the watch and reports
        // a failure to do so as an error.
        match queue.kevent(&[watch], &mut [], None) {
            Ok(_) => {}
            Err(Errno::ESRCH) => return Ok(Some(true)),
            // Another user's process may not be watched.
            Err(Errno::EPERM | Errno::EACCES) => return Ok(None),
            Err(err) => return Err(err.into()),
        }

        let mut events = [watch];
        loop {
            let timeout = deadline.map(|deadline| {
                *TimeSpec::from_duration(deadline.saturating_duration_since(Instant::now()))
                    .as_ref()
            });
            match queue.kevent(&[], &mut events, timeout) {
                Ok(0) => {
                    if deadline.is_some_and(|deadline| Instant::now() >= deadline) {
                        return Ok(Some(false));
                    }
                }
                Ok(_) => return Ok(Some(true)),
                Err(Errno::EINTR) => {}
                Err(err) => return Err(err.into()),
            }
        }
    }
}

#[cfg(not(any(
    target_os = "linux",
    target_os = "android",
    target_os = "macos",
    target_os = "ios",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "netbsd",
    target_os = "openbsd"
)))]
mod notify {
    use super::*;

    pub fn wait_for_exit(_pid: Pid, _deadline: Option<Instant>) -> io::Result<Option<bool>> {
        Ok(None)
    }
}

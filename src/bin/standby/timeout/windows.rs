//! Supervising a command on Windows.
//!
//! Windows has no signals. At the time limit standby terminates the command
//! and everything it started, whichever signal was asked for. A job object
//! holds that process tree. It is set to kill the tree when its last handle
//! closes, so the command cannot outlive standby either.

use super::{Failure, Outcome, limit};
use crate::cli::TimeoutArgs;
use crate::windows::wait_for_exit;
use standby::signals::Signal;
use std::io;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::process::{Child, Command};
use std::ptr;
use std::time::Instant;
use windows_sys::Win32::Foundation::HANDLE;
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
    SetInformationJobObject, TerminateJobObject,
};

/// Exit code given to processes that standby terminates, as `taskkill` does.
const TERMINATED: u32 = 1;

pub fn run(args: &TimeoutArgs) -> Result<i32, Failure> {
    let job = Job::new();

    let mut command = Command::new(&args.command[0]);
    command.args(&args.command[1..]);
    let mut child = command.spawn().map_err(Failure::Spawn)?;
    // Without a job, only the command itself can be terminated. A process the
    // command starts before this line runs would not join the job; a command
    // needs longer than that to load. Starting it suspended would close the
    // gap, but std exposes the thread to resume only from Rust 1.100.
    let job = job.filter(|job| job.assign(&child));

    let timed_out = match supervise(&child, args) {
        Ok(timed_out) => timed_out,
        Err(err) => {
            // Never leave the command running unsupervised.
            terminate(&mut child, job.as_ref());
            let _ = child.wait();
            return Err(err.into());
        }
    };
    if timed_out {
        if args.verbose {
            report!(
                "standby: terminating command '{}'",
                args.command[0].to_string_lossy()
            );
        }
        terminate(&mut child, job.as_ref());
    }
    let status = child.wait()?;

    if !timed_out && let Some(job) = &job {
        // The command finished by itself. Anything it left running in the
        // background stays, as it does on Unix.
        job.kill_on_close(false);
    }

    let outcome = Outcome {
        status: status.code().unwrap_or(1),
        timed_out,
        killed: args.signal == Signal::KILL,
    };
    Ok(outcome.exit_status(args.preserve_status))
}

/// Waits for the command to exit. Returns `true` if the time limit came first.
fn supervise(child: &Child, args: &TimeoutArgs) -> io::Result<bool> {
    let deadline = limit(args.duration).and_then(|limit| Instant::now().checked_add(limit));
    Ok(!wait_for_exit(child.as_raw_handle() as HANDLE, deadline)?)
}

fn terminate(child: &mut Child, job: Option<&Job>) {
    if !job.is_some_and(Job::terminate) {
        // Fails only when the command has already exited.
        let _ = child.kill();
    }
}

/// A job object: the command and every process it starts.
struct Job(OwnedHandle);

impl Job {
    /// Creates a job that kills its processes when its last handle closes.
    /// Returns `None` if the system refuses.
    fn new() -> Option<Self> {
        // SAFETY: both pointers may be null, for default security and no name.
        let handle = unsafe { CreateJobObjectW(ptr::null(), ptr::null()) };
        if handle.is_null() {
            return None;
        }
        // SAFETY: `handle` is a valid job handle that nothing else owns.
        let job = Self(unsafe { OwnedHandle::from_raw_handle(handle) });
        job.kill_on_close(true).then_some(job)
    }

    fn handle(&self) -> HANDLE {
        self.0.as_raw_handle() as HANDLE
    }

    /// Sets whether closing the last handle kills the processes in the job.
    fn kill_on_close(&self, enabled: bool) -> bool {
        let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        if enabled {
            limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        }
        // SAFETY: the handle is a valid job handle, and the pointer and size
        // describe `limits`, which is the structure this information class
        // takes and which outlives the call.
        let done = unsafe {
            SetInformationJobObject(
                self.handle(),
                JobObjectExtendedLimitInformation,
                ptr::from_ref(&limits).cast(),
                size_of_val(&limits) as u32,
            )
        };
        done != 0
    }

    /// Puts the command in the job. Processes it starts afterwards join too.
    fn assign(&self, child: &Child) -> bool {
        // SAFETY: both handles are valid for the duration of the call.
        unsafe { AssignProcessToJobObject(self.handle(), child.as_raw_handle() as HANDLE) != 0 }
    }

    /// Terminates every process in the job.
    fn terminate(&self) -> bool {
        // SAFETY: the handle is a valid job handle.
        unsafe { TerminateJobObject(self.handle(), TERMINATED) != 0 }
    }
}

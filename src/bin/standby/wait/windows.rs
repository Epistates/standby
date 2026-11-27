//! Waiting for a process on Windows, through a handle to it.

use std::io;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::time::Instant;
use windows_sys::Win32::Foundation::{ERROR_INVALID_PARAMETER, HANDLE};
use windows_sys::Win32::System::Threading::{
    GetProcessId, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE,
};

/// Waits until process `pid` has exited. Returns `false` if `deadline` comes
/// first. A process that does not exist has exited.
pub fn wait_for_exit(pid: u32, deadline: Option<Instant>) -> io::Result<bool> {
    // SAFETY: the call takes no pointers.
    let handle = unsafe {
        OpenProcess(
            PROCESS_SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION,
            0,
            pid,
        )
    };
    if handle.is_null() {
        let err = io::Error::last_os_error();
        // This is how Windows reports an ID that belongs to no process.
        return if err.raw_os_error() == Some(ERROR_INVALID_PARAMETER as i32) {
            Ok(true)
        } else {
            Err(err)
        };
    }
    // SAFETY: `handle` is a valid process handle that nothing else owns.
    let process = unsafe { OwnedHandle::from_raw_handle(handle) };
    // Windows ignores the two low bits of a process ID, so asking for 1235
    // opens process 1232. Only an exact match is the process that was meant.
    // SAFETY: the handle is valid.
    if unsafe { GetProcessId(handle) } != pid {
        return Ok(true);
    }
    crate::windows::wait_for_exit(process.as_raw_handle() as HANDLE, deadline)
}

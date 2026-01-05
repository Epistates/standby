//! Terminal state management with RAII guarantees.
//!
//! This module provides automatic terminal restoration using Rust's Drop trait,
//! ensuring that terminal state is restored even if execution is interrupted by
//! early returns, panic, or signal handlers.

use std::io::Write;

#[cfg(unix)]
use nix::sys::termios::{self, SetArg, Termios};
#[cfg(unix)]
use std::os::unix::io::{AsRawFd, BorrowedFd};

/// RAII guard for terminal attribute restoration.
///
/// This struct guarantees that terminal attributes are restored when the guard
/// is dropped, even if a function returns early or panics.
///
/// # Example
///
/// ```ignore
/// let _guard = TerminalGuard::new()?;  // Save terminal state
/// // ... perform operations that might modify terminal ...
/// // Terminal is automatically restored when _guard goes out of scope
/// ```
pub struct TerminalGuard {
    #[cfg(unix)]
    saved_attrs: Option<Termios>,
}

impl Default for TerminalGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl TerminalGuard {
    /// Create a new terminal guard that saves current attributes.
    ///
    /// Returns `None` if stdin is not a TTY (e.g., when piped/redirected).
    /// This is not an error - it simply means there's no terminal to restore.
    pub fn new() -> Self {
        #[cfg(unix)]
        {
            let saved_attrs = save_terminal_attrs();
            TerminalGuard { saved_attrs }
        }

        #[cfg(not(unix))]
        {
            TerminalGuard
        }
    }

    /// Restore terminal now, without waiting for drop.
    ///
    /// Useful if you need to ensure cleanup happens before other code runs.
    /// Subsequent drop calls will be no-ops.
    pub fn restore_now(&mut self) {
        #[cfg(unix)]
        {
            if let Some(ref attrs) = self.saved_attrs {
                restore_terminal_attrs(attrs);
                self.saved_attrs = None;
            }
        }

        restore_cursor_visibility();
    }
}

impl Drop for TerminalGuard {
    /// Automatically restore terminal when guard is dropped.
    fn drop(&mut self) {
        self.restore_now();
    }
}

/// Save terminal attributes from stdin if available (Unix only).
#[cfg(unix)]
fn save_terminal_attrs() -> Option<Termios> {
    let stdin = std::io::stdin();
    let fd = stdin.as_raw_fd();

    // Try to get terminal attributes - will fail if not a TTY
    // SAFETY: stdin remains valid for the lifetime of the program
    unsafe {
        let borrowed_fd = BorrowedFd::borrow_raw(fd);
        termios::tcgetattr(borrowed_fd).ok()
    }
}

/// Restore terminal attributes to stdin (Unix only).
#[cfg(unix)]
fn restore_terminal_attrs(attrs: &Termios) {
    let stdin = std::io::stdin();
    let fd = stdin.as_raw_fd();

    // Attempt to restore - ignore errors as terminal may have been closed
    // SAFETY: stdin remains valid for the lifetime of the program
    unsafe {
        let borrowed_fd = BorrowedFd::borrow_raw(fd);
        let _ = termios::tcsetattr(borrowed_fd, SetArg::TCSANOW, attrs);
    }
}

/// Restore cursor visibility by sending DECTCEM escape sequence (all platforms).
///
/// TUI applications often hide the cursor with `\e[?25l` (DECTCEM hide cursor).
/// When killed, they don't get to send `\e[?25h` (DECTCEM show cursor) to restore it.
/// This function explicitly sends the show cursor sequence.
///
/// This is separate from termios restoration because:
/// - termios controls terminal driver behavior (echo, canonical mode, etc.)
/// - Escape sequences control terminal emulator display (cursor, colors, etc.)
///
/// The cursor visibility state lives in the terminal emulator process, not in termios.
/// That's why `exec zsh` doesn't fix it, but opening a new terminal tab does.
///
/// This matches the behavior of `tput cnorm` and is safe to call multiple times.
fn restore_cursor_visibility() {
    // Send DECTCEM "show cursor" escape sequence: CSI ? 25 h
    // \x1b = ESC, [?25h = show cursor
    // This is idempotent - safe to send even if cursor is already visible
    let _ = std::io::stdout().write_all(b"\x1b[?25h");

    // Flush immediately to ensure the escape sequence is sent
    // Ignore errors - terminal may be closed or redirected
    let _ = std::io::stdout().flush();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_guard_creation() {
        // This should not panic even if stdin is not a TTY
        let _guard = TerminalGuard::new();
    }

    #[test]
    fn test_terminal_guard_drop() {
        // Verify that drop doesn't panic
        {
            let _guard = TerminalGuard::new();
        } // Guard is dropped here, should not panic
    }

    #[test]
    fn test_cursor_visibility_no_panic() {
        // This should not panic even if stdout is redirected
        restore_cursor_visibility();
    }
}

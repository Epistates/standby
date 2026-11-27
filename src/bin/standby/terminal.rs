//! Putting the terminal back after a command was killed in the middle of
//! using it.
//!
//! A full-screen program sets the terminal to raw mode and hides the cursor,
//! and undoes both when it exits. Killed at a time limit, it may not get the
//! chance, and the shell is left without echo or cursor.

use nix::sys::termios::{self, SetArg, Termios};
use nix::unistd;
use std::io::{self, IsTerminal, Write};

/// DECTCEM: make the cursor visible.
const SHOW_CURSOR: &[u8] = b"\x1b[?25h";

/// The terminal settings as they were before the command ran.
pub struct TerminalState(Option<Termios>);

impl TerminalState {
    /// Records the settings of the terminal on stdin, if stdin is one.
    pub fn capture() -> Self {
        Self(termios::tcgetattr(io::stdin()).ok())
    }

    /// Puts the recorded settings back and shows the cursor.
    ///
    /// Does nothing unless standby is the terminal's foreground job. From the
    /// background it would overwrite the settings of whatever runs there now.
    pub fn restore(&self) {
        let Some(settings) = &self.0 else { return };
        let stdin = io::stdin();
        if unistd::tcgetpgrp(&stdin).ok() != Some(unistd::getpgrp()) {
            return;
        }
        // Best effort: the terminal may be gone by now.
        let _ = termios::tcsetattr(&stdin, SetArg::TCSANOW, settings);
        let mut stdout = io::stdout();
        if stdout.is_terminal() {
            let _ = stdout.write_all(SHOW_CURSOR);
            let _ = stdout.flush();
        }
    }
}

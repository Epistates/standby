//! Debug and logging utilities for timeout troubleshooting.
//!
//! When verbose mode is enabled, provides detailed logging of timeout behavior
//! including process spawning, signal sending, and timing information.

use std::sync::OnceLock;

static VERBOSE_MODE: OnceLock<bool> = OnceLock::new();

/// Initialize verbose mode globally.
pub fn init_verbose(verbose: bool) {
    let _ = VERBOSE_MODE.set(verbose);
}

/// Check if verbose mode is enabled.
pub fn is_verbose() -> bool {
    VERBOSE_MODE.get().copied().unwrap_or(false)
}

/// Log a debug message if verbose mode is enabled.
///
/// Example: `debug!("Process spawned with PID: {}", pid);`
#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {
        if $crate::debug::is_verbose() {
            eprintln!("[standby] {}", format!($($arg)*));
        }
    };
}

// Make debug macro available without prefix
pub use debug;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verbose_mode_disabled() {
        init_verbose(false);
        // This should not produce output
        debug!("This should not be printed");
    }

    #[test]
    fn test_verbose_mode_enabled() {
        init_verbose(true);
        // This would print to stderr
        debug!("This should be printed if verbose");
    }
}

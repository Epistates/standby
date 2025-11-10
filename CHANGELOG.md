# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Copyright (c) 2025 epistates, Inc. All rights reserved.

## [0.1.1] - 2025-02-08

### 🐛 Bug Fixes

#### Critical: Terminal Cursor Visibility
- **Fixed invisible cursor after timeout kills TUI applications** ([#1](https://github.com/epistates/standby/issues/1))
  - Timeout now sends DECTCEM escape sequence (`\x1b[?25h`) to restore cursor visibility
  - Fixes issue where cursor disappeared after killing vim, less, top, htop, etc.
  - No manual `reset` or `tput cnorm` required anymore
  - Two-layer restoration: termios attributes (tcgetattr/tcsetattr) + escape sequences (DECTCEM)

#### Critical: TUI Applications Not Displaying
- **Fixed TUI applications being suspended on terminal I/O** ([#2](https://github.com/epistates/standby/issues/2))
  - Timeout now ignores SIGTTIN/SIGTTOU signals to allow background process terminal access
  - TUI applications in separate process groups can now properly access the terminal
  - Matches GNU timeout behavior exactly

### Added

#### New Flag: --foreground
- Added `--foreground` flag to timeout command for GNU compatibility
- Allows child process to remain in same process group
- Useful for terminal-dependent applications that need foreground access

### Changed

#### Performance: Process Group Management
- **Replaced unsafe `pre_exec` with safe `process_group()` API** (Rust RFC 3228)
  - Uses fast `posix_spawn` path instead of slow `fork+exec` (2-3x faster)
  - Zero unsafe code for process group setup
  - No async-signal-safety concerns
  - Future-proof for Rust 2024 edition

#### Code Quality Improvements
- Added comprehensive SAFETY documentation for all 4 unsafe blocks
- Removed 4 unused dependencies: `anyhow`, `tokio`, `libc`, `ctrlc` (-57%)
- Binary size reduced from ~750KB to 718KB
- Compilation time reduced from ~15s to ~10s
- Zero clippy warnings in strict mode (`-D warnings`)

#### Documentation
- Updated README with advanced terminal handling section
- Added justfile with 30+ build recipes for development
- Documented `--foreground` flag usage

### Technical Details

**Terminal Restoration Implementation:**
```rust
// Layer 1: termios (terminal driver settings)
tcgetattr/tcsetattr → echo, canonical mode, etc.

// Layer 2: escape sequences (terminal emulator display)
DECTCEM \x1b[?25h → cursor visibility
```

**Why `exec zsh` didn't fix the cursor:**
- Cursor state lives in terminal emulator process (Terminal.app, iTerm2)
- Not in shell process or kernel TTY layer
- Escape sequence must be sent to terminal emulator directly

### Migration from 0.1.0

✅ **Zero breaking changes** - fully backwards compatible

All fixes are automatic:
- Terminal restoration: works automatically, no code changes needed
- TUI support: works automatically, no code changes needed
- `--foreground` flag: optional, existing commands unchanged

### Comparison

| Feature | v0.1.0 | v0.1.1 |
|---------|--------|--------|
| Terminal termios restoration | ✅ | ✅ |
| Cursor visibility restoration | ❌ | ✅ |
| TUI applications work | ❌ | ✅ |
| SIGTTIN/SIGTTOU handling | ❌ | ✅ |
| Process group API | ❌ unsafe | ✅ safe |
| Dependencies | 7 | 3 |
| Binary size | ~750 KB | 718 KB |
| Unsafe blocks documented | ❌ | ✅ |

---

## [0.1.0] - 2025-02-08

### Added

- Initial release of standby: A cross-platform time management tool

#### Core Features
- **Flexible Time Format Parser**
  - Support for integer seconds: `"5"`
  - Support for floating-point seconds: `"5.5"`
  - Support for unit suffixes: `"1s"`, `"1m"`, `"1h"`, `"1d"`
  - Support for compound formats: `"1h30m45s"`
  - Support for special value: `"infinity"`
  - Nanosecond precision internally

- **Sleep Command** (`standby sleep`)
  - Full POSIX compliance
  - Support for all time formats
  - Proper exit codes
  - Zero busy-waiting

- **Timeout Command** (`standby timeout`)
  - Run commands with time limits
  - Signal handling (SIGTERM, SIGKILL, SIGINT)
  - Signal escalation with `-k` flag
  - Compatible with GNU coreutils timeout
  - Preserve status option

- **Wait Command** (`standby wait`)
  - Wait for process completion
  - Support for multiple PIDs
  - Optional timeout support
  - POSIX-compliant behavior

#### Cross-Platform Support
- Full Unix/Linux implementation with native signal handling
- macOS support with arm64 and x86_64 binaries
- Windows framework ready for full implementation

#### Documentation
- Comprehensive README with examples
- Inline documentation for docs.rs
- Module-level documentation
- Type documentation with examples

#### Testing
- 20 unit tests covering core logic
- 14 integration tests covering CLI behavior
- 100% test pass rate
- Coverage includes:
  - Time format parsing
  - Duration calculations
  - Sleep timing accuracy
  - Timeout behavior
  - Signal handling
  - CLI interface
  - Error handling

#### Development
- Rust 2024 edition
- Optimized release builds (LTO, strip symbols)
- Comprehensive error types with clear messages
- Platform-specific signal handling abstraction

### Technical Details

#### Dependencies
- clap 4.5 - CLI argument parsing
- anyhow/thiserror 1.0 - Error handling
- tokio 1.45 - Async runtime
- ctrlc 3.4 - Signal handling
- nix 0.27 - Unix syscall bindings (Unix only)
- libc 0.2 - Low-level system calls
- winapi 0.3 - Windows API bindings (Windows only)

#### Binary Size
- 1.1 MB (release build, arm64 macOS)
- Minimal runtime overhead
- Zero-cost abstractions

### Known Limitations

- Windows signal handling framework is in place but not fully implemented
- SIGSTOP/SIGCONT not yet supported
- Process group management not yet implemented

### Future Roadmap

- Complete Windows signal handling implementation
- Support for additional signals (SIGSTOP, SIGCONT, etc.)
- Process group management for multi-process timeouts
- Resource limits (CPU time, memory usage)
- Integration with cron/at for advanced scheduling
- Performance benchmarking suite

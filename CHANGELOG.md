# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Copyright (c) 2025 epistates, Inc. All rights reserved.

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

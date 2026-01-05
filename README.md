# Standby: Cross-Platform Time Management Tool

[![Crates.io](https://img.shields.io/crates/v/standby.svg)](https://crates.io/crates/standby)
[![Docs](https://docs.rs/standby/badge.svg)](https://docs.rs/standby)
[![License](https://img.shields.io/crates/l/standby.svg)](https://github.com/epistates/standby/blob/main/LICENSE-MIT)
[![Repository](https://img.shields.io/badge/repository-epistates%2Fstandby-blue)](https://github.com/epistates/standby)

A comprehensive, production-ready Rust CLI tool for time management across all platforms. Standby provides a unified interface for sleep, timeout, wait, and delay operations with full POSIX compliance and GNU coreutils compatibility.

## Features

- **Universal Time Format Parser**: Flexible parsing of time durations
  - Integer seconds: `5`
  - Floating-point seconds: `5.5`
  - Unit suffixes: `1s`, `1m`, `1h`, `1d`
  - Compound formats: `1h30m45s`
  - Special values: `infinity`

- **Cross-Platform Support**: Works seamlessly on Unix/Linux, macOS, and Windows
  - Native signal handling for Unix/Linux
  - Windows console handler support
  - Platform-specific optimizations

- **Three Core Commands**:
  1. **sleep** - Suspend execution for a specified duration
  2. **timeout** - Run commands with time limits and signal escalation
  3. **wait** - Wait for processes to complete with optional timeout

## Building

```bash
cargo build --release
```

The binary will be at `target/release/standby`.

## Usage

### Sleep Command
```bash
# Basic usage (5 seconds)
standby sleep 5

# Various time formats
standby sleep 5s           # 5 seconds
standby sleep 0.5s         # 500 milliseconds
standby sleep 1m           # 1 minute
standby sleep 1m30s        # 1 minute 30 seconds
standby sleep 1h           # 1 hour
standby sleep 1d           # 1 day
standby sleep infinity     # Sleep forever
```

### Timeout Command
```bash
# Run a command with a 10-second timeout
standby timeout 10 sleep 60

# Using signal options
standby timeout -s TERM 10 sleep 60     # Send SIGTERM on timeout
standby timeout -s KILL 10 sleep 60     # Send SIGKILL on timeout

# Signal escalation: send SIGTERM, then SIGKILL after 2 seconds
standby timeout -k 2s 10 sleep 60

# Preserve exit status of the command
standby timeout --preserve-status 10 sleep 60

# Run in foreground process group (like GNU timeout --foreground)
standby timeout --foreground 10 vim myfile.txt

# Enable verbose debugging (shows timing, PIDs, signal details)
standby timeout -v 10 sleep 60
standby timeout --verbose 10 sleep 60
```

**Advanced Terminal Handling:**

Standby's timeout command includes world-class terminal state restoration:
- Automatically restores terminal attributes (echo, canonical mode, etc.)
- Restores cursor visibility after killing TUI applications (vim, less, top)
- No need to run `reset` or `tput cnorm` after timeout
- Handles SIGTTIN/SIGTTOU signals for background process terminal access
- Uses safe `process_group()` API instead of unsafe `pre_exec`

**Signal Support:**

Supported signals vary by platform. Use `-s TERM`, `-s KILL`, `-s INT`, `-s STOP`, `-s CONT`, `-s TSTP`, or `-s HUP`:

| Signal | Number | Linux/macOS | Windows | Purpose |
|--------|--------|-------------|---------|---------|
| TERM   | 15     | ✅ | ⚠️ | Graceful termination request |
| KILL   | 9      | ✅ | ✅ | Forceful termination (cannot be caught) |
| INT    | 2      | ✅ | ⚠️ | Interrupt signal (Ctrl+C) |
| STOP   | 19     | ✅ | ❌ | Pause process (cannot be caught) |
| CONT   | 18     | ✅ | ❌ | Resume paused process |
| TSTP   | 20     | ✅ | ❌ | Terminal stop (like Ctrl+Z, can be caught) |
| HUP    | 1      | ✅ | ❌ | Hangup signal (terminal closed) |

**Platform Notes:**
- **Unix/Linux**: Full signal support including SIGSTOP/SIGCONT for job control
- **Windows**: Limited to KILL (via TerminateProcess); use `--kill-after 0` for immediate forceful termination
- For graceful termination on all platforms, use the default `-s TERM` and specify `--kill-after` timeout

### Wait Command
```bash
# Wait for a process to complete
standby wait 1234

# Wait for multiple processes
standby wait 1234 5678 9012

# Wait with timeout
standby wait --timeout 30s 1234
```

### Completions Command
```bash
# Generate bash completion script
standby completions bash > ~/.bash_completion.d/standby

# Generate zsh completion script
standby completions zsh > ~/.zsh/completions/_standby

# Generate fish completion script
standby completions fish > ~/.config/fish/completions/standby.fish

# Install bash completions (Ubuntu/Debian)
standby completions bash | sudo tee /etc/bash_completion.d/standby

# View bash completion script
standby completions bash
```

**Completion Features:**
- Bash: Full subcommand and signal completion
- Zsh: Option and subcommand completion with descriptions
- Fish: Comprehensive descriptions for all options and signals

## Testing

Run all tests (unit + integration):
```bash
cargo test
```

Run only unit tests:
```bash
cargo test --lib
```

Run only integration tests:
```bash
cargo test --test integration_tests
```

## Project Structure

```
src/
├── main.rs              # CLI entry point
├── lib.rs               # Library exports
├── errors.rs            # Error types
├── terminal.rs          # Terminal state management (RAII guard)
├── commands/
│   ├── mod.rs          # Command CLI setup
│   ├── sleep.rs        # Sleep subcommand
│   ├── timeout.rs      # Timeout subcommand
│   └── wait.rs         # Wait subcommand
├── time/
│   ├── mod.rs          # Time module exports
│   ├── parser.rs       # Duration format parser
│   └── duration.rs     # Duration type
└── signals/
    ├── mod.rs          # Signal handler abstraction
    ├── unix.rs         # Unix/Linux signal handling (7 signals)
    └── windows.rs      # Windows signal handling (TerminateProcess)

tests/
└── integration_tests.rs # End-to-end tests
```

## POSIX Compliance

Standby is designed with POSIX compliance as the baseline, with extensions for GNU coreutils compatibility:

- ✅ POSIX sleep with integer seconds
- ✅ GNU extensions: floating-point, unit suffixes
- ✅ Signal handling per POSIX specification
- ✅ Process timeout implementation
- ✅ Cross-platform compatibility

## Test Coverage

- 20 unit tests covering:
  - Time format parsing (integer, float, suffixes, compound)
  - Duration calculations
  - Special values (infinity)

- 14 integration tests covering:
  - Sleep with various time formats
  - Timeout with process termination
  - Signal handling
  - CLI help and version
  - Error handling

## Exit Codes

- `0`: Successful completion
- `1`: Timeout occurred or command failed
- Other codes: Exit code from the executed command

## Dependencies

- **clap** - CLI argument parsing with derive macros
- **thiserror** - Error handling
- **nix** - Unix syscall bindings (Unix only)
- **winapi** - Windows API bindings (Windows only)

Note: Minimal dependency footprint with no async runtime or external signal handling libraries for synchronous CLI operations.

## Performance

Standby is compiled to native binaries with optimizations enabled, providing near-native performance with minimal overhead.

## Features & Timeline

### Completed (v0.1.2)
- [x] Support for additional signals (SIGSTOP, SIGCONT, SIGTSTP, SIGHUP)
- [x] Windows TerminateProcess support for SIGKILL
- [x] RAII terminal guard pattern for guaranteed restoration

### Completed (v0.2.0)
- [x] Linux timerfd support for nanosecond-precision timeouts (~1μs latency)
- [x] macOS adaptive polling for efficient timeout handling (1-10ms latency)
- [x] Shell completion scripts for bash, zsh, and fish
- [x] Optional verbose mode (`-v/--verbose`) for timeout troubleshooting
- [x] Debug logging with timing information and signal details

### Future Enhancements
- [ ] True kqueue integration for macOS (currently uses adaptive polling)
- [ ] Real-time timer precision improvements (sub-microsecond)
- [ ] Integration with system schedulers (cron, at)
- [ ] Resource limits (CPU time, memory usage tracking)

## License

MIT or Apache 2.0 (standard Rust project licenses)

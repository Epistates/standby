# standby

[![Crates.io](https://img.shields.io/crates/v/standby.svg)](https://crates.io/crates/standby)
[![Docs](https://docs.rs/standby/badge.svg)](https://docs.rs/standby)
[![CI](https://github.com/epistates/standby/actions/workflows/ci.yml/badge.svg)](https://github.com/epistates/standby/actions/workflows/ci.yml)
[![License](https://img.shields.io/crates/l/standby.svg)](#license)

`sleep`, `timeout` and `wait` in one binary that behaves the same on Linux, macOS and Windows.

```bash
standby sleep 1m30s                  # pause
standby timeout 10m cargo test       # stop a command that runs too long
standby wait --timeout 1h 4242       # block until process 4242 has exited
```

- **`timeout` stops the whole process tree.** The command runs in its own process group (a job object on Windows), so the build tool's compilers and the dev server's workers stop with it.
- **`timeout` is a drop-in for GNU `timeout`.** It has the same options and the same exit statuses: 124, 125, 126, 127 and 137.
- **`wait` works on any process**, not only children of your shell, and the kernel reports the exit: nothing is polled.
- **Durations read the way you write them:** `90`, `2.5s`, `1h30m`, `500ms`, `infinity`.

## Install

```bash
cargo install standby
```

This needs Rust 1.88 or later. To build from a checkout, run `cargo build --release`; the binary is `target/release/standby`.

## Durations

A duration is a number of seconds, or one or more numbers each followed by a unit.

| Unit | Meaning |
|------|---------|
| `ns` | nanoseconds |
| `us`, `µs` | microseconds |
| `ms` | milliseconds |
| `s` | seconds |
| `m` | minutes |
| `h` | hours |
| `d` | days |

Numbers may have a fraction (`0.5`, `1.5h`) or an exponent (`1e3`). Segments add up in any order: `1h30m45s`, `"1m 30s"`. `infinity` and `inf` mean forever.

Values are exact to the nanosecond: `0.1` is exactly 100 ms. Anything smaller than a nanosecond rounds up, so a positive duration never becomes zero.

## sleep

```bash
standby sleep 5            # 5 seconds
standby sleep 250ms
standby sleep 1m 30s       # several durations add up
standby sleep infinity     # until interrupted
```

## timeout

```
standby timeout [OPTIONS] <DURATION> <COMMAND>...
```

```bash
standby timeout 30 make test                  # TERM after 30 seconds
standby timeout -k 5 30 npm run dev           # then KILL whatever is left 5 seconds later
standby timeout -s INT 10m ./long-job
standby timeout --foreground 60 vim notes.md  # an interactive program
```

Everything after the command belongs to the command, so `standby timeout 5 grep -s pattern file` passes `-s` to `grep`.

| Option | Meaning |
|--------|---------|
| `-s`, `--signal <SIGNAL>` | Signal to send at the time limit, by name (`TERM`, `SIGINT`) or number. Default `TERM`. |
| `-k`, `--kill-after <DURATION>` | Send `KILL` to whatever the command started that is still running this long after the first signal, and wait until none of it is left. |
| `-p`, `--preserve-status` | Exit with the command's own status even when the time limit is reached. |
| `-f`, `--foreground` | Leave the command in standby's process group so that it can read from the terminal. Its children are then not stopped at the time limit. |
| `-v`, `--verbose` | Report on stderr each signal that is sent. |

A duration of `0` or `infinity` means no limit.

### Exit status

| Status | Meaning |
|--------|---------|
| 124 | The time limit was reached (unless `--preserve-status`). |
| 125 | standby itself failed, for example on a bad option. |
| 126 | The command was found but could not be run. |
| 127 | The command was not found. |
| 137 | The command was sent `KILL`. |
| other | The status of the command, or 128+N if signal N ended it. |

### What happens at the time limit

On Linux, macOS and the BSDs:

1. The signal goes to the command's process group: the command and everything it started that has not moved to a group of its own.
2. With `--kill-after`, anything in the group that is still running after that long is sent `KILL`, and standby returns only once the group is empty. Without it, standby returns as soon as the command itself exits.
3. If the command used the terminal and was killed, standby restores the terminal settings and the cursor.

While the command runs, standby passes on the signals it receives: `HUP`, `INT`, `QUIT`, `TERM`, `USR1` and `USR2`. Ctrl-C therefore stops the command whether standby was started from a terminal or from a script, and `kill <standby pid>` does not leave the command behind. Ctrl-Z suspends the command along with standby, and `fg` resumes both. If a signal ends the command before the time limit, standby ends the same way, so that a calling shell sees the signal.

`KILL` and `STOP` cannot be caught. If standby itself is killed with `KILL`, the command keeps running.

The command does not run in the terminal's foreground process group, so it cannot read from the terminal. Use `--foreground` for interactive programs.

On Windows there are no signals. At the time limit standby terminates the command and every process it started, whichever signal was asked for; `--kill-after` and `--foreground` have no effect. If standby itself is terminated, Windows terminates the command's process tree with it. The status is 124, or 137 with `-s KILL`.

### Differences from GNU `timeout`

- GNU `timeout` puts itself in a new process group. Started from a script, it then never sees Ctrl-C, and the command runs on. standby stays in the caller's group and passes the signal on.
- With `--kill-after`, standby waits for the command's whole process group, not only for the command.
- standby accepts more duration formats (`1m30s`, `500ms`) and restores the terminal after killing a full-screen program.
- `--verbose` prints `standby: sending signal TERM to command 'sleep'`.

## wait

```
standby wait [--timeout <DURATION>] <PID>...
```

```bash
standby wait 4242                    # until process 4242 has exited
standby wait 4242 4243 4244          # until all three have
standby wait --timeout 30m 4242      # give up after 30 minutes
standby wait --timeout 0 4242        # check once: 0 if it has exited, 124 if not
```

A process ID that does not exist counts as already exited. The exit status of the process is not available: only its parent can collect that.

| Status | Meaning |
|--------|---------|
| 0 | Every process has exited. |
| 124 | The timeout was reached first. |
| 1 | A process could not be watched. |

`wait` uses pidfd on Linux, kqueue on macOS and the BSDs, and a process handle on Windows. Where the kernel will not report the exit (Linux before 5.3, or another user's process on macOS), it probes at intervals of up to 100 ms instead.

## Shell completions

```bash
standby completions bash > ~/.local/share/bash-completion/completions/standby
standby completions zsh > ~/.zfunc/_standby
standby completions fish > ~/.config/fish/completions/standby.fish
```

`elvish` and `powershell` are supported too.

## Library

The crate also exposes the duration and signal parsers:

```rust
use standby::time::{Duration, parse_duration};

let limit = parse_duration("1m30s").unwrap();
assert_eq!(limit.to_std(), Some(std::time::Duration::from_secs(90)));
assert_eq!("infinity".parse::<Duration>().unwrap(), Duration::Infinite);
```

See [docs.rs/standby](https://docs.rs/standby).

## Development

```bash
just check    # format, lint, test and docs, as CI runs them
just deny     # dependency licenses, sources and advisories
```

Without [just](https://github.com/casey/just): `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT) at your option.

# Contributing

Bug reports and pull requests are welcome. For a security problem, follow [SECURITY.md](SECURITY.md) instead of opening a public issue.

## Before you open a pull request

Run the checks CI runs:

```bash
just check    # cargo fmt --check, clippy with warnings as errors, tests, rustdoc
just deny     # licenses, sources and advisories (needs cargo-deny)
```

Without [just](https://github.com/casey/just), run `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test --locked` and `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --locked`.

CI also tests on Linux (x86-64 and arm64), macOS and Windows, and builds with the minimum supported Rust version in `Cargo.toml`. If you change Unix-only or Windows-only code, check the other platform with `just clippy-cross` (it needs the rustup targets).

## What a change needs

- **A test.** Behaviour of the command-line tool is tested in `tests/cli.rs`, which runs the real binary. Parsing is tested next to the code.
- **Documentation.** Update the README, the `--help` text in `src/bin/standby/cli.rs` and the rustdoc when behaviour changes. Every public item has a doc comment.
- **A changelog entry** under `Unreleased` in `CHANGELOG.md`. Mark a change to an existing exit status, option or default as **Breaking**.
- **`unsafe` with a reason.** Each `unsafe` block carries a `SAFETY:` comment saying why it is sound. Clippy enforces this.

## Design rules

- `timeout` matches GNU `timeout` unless the README's "Differences from GNU `timeout`" says otherwise. Raise a new difference in the pull request.
- The library exposes only what is useful without the binary: duration and signal parsing. Process handling stays in the binary.
- Dependencies stay few. A new one needs a reason, and `cargo deny` must pass.

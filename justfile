# https://github.com/casey/just

# List the recipes
default:
    @just --list

# Everything CI checks: format, lint, test and docs
check: fmt-check clippy test doc

# Format the code
fmt:
    cargo fmt

# Fail if the code is not formatted
fmt-check:
    cargo fmt --check

# Lint, with warnings as errors
clippy:
    cargo clippy --all-targets --locked -- -D warnings

# Run unit, integration and doc tests
test:
    cargo test --locked

# Build the API docs, with warnings as errors
doc:
    RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --locked

# Lint for the platforms that cannot be tested from this one (needs the rustup targets)
clippy-cross:
    cargo clippy --all-targets --locked --target x86_64-unknown-linux-gnu -- -D warnings
    cargo clippy --all-targets --locked --target x86_64-pc-windows-msvc -- -D warnings

# Check dependency licenses, sources and advisories (needs cargo-deny)
deny:
    cargo deny check

# Build the optimized binary at target/release/standby
release:
    cargo build --release --locked

# Install the binary into ~/.cargo/bin
install:
    cargo install --path . --locked

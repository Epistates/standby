# Justfile for standby - World-class time management tool
# https://github.com/casey/just

# Default recipe (runs when you type 'just')
default:
    @just --list

# Build the project in debug mode
build:
    cargo build

# Build the project in release mode with optimizations
build-release:
    cargo build --release

# Alias for release build
br: build-release

# Run all tests (unit + integration + doc tests)
test:
    cargo test

# Run tests in release mode (faster execution)
test-release:
    cargo test --release

# Run only unit tests
test-unit:
    cargo test --lib

# Run only integration tests
test-integration:
    cargo test --test integration_tests

# Run clippy for linting
clippy:
    cargo clippy -- -D warnings

# Check code formatting
fmt-check:
    cargo fmt --all -- --check

# Format code
fmt:
    cargo fmt --all

# Run all checks (clippy + fmt + tests)
check: clippy fmt-check test

# Clean build artifacts
clean:
    cargo clean

# Install the binary to ~/.cargo/bin (or system default)
install:
    cargo install --path .

# Install to a custom location
install-to path:
    cargo install --path . --root {{path}}

# Uninstall the binary
uninstall:
    cargo uninstall standby

# Build documentation
doc:
    cargo doc --no-deps --open

# Build documentation including private items
doc-all:
    cargo doc --no-deps --document-private-items --open

# Run the binary (debug mode)
run *ARGS:
    cargo run -- {{ARGS}}

# Run the binary (release mode)
run-release *ARGS:
    cargo run --release -- {{ARGS}}

# Quick test: sleep for 1 second
test-sleep:
    ./target/release/standby sleep 1

# Quick test: timeout with terminal restoration
test-timeout:
    ./target/release/standby timeout 2 less /etc/passwd || echo "Timeout test complete - check if terminal is OK"

# Build and run a specific example
example NAME *ARGS:
    cargo run --release -- {{NAME}} {{ARGS}}

# Package the crate for publishing
package:
    cargo package --allow-dirty

# Publish to crates.io (dry-run)
publish-dry:
    cargo publish --dry-run

# Publish to crates.io
publish:
    cargo publish

# Update dependencies
update:
    cargo update

# Check for outdated dependencies
outdated:
    cargo outdated

# Run cargo-audit for security vulnerabilities
audit:
    cargo audit

# Build for all targets (if cross-compilation is set up)
build-all:
    cargo build --release --target x86_64-unknown-linux-gnu
    cargo build --release --target x86_64-apple-darwin
    cargo build --release --target aarch64-apple-darwin
    cargo build --release --target x86_64-pc-windows-gnu

# Profile the binary with perf (Linux only)
profile COMMAND:
    cargo build --release
    perf record -g ./target/release/standby {{COMMAND}}
    perf report

# Watch for changes and run tests
watch:
    cargo watch -x test

# Watch for changes and run checks
watch-check:
    cargo watch -x check -x clippy

# Install the binary and run a test
install-and-test: install
    standby sleep 0.5
    @echo "Installation successful!"

# Full CI pipeline (what CI would run)
ci: clippy fmt-check test doc
    @echo "✓ All CI checks passed!"

# Development setup - install tools
dev-setup:
    rustup component add clippy rustfmt
    cargo install cargo-watch cargo-outdated cargo-audit

# Benchmark (placeholder - you can add proper benchmarks later)
bench:
    @echo "Running quick benchmarks..."
    time ./target/release/standby sleep 0.1
    @echo "\nRunning 20 timeout spawns..."
    @time for i in {1..20}; do ./target/release/standby timeout 5 echo "test" > /dev/null 2>&1; done

# Show version info
version:
    @cargo pkgid | cut -d# -f2

# Show help for all commands
help:
    @echo "Standby Build System"
    @echo "===================="
    @echo ""
    @echo "Common commands:"
    @echo "  just build          - Build debug version"
    @echo "  just br             - Build release version"
    @echo "  just test           - Run all tests"
    @echo "  just check          - Run all checks (clippy + fmt + tests)"
    @echo "  just install        - Install binary to ~/.cargo/bin"
    @echo "  just clean          - Clean build artifacts"
    @echo "  just ci             - Run full CI pipeline"
    @echo ""
    @echo "Run 'just --list' for all available commands"

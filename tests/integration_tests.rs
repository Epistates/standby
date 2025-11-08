use std::process::Command;
use std::time::Instant;

#[test]
fn test_sleep_basic() {
    let start = Instant::now();
    let output = Command::new("./target/release/standby")
        .args(&["sleep", "1"])
        .output()
        .expect("Failed to run standby sleep");

    let elapsed = start.elapsed();

    assert!(output.status.success());
    assert!(elapsed.as_millis() >= 900, "Sleep was too fast: {:?}", elapsed);
    assert!(elapsed.as_millis() <= 1500, "Sleep was too slow: {:?}", elapsed);
}

#[test]
fn test_sleep_float_seconds() {
    let start = Instant::now();
    let output = Command::new("./target/release/standby")
        .args(&["sleep", "0.5"])
        .output()
        .expect("Failed to run standby sleep");

    let elapsed = start.elapsed();

    assert!(output.status.success());
    assert!(elapsed.as_millis() >= 400, "Sleep was too fast: {:?}", elapsed);
    assert!(elapsed.as_millis() <= 1000, "Sleep was too slow: {:?}", elapsed);
}

#[test]
fn test_sleep_with_suffix() {
    let start = Instant::now();
    let output = Command::new("./target/release/standby")
        .args(&["sleep", "1s"])
        .output()
        .expect("Failed to run standby sleep");

    let elapsed = start.elapsed();

    assert!(output.status.success());
    assert!(elapsed.as_millis() >= 950);
}

#[test]
fn test_sleep_minutes() {
    let start = Instant::now();
    let output = Command::new("./target/release/standby")
        .args(&["sleep", "0.5m"])
        .output()
        .expect("Failed to run standby sleep");

    let elapsed = start.elapsed();

    assert!(output.status.success());
    // 0.5m = 30 seconds
    assert!(elapsed.as_secs() >= 29, "Sleep was too fast");
}

#[test]
fn test_timeout_kills_process() {
    let start = Instant::now();
    let output = Command::new("./target/release/standby")
        .args(&["timeout", "1", "sleep", "10"])
        .output()
        .expect("Failed to run standby timeout");

    let elapsed = start.elapsed();

    // Should fail because the process was killed
    assert!(!output.status.success());

    // Should take roughly 1 second (plus signal time)
    assert!(elapsed.as_millis() >= 900);
    assert!(elapsed.as_millis() <= 2000, "Timeout took too long: {:?}", elapsed);
}

#[test]
fn test_timeout_with_float_duration() {
    let start = Instant::now();
    let output = Command::new("./target/release/standby")
        .args(&["timeout", "0.5", "sleep", "5"])
        .output()
        .expect("Failed to run standby timeout");

    let elapsed = start.elapsed();

    assert!(!output.status.success());
    assert!(elapsed.as_millis() >= 400);
    assert!(elapsed.as_millis() <= 1500);
}

#[test]
fn test_timeout_success_when_command_completes() {
    let output = Command::new("./target/release/standby")
        .args(&["timeout", "5", "sleep", "1"])
        .output()
        .expect("Failed to run standby timeout");

    // Should succeed because sleep completes before timeout
    assert!(output.status.success() || output.status.code() == Some(0));
}

#[test]
fn test_timeout_with_signal_option() {
    let start = Instant::now();
    let output = Command::new("./target/release/standby")
        .args(&["timeout", "-s", "TERM", "1", "sleep", "10"])
        .output()
        .expect("Failed to run standby timeout");

    let elapsed = start.elapsed();

    assert!(!output.status.success());
    assert!(elapsed.as_millis() >= 900);
    assert!(elapsed.as_millis() <= 2000);
}

#[test]
fn test_cli_help_main() {
    let output = Command::new("./target/release/standby")
        .arg("--help")
        .output()
        .expect("Failed to run standby help");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("sleep"));
    assert!(stdout.contains("timeout"));
    assert!(stdout.contains("wait"));
}

#[test]
fn test_cli_version() {
    let output = Command::new("./target/release/standby")
        .arg("--version")
        .output()
        .expect("Failed to run standby version");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("standby"));
}

#[test]
fn test_invalid_time_format() {
    let output = Command::new("./target/release/standby")
        .args(&["sleep", "invalid"])
        .output()
        .expect("Failed to run standby");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Error"));
}

#[test]
fn test_sleep_help() {
    let output = Command::new("./target/release/standby")
        .args(&["sleep", "--help"])
        .output()
        .expect("Failed to run sleep help");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("DURATION"));
}

#[test]
fn test_timeout_help() {
    let output = Command::new("./target/release/standby")
        .args(&["timeout", "--help"])
        .output()
        .expect("Failed to run timeout help");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("DURATION"));
    assert!(stdout.contains("COMMAND"));
}

#[test]
fn test_timeout_with_command_args() {
    let output = Command::new("./target/release/standby")
        .args(&["timeout", "5", "echo", "hello", "world"])
        .output()
        .expect("Failed to run standby timeout with echo");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("hello") || stdout.contains("world"));
}

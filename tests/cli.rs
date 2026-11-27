//! End-to-end tests of the `standby` binary.
//!
//! Where a test needs a long-running command, it uses `standby sleep`, which
//! exists on every platform these tests run on.

use std::process::{Child, Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const STANDBY: &str = env!("CARGO_BIN_EXE_standby");

/// Far longer than any test waits; a command that runs this long was not stopped.
const FOREVER: &str = "600";

/// Upper bound for things that should happen promptly. Generous, because CI
/// machines stall.
const PROMPTLY: Duration = Duration::from_secs(20);

fn standby(args: &[&str]) -> Command {
    let mut command = Command::new(STANDBY);
    command.args(args).stdin(Stdio::null());
    command
}

fn run(args: &[&str]) -> Output {
    standby(args).output().expect("standby should start")
}

fn timed(args: &[&str]) -> (Output, Duration) {
    let start = Instant::now();
    let output = run(args);
    (output, start.elapsed())
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// A process that is killed when the test is done with it, pass or fail.
struct Scoped(Child);

impl Scoped {
    fn spawn(command: &mut Command) -> Self {
        Self(command.spawn().expect("process should start"))
    }

    fn pid(&self) -> String {
        self.0.id().to_string()
    }
}

impl Drop for Scoped {
    fn drop(&mut self) {
        // Ask politely first: a `standby timeout` that is killed outright
        // cannot stop its command, which would outlive the test.
        #[cfg(unix)]
        {
            for signal in ["-TERM", "-CONT"] {
                let _ = Command::new("kill")
                    .args([signal, &self.pid()])
                    .stderr(Stdio::null())
                    .status();
            }
            let start = Instant::now();
            while start.elapsed() < Duration::from_secs(5) {
                if matches!(self.0.try_wait(), Ok(Some(_))) {
                    return;
                }
                thread::sleep(Duration::from_millis(10));
            }
        }
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

mod sleep {
    use super::*;

    #[test]
    fn pauses_for_the_duration() {
        let (output, elapsed) = timed(&["sleep", "0.3"]);
        assert_eq!(output.status.code(), Some(0));
        assert!(elapsed >= Duration::from_millis(300), "{elapsed:?}");
    }

    #[test]
    fn adds_several_durations() {
        let (output, elapsed) = timed(&["sleep", "0.2", "100ms"]);
        assert_eq!(output.status.code(), Some(0));
        assert!(elapsed >= Duration::from_millis(300), "{elapsed:?}");
    }

    /// 0.2.0 slept five minutes here.
    #[test]
    fn reads_milliseconds_as_milliseconds() {
        let (output, elapsed) = timed(&["sleep", "5ms"]);
        assert_eq!(output.status.code(), Some(0));
        assert!(elapsed < PROMPTLY, "{elapsed:?}");
    }

    #[test]
    fn rejects_a_bad_duration_with_status_1() {
        let output = run(&["sleep", "5x"]);
        assert_eq!(output.status.code(), Some(1));
        assert!(
            stderr(&output).contains("unknown unit 'x'"),
            "{}",
            stderr(&output)
        );

        assert_eq!(run(&["sleep"]).status.code(), Some(1));
        assert_eq!(run(&["sleep", "-1"]).status.code(), Some(1));
    }
}

mod timeout {
    use super::*;

    #[test]
    fn a_command_that_finishes_in_time_keeps_its_status() {
        let output = run(&["timeout", "60", STANDBY, "sleep", "0.1"]);
        assert_eq!(output.status.code(), Some(0));

        #[cfg(unix)]
        let output = run(&["timeout", "60", "sh", "-c", "exit 3"]);
        #[cfg(windows)]
        let output = run(&["timeout", "60", "cmd", "/C", "exit 3"]);
        assert_eq!(output.status.code(), Some(3));
    }

    #[test]
    fn stops_the_command_at_the_limit_with_status_124() {
        let (output, elapsed) = timed(&["timeout", "0.3", STANDBY, "sleep", FOREVER]);
        assert_eq!(output.status.code(), Some(124));
        assert!(elapsed >= Duration::from_millis(300), "{elapsed:?}");
        assert!(elapsed < PROMPTLY, "{elapsed:?}");
    }

    #[test]
    fn says_nothing_unless_verbose() {
        let output = run(&["timeout", "0.2", STANDBY, "sleep", FOREVER]);
        assert_eq!(stderr(&output), "");
        assert_eq!(stdout(&output), "");

        let output = run(&["timeout", "--verbose", "0.2", STANDBY, "sleep", FOREVER]);
        assert_eq!(output.status.code(), Some(124));
        let expected = if cfg!(unix) {
            "standby: sending signal TERM to command"
        } else {
            "standby: terminating command"
        };
        assert!(stderr(&output).contains(expected), "{}", stderr(&output));
    }

    #[test]
    fn the_kill_signal_reports_status_137() {
        let (output, elapsed) = timed(&["timeout", "-s", "KILL", "0.3", STANDBY, "sleep", FOREVER]);
        assert_eq!(output.status.code(), Some(137));
        assert!(elapsed < PROMPTLY, "{elapsed:?}");
    }

    #[test]
    fn zero_and_infinity_set_no_limit() {
        for limit in ["0", "infinity"] {
            let output = run(&["timeout", limit, STANDBY, "sleep", "0.3"]);
            assert_eq!(output.status.code(), Some(0), "{limit}");
        }
    }

    #[test]
    fn a_missing_command_reports_status_127() {
        let output = run(&["timeout", "5", "standby-test-no-such-command"]);
        assert_eq!(output.status.code(), Some(127));
        assert!(
            stderr(&output).contains("failed to run command 'standby-test-no-such-command'"),
            "{}",
            stderr(&output)
        );
    }

    #[test]
    fn a_usage_error_reports_status_125() {
        for args in [
            &["timeout", "5x", "true"][..],
            &["timeout", "-s", "NOPE", "5", "true"],
            &["timeout", "-k", "soon", "5", "true"],
            &["timeout", "5"],
            &["timeout"],
            &["timeout", "--no-such-option", "5", "true"],
        ] {
            let output = run(args);
            assert_eq!(output.status.code(), Some(125), "{args:?}");
            assert!(!stderr(&output).is_empty(), "{args:?}");
        }
    }

    /// 0.2.0 took the `-s` in `timeout 5 grep -s x` for its own.
    #[test]
    fn options_after_the_command_reach_the_command() {
        let output = run(&["timeout", "60", STANDBY, "sleep", "--help"]);
        assert_eq!(output.status.code(), Some(0));
        assert!(
            stdout(&output).contains("Usage: standby sleep"),
            "{}",
            stdout(&output)
        );
    }

    #[test]
    fn help_is_not_an_error() {
        let output = run(&["timeout", "--help"]);
        assert_eq!(output.status.code(), Some(0));
        assert!(
            stdout(&output).contains("Exit status:"),
            "{}",
            stdout(&output)
        );
    }
}

mod wait {
    use super::*;

    fn sleeper(duration: &str) -> Scoped {
        Scoped::spawn(&mut standby(&["sleep", duration]))
    }

    /// The ID of a process that has exited and been reaped.
    fn exited_pid() -> String {
        let mut child = standby(&["sleep", "0"])
            .spawn()
            .expect("standby should start");
        child.wait().expect("child should exit");
        child.id().to_string()
    }

    /// 0.2.0 returned at once for any process it had not started itself.
    #[test]
    fn returns_once_the_process_exits() {
        let mut child = standby(&["sleep", "0.5"])
            .spawn()
            .expect("standby should start");
        let pid = child.id().to_string();
        let start = Instant::now();
        // Reaped in the background, as its real parent would.
        let reaper = thread::spawn(move || child.wait());

        let output = run(&["wait", &pid]);
        let elapsed = start.elapsed();
        assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
        assert!(elapsed >= Duration::from_millis(400), "{elapsed:?}");
        assert!(elapsed < PROMPTLY, "{elapsed:?}");
        reaper.join().unwrap().unwrap();
    }

    #[test]
    fn waits_for_every_process() {
        let mut quick = standby(&["sleep", "0.1"])
            .spawn()
            .expect("standby should start");
        let mut slow = standby(&["sleep", "0.6"])
            .spawn()
            .expect("standby should start");
        let (quick_pid, slow_pid) = (quick.id().to_string(), slow.id().to_string());
        let start = Instant::now();
        let reaper = thread::spawn(move || (quick.wait(), slow.wait()));

        // The slow one first, so the quick one has exited by its turn.
        let output = run(&["wait", &slow_pid, &quick_pid, &exited_pid()]);
        assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
        assert!(
            start.elapsed() >= Duration::from_millis(500),
            "{:?}",
            start.elapsed()
        );
        let (quick, slow) = reaper.join().unwrap();
        quick.unwrap();
        slow.unwrap();
    }

    #[test]
    fn a_process_that_does_not_exist_has_exited() {
        let (output, elapsed) = timed(&["wait", &exited_pid()]);
        assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
        assert!(elapsed < PROMPTLY, "{elapsed:?}");
    }

    #[test]
    fn gives_up_at_the_timeout_with_status_124() {
        let sleeper = sleeper(FOREVER);
        let (output, elapsed) = timed(&["wait", "--timeout", "0.3", &sleeper.pid()]);
        assert_eq!(output.status.code(), Some(124));
        assert!(
            stderr(&output).contains("timed out waiting for process"),
            "{}",
            stderr(&output)
        );
        assert!(elapsed >= Duration::from_millis(300), "{elapsed:?}");
        assert!(elapsed < PROMPTLY, "{elapsed:?}");
    }

    #[test]
    fn a_zero_timeout_checks_once() {
        let sleeper = sleeper(FOREVER);
        let (output, elapsed) = timed(&["wait", "-t", "0", &sleeper.pid()]);
        assert_eq!(output.status.code(), Some(124));
        assert!(elapsed < PROMPTLY, "{elapsed:?}");

        assert_eq!(
            run(&["wait", "-t", "0", &exited_pid()]).status.code(),
            Some(0)
        );
    }

    #[test]
    fn an_infinite_timeout_is_no_timeout() {
        let mut child = standby(&["sleep", "0.3"])
            .spawn()
            .expect("standby should start");
        let pid = child.id().to_string();
        let reaper = thread::spawn(move || child.wait());
        assert_eq!(
            run(&["wait", "-t", "infinity", &pid]).status.code(),
            Some(0)
        );
        reaper.join().unwrap().unwrap();
    }

    #[test]
    fn a_usage_error_reports_status_2() {
        for args in [
            &["wait"][..],
            &["wait", "0"],
            &["wait", "abc"],
            &["wait", "-t", "x", "1"],
        ] {
            assert_eq!(run(args).status.code(), Some(2), "{args:?}");
        }
    }
}

mod general {
    use super::*;

    #[test]
    fn prints_its_version() {
        let output = run(&["--version"]);
        assert_eq!(output.status.code(), Some(0));
        assert_eq!(
            stdout(&output).trim(),
            concat!("standby ", env!("CARGO_PKG_VERSION"))
        );
    }

    #[test]
    fn help_lists_every_subcommand() {
        let output = run(&["--help"]);
        assert_eq!(output.status.code(), Some(0));
        for subcommand in ["sleep", "timeout", "wait", "completions"] {
            assert!(stdout(&output).contains(subcommand), "{subcommand}");
        }
    }

    #[test]
    fn no_subcommand_is_a_usage_error() {
        assert_eq!(run(&[]).status.code(), Some(2));
        assert_eq!(run(&["nonsense"]).status.code(), Some(2));
    }

    #[test]
    fn generates_completions_for_every_shell() {
        for shell in ["bash", "zsh", "fish", "elvish", "powershell"] {
            let output = run(&["completions", shell]);
            assert_eq!(output.status.code(), Some(0), "{shell}");
            let script = stdout(&output);
            assert!(script.contains("standby"), "{shell}");
            assert!(script.contains("kill-after"), "{shell}");
        }
        assert_eq!(run(&["completions", "nosuchshell"]).status.code(), Some(2));
    }
}

/// Signals, process groups and exit statuses that only Unix has.
#[cfg(unix)]
mod unix {
    use super::*;
    use std::io::{BufRead, BufReader};
    use std::os::unix::fs::PermissionsExt;
    use std::os::unix::process::ExitStatusExt;
    use std::process::ChildStdout;
    use std::sync::mpsc;

    fn send(signal: &str, pid: &str) {
        let status = Command::new("kill")
            .args([&format!("-{signal}"), pid])
            .status()
            .expect("kill should run");
        assert!(status.success(), "kill -{signal} {pid}");
    }

    fn is_alive(pid: &str) -> bool {
        Command::new("kill")
            .args(["-0", pid])
            .stderr(Stdio::null())
            .status()
            .expect("kill should run")
            .success()
    }

    fn eventually(what: &str, condition: impl Fn() -> bool) {
        let start = Instant::now();
        while !condition() {
            assert!(start.elapsed() < PROMPTLY, "timed out waiting until {what}");
            thread::sleep(Duration::from_millis(20));
        }
    }

    /// Starts `standby timeout` over a shell script and returns it with the
    /// first line the script prints, which shows that the script is running,
    /// and with the rest of the script's output.
    fn start(options: &[&str], script: &str) -> (Scoped, String, BufReader<ChildStdout>) {
        let mut command = standby(&["timeout"]);
        command
            .args(options)
            .args(["sh", "-c", script])
            .stdout(Stdio::piped());
        let mut scoped = Scoped::spawn(&mut command);
        let mut output = BufReader::new(scoped.0.stdout.take().unwrap());
        let mut line = String::new();
        output
            .read_line(&mut line)
            .expect("script should print a line");
        (scoped, line.trim().to_string(), output)
    }

    #[test]
    fn preserve_status_reports_the_signal_that_ended_the_command() {
        let output = run(&["timeout", "--preserve-status", "0.3", "sleep", FOREVER]);
        assert_eq!(output.status.code(), Some(128 + 15));

        let output = run(&["timeout", "-p", "-s", "INT", "0.3", "sleep", FOREVER]);
        assert_eq!(output.status.code(), Some(128 + 2));
    }

    #[test]
    fn a_command_that_exits_cleanly_on_the_signal_still_timed_out() {
        let script = "trap 'exit 0' TERM; sleep 600 & wait";
        let output = run(&["timeout", "0.3", "sh", "-c", script]);
        assert_eq!(output.status.code(), Some(124));

        let output = run(&["timeout", "-p", "0.3", "sh", "-c", script]);
        assert_eq!(output.status.code(), Some(0));
    }

    #[test]
    fn kill_after_escalates_when_the_signal_is_ignored() {
        let script = "trap '' TERM; sleep 600";
        let (output, elapsed) = timed(&["timeout", "-v", "-k", "0.3", "0.3", "sh", "-c", script]);
        assert_eq!(output.status.code(), Some(137));
        assert!(elapsed >= Duration::from_millis(600), "{elapsed:?}");
        assert!(elapsed < PROMPTLY, "{elapsed:?}");
        let log = stderr(&output);
        assert!(log.contains("sending signal TERM to command 'sh'"), "{log}");
        assert!(log.contains("sending signal KILL to command 'sh'"), "{log}");
    }

    #[test]
    fn kill_after_is_not_needed_when_the_command_obeys() {
        let script = "sleep 600 & wait";
        let (output, elapsed) = timed(&["timeout", "-k", "60", "0.3", "sh", "-c", script]);
        assert_eq!(output.status.code(), Some(124));
        assert!(elapsed < PROMPTLY, "{elapsed:?}");
    }

    #[test]
    fn kill_after_stops_children_that_outlive_the_command() {
        // The shell dies of TERM at once; the `sleep` it started ignores TERM.
        let script = "trap '' TERM; sleep 600 & echo $!; trap - TERM; wait";
        let (output, elapsed) = timed(&["timeout", "-k", "0.4", "0.3", "sh", "-c", script]);
        assert_eq!(output.status.code(), Some(124));
        assert!(elapsed >= Duration::from_millis(700), "{elapsed:?}");
        assert!(elapsed < PROMPTLY, "{elapsed:?}");
        let straggler = stdout(&output).trim().to_string();
        assert!(!straggler.is_empty());
        eventually("the straggler is gone", || !is_alive(&straggler));
    }

    #[test]
    fn kill_after_waits_for_children_that_are_still_shutting_down() {
        // The shell dies of TERM at once; its child takes a while to finish.
        let child = "trap 'sleep 0.5; exit 0' TERM; while :; do sleep 0.05; done";
        let script = format!("sh -c \"{child}\" & echo $!; wait");
        let (output, elapsed) = timed(&["timeout", "-k", "60", "0.3", "sh", "-c", &script]);
        assert_eq!(output.status.code(), Some(124));
        assert!(elapsed >= Duration::from_millis(800), "{elapsed:?}");
        assert!(elapsed < PROMPTLY, "{elapsed:?}");
        // Already gone when standby returns: no `eventually` here.
        assert!(!is_alive(stdout(&output).trim()));
    }

    /// 0.2.0 signalled only the command itself and left its children running.
    #[test]
    fn stops_the_commands_children_too() {
        let output = run(&["timeout", "0.3", "sh", "-c", "sleep 600 & echo $!; wait"]);
        assert_eq!(output.status.code(), Some(124));
        let grandchild = stdout(&output).trim().to_string();
        assert!(!grandchild.is_empty());
        eventually("the grandchild is gone", || !is_alive(&grandchild));
    }

    #[test]
    fn foreground_stops_only_the_command() {
        let (output, elapsed) = timed(&["timeout", "--foreground", "0.3", "sleep", FOREVER]);
        assert_eq!(output.status.code(), Some(124));
        assert!(elapsed < PROMPTLY, "{elapsed:?}");
    }

    #[test]
    fn a_command_that_cannot_be_run_reports_status_126() {
        let path = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("not-executable");
        std::fs::write(&path, "#!/bin/sh\n").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();

        let output = run(&["timeout", "5", path.to_str().unwrap()]);
        assert_eq!(output.status.code(), Some(126), "{}", stderr(&output));
    }

    #[test]
    fn dies_of_the_signal_that_ended_the_command() {
        let output = run(&["timeout", "60", "sh", "-c", "kill -TERM $$"]);
        assert_eq!(output.status.signal(), Some(15));

        let output = run(&["timeout", "60", "sh", "-c", "kill -KILL $$"]);
        assert_eq!(output.status.signal(), Some(9));
    }

    /// 0.2.0 died of the signal and left the command running.
    #[test]
    fn passes_signals_on_to_the_command() {
        let script = "trap 'exit 42' TERM; echo ready; sleep 600 & wait";
        let (mut standby, ready, _output) = start(&[FOREVER], script);
        assert_eq!(ready, "ready");

        send("TERM", &standby.pid());
        let status = standby.0.wait().unwrap();
        assert_eq!(status.code(), Some(42));
    }

    #[test]
    fn an_interrupt_ends_the_command_and_standby_alike() {
        let (mut standby, pid, _output) = start(&[FOREVER], "echo $$; exec sleep 600");

        send("INT", &standby.pid());
        let status = standby.0.wait().unwrap();
        assert_eq!(status.signal(), Some(2));
        eventually("the command is gone", || !is_alive(&pid));
    }

    #[test]
    fn a_passed_on_signal_starts_the_kill_after_clock() {
        let script = "trap '' TERM; echo ready; sleep 600";
        let (mut standby, ready, _output) = start(&["-k", "0.3", FOREVER], script);
        assert_eq!(ready, "ready");

        let start = Instant::now();
        send("TERM", &standby.pid());
        let status = standby.0.wait().unwrap();
        assert_eq!(status.code(), Some(137));
        assert!(start.elapsed() < PROMPTLY, "{:?}", start.elapsed());
    }

    #[test]
    fn suspends_and_resumes_the_command_with_itself() {
        // The command prints a line every few milliseconds for as long as it
        // runs, so silence means that it is stopped.
        let script = "while :; do echo tick; sleep 0.02; done";
        let (mut standby, tick, mut output) = start(&[FOREVER], script);
        assert_eq!(tick, "tick");
        let pid = standby.pid();

        let (ticks, received) = mpsc::channel();
        thread::spawn(move || {
            let mut line = String::new();
            while matches!(output.read_line(&mut line), Ok(count) if count > 0) {
                if ticks.send(()).is_err() {
                    break;
                }
                line.clear();
            }
        });
        let quiet = Duration::from_secs(1);

        send("TSTP", &pid);
        let start = Instant::now();
        while received.recv_timeout(quiet).is_ok() {
            assert!(start.elapsed() < PROMPTLY, "the command was not stopped");
        }

        // Continuing standby is what continues the command.
        send("CONT", &pid);
        received
            .recv_timeout(PROMPTLY)
            .expect("the command should run again");

        send("TERM", &pid);
        assert_eq!(standby.0.wait().unwrap().signal(), Some(15));
    }

    #[test]
    fn works_when_the_caller_ignores_child_signals() {
        // An ignored SIGCHLD is inherited and makes the kernel reap children
        // unseen; standby must not depend on what it inherits.
        let script = format!("trap '' CHLD; exec '{STANDBY}' timeout 0.3 sleep 600");
        let (output, elapsed) = {
            let start = Instant::now();
            let output = Command::new("sh").args(["-c", &script]).output().unwrap();
            (output, start.elapsed())
        };
        assert_eq!(output.status.code(), Some(124), "{}", stderr(&output));
        assert!(elapsed < PROMPTLY, "{elapsed:?}");
    }

    #[test]
    fn waits_for_a_process_it_did_not_start() {
        // A grandchild of the test, which no `wait` in a shell could wait for.
        let (_standby, pid, _output) = start(&[FOREVER], "sleep 0.5 & echo $!; wait; sleep 600");
        let (output, elapsed) = timed(&["wait", "-t", "15", &pid]);
        assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
        assert!(elapsed < PROMPTLY, "{elapsed:?}");
        assert!(!is_alive(&pid));
    }
}

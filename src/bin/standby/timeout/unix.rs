//! Supervising a command on Unix.
//!
//! The command runs in a process group of its own, and every signal goes to
//! that group, so its children stop with it. standby stays in the group it
//! was started in. A Ctrl-C or a `kill` therefore reaches standby whether it
//! was started from a terminal or from a script, and standby passes it on.
//!
//! One thread turns incoming signals into messages. The main thread waits for
//! a message or for the next deadline, whichever comes first, so nothing is
//! polled and nothing is done in signal context.

use super::{Failure, Outcome, limit};
use crate::cli::TimeoutArgs;
use crate::terminal::TerminalState;
use nix::errno::Errno;
use nix::sys::resource::{Resource, setrlimit};
use nix::sys::signal::{self, SigHandler, SigSet, Signal};
use nix::unistd::{self, Pid};
use signal_hook::consts::signal::{
    SIGCHLD, SIGHUP, SIGINT, SIGQUIT, SIGTERM, SIGTSTP, SIGUSR1, SIGUSR2,
};
use signal_hook::iterator::Signals;
use std::ffi::{OsStr, c_int};
use std::io;
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::process::{Child, Command, ExitStatus};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

/// Signals that would otherwise end standby and leave the command running.
/// They are passed on to the command instead.
const FORWARDED: [c_int; 6] = [SIGHUP, SIGINT, SIGQUIT, SIGTERM, SIGUSR1, SIGUSR2];

pub fn run(args: &TimeoutArgs) -> Result<i32, Failure> {
    // All of this comes before the command starts, so that no signal and no
    // change to the terminal can slip in unseen.
    let terminal = TerminalState::capture();
    let events = Events::listen(!args.foreground)?;
    if !args.foreground {
        ignore_terminal_stops();
    }

    let mut command = Command::new(&args.command[0]);
    command.args(&args.command[1..]);
    if !args.foreground {
        command.process_group(0);
    }
    let mut child = command.spawn().map_err(Failure::Spawn)?;

    let target = Target {
        // A process ID always fits: the kernel hands them out as `pid_t`.
        pid: Pid::from_raw(child.id() as i32),
        group: !args.foreground,
        verbose: args.verbose,
        name: &args.command[0],
    };

    let Supervised {
        status,
        timed_out,
        kill_at,
    } = match supervise(&mut child, &target, &events, args) {
        Ok(supervised) => supervised,
        Err(err) => {
            // Never leave the command running unsupervised.
            target.deliver(Signal::SIGKILL);
            let _ = child.wait();
            return Err(err.into());
        }
    };
    if let Some(kill_at) = kill_at
        && target.group
    {
        stop_stragglers(&target, &events, kill_at);
    }

    let ended_by = status.signal();
    if timed_out || ended_by.is_some() {
        terminal.restore();
    }
    if !timed_out && let Some(signal) = ended_by {
        // Reporting a death by signal as an ordinary exit would hide it from
        // a calling shell, which stops a script on Ctrl-C only when its
        // command died of SIGINT.
        die_of(signal);
    }

    let outcome = Outcome {
        status: status
            .code()
            .or(ended_by.map(|signal| 128 + signal))
            .unwrap_or(1),
        timed_out,
        killed: ended_by == Some(Signal::SIGKILL as i32),
    };
    Ok(outcome.exit_status(args.preserve_status))
}

/// What `supervise` saw.
struct Supervised {
    status: ExitStatus,
    /// A deadline was reached.
    timed_out: bool,
    /// The command exited after a signal and before `--kill-after` ran out:
    /// the time at which whatever is left of its process group is to be killed.
    kill_at: Option<Instant>,
}

/// Waits for the command to exit, sending signals as deadlines pass and
/// passing on the signals standby receives.
fn supervise(
    child: &mut Child,
    target: &Target,
    events: &Events,
    args: &TimeoutArgs,
) -> io::Result<Supervised> {
    let at_limit = Signal::try_from(args.signal.number()).unwrap_or(Signal::SIGTERM);
    let mut limit_at = limit(args.duration).and_then(|limit| Instant::now().checked_add(limit));
    let mut kill_after = args.kill_after.and_then(limit);
    let mut kill_at = None;
    let mut timed_out = false;

    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(Supervised {
                status,
                timed_out,
                kill_at,
            });
        }

        let signal = match events.next(kill_at.or(limit_at)) {
            Event::Signal(SIGCHLD) => continue,
            Event::Signal(SIGTSTP) => {
                target.suspend();
                continue;
            }
            Event::Signal(number) => match Signal::try_from(number) {
                Ok(signal) => signal,
                Err(_) => continue,
            },
            Event::Deadline => {
                timed_out = true;
                if kill_at.take().is_some() {
                    Signal::SIGKILL
                } else {
                    limit_at = None;
                    at_limit
                }
            }
        };
        target.signal(signal);

        // From the first signal on, whatever its origin, the command has
        // `kill_after` left to exit, and the time limit no longer matters.
        if let Some(grace) = kill_after.take() {
            kill_at = Instant::now().checked_add(grace);
            limit_at = None;
        }
    }
}

/// Waits for the rest of the command's process group once the command itself
/// has exited, and kills what is left of it at `kill_at`.
///
/// The command often exits at the first signal while processes it started are
/// still shutting down, or ignore the signal. With `--kill-after`, standby
/// returns only when none of them is left.
fn stop_stragglers(target: &Target, events: &Events, kill_at: Instant) {
    // Nothing reports that a process group has emptied, so it is probed.
    const LONGEST_PAUSE: Duration = Duration::from_millis(20);
    let mut pause = Duration::from_millis(1);

    // A group ID is not given to anything else while the group has members.
    // Each signal below directly follows a probe that found some, so it
    // cannot reach an unrelated group that took over the ID.
    while target.group_has_members() {
        let now = Instant::now();
        if now >= kill_at {
            target.announce(Signal::SIGKILL);
            let _ = signal::killpg(target.pid, Signal::SIGKILL);
            return;
        }
        match events.next(Some(kill_at.min(now + pause))) {
            Event::Signal(SIGCHLD | SIGTSTP) | Event::Deadline => {}
            Event::Signal(number) => {
                if let Ok(signal) = Signal::try_from(number) {
                    let _ = signal::killpg(target.pid, signal);
                }
            }
        }
        pause = (pause * 2).min(LONGEST_PAUSE);
    }
}

/// The command, as something to send signals to.
struct Target<'a> {
    pid: Pid,
    /// The command leads a process group of its own.
    group: bool,
    verbose: bool,
    name: &'a OsStr,
}

impl Target<'_> {
    /// Reports a signal on stderr when `--verbose` is set.
    fn announce(&self, signal: Signal) {
        if self.verbose {
            let name = signal.as_str();
            eprintln!(
                "standby: sending signal {} to command '{}'",
                name.strip_prefix("SIG").unwrap_or(name),
                self.name.to_string_lossy()
            );
        }
    }

    /// Whether any process is left in the command's process group. Only
    /// meaningful once the command itself has been reaped.
    fn group_has_members(&self) -> bool {
        signal::killpg(self.pid, None) != Err(Errno::ESRCH)
    }

    /// Sends a signal to the command and makes sure it is acted on.
    fn signal(&self, signal: Signal) {
        self.announce(signal);
        self.deliver(signal);
        // A stopped process acts on a signal only once it is continued.
        let stops = matches!(
            signal,
            Signal::SIGSTOP | Signal::SIGTSTP | Signal::SIGTTIN | Signal::SIGTTOU
        );
        if self.group && !stops && signal != Signal::SIGKILL && signal != Signal::SIGCONT {
            self.deliver(Signal::SIGCONT);
        }
    }

    /// Sends a signal to the command's process group, or to the command alone
    /// when it shares standby's group.
    ///
    /// Errors are ignored: the only one that can happen here is that there is
    /// nothing left to signal. Must not be called once the command has been
    /// reaped, because its process ID may then belong to something else.
    fn deliver(&self, signal: Signal) {
        if !self.group {
            let _ = signal::kill(self.pid, signal);
            return;
        }
        let _ = signal::killpg(self.pid, signal);
        // A command that has moved itself to another group is not reached
        // through the group it left.
        if unistd::getpgid(Some(self.pid)) != Ok(self.pid) {
            let _ = signal::kill(self.pid, signal);
        }
    }

    /// Stops the command and then standby, as Ctrl-Z would stop one job, and
    /// resumes the command once standby is continued.
    fn suspend(&self) {
        self.deliver(Signal::SIGTSTP);
        let _ = signal::raise(Signal::SIGSTOP);
        self.deliver(Signal::SIGCONT);
    }
}

enum Event {
    /// standby received this signal.
    Signal(c_int),
    /// The deadline passed.
    Deadline,
}

/// Signals received by standby, as a queue.
struct Events(Receiver<c_int>);

impl Events {
    /// Starts collecting signals. With `job_control`, Ctrl-Z is collected too.
    fn listen(job_control: bool) -> io::Result<Self> {
        let mut wanted = vec![SIGCHLD];
        wanted.extend(FORWARDED);
        if job_control {
            wanted.push(SIGTSTP);
        }

        // A signal mask survives exec. If the caller blocked any of these,
        // standby would never see them.
        let mut mask = SigSet::empty();
        for signal in wanted
            .iter()
            .filter_map(|&number| Signal::try_from(number).ok())
        {
            mask.add(signal);
        }
        mask.thread_unblock()?;

        let mut signals = Signals::new(&wanted)?;
        let (sender, receiver) = mpsc::channel();
        thread::Builder::new()
            .name("signals".to_string())
            .spawn(move || {
                for signal in signals.forever() {
                    if sender.send(signal).is_err() {
                        break;
                    }
                }
            })?;
        Ok(Self(receiver))
    }

    /// Waits for the next signal, or until `deadline` if that comes first.
    fn next(&self, deadline: Option<Instant>) -> Event {
        let received = match deadline {
            Some(deadline) => self
                .0
                .recv_timeout(deadline.saturating_duration_since(Instant::now())),
            None => self.0.recv().map_err(|_| RecvTimeoutError::Disconnected),
        };
        match received {
            Ok(signal) => Event::Signal(signal),
            Err(RecvTimeoutError::Timeout) => Event::Deadline,
            Err(RecvTimeoutError::Disconnected) => {
                // The signal thread is gone, which should not happen. Fall
                // back to checking on the command at short intervals.
                thread::sleep(Duration::from_millis(10));
                match deadline {
                    Some(deadline) if Instant::now() >= deadline => Event::Deadline,
                    _ => Event::Signal(SIGCHLD),
                }
            }
        }
    }
}

/// Makes a read from the terminal fail instead of stopping the reader.
///
/// The command runs outside the terminal's foreground process group. By
/// default the kernel would stop it the moment it read from the terminal, and
/// it would sit there until the time limit. Ignored signals are inherited
/// across exec, so ignoring these here makes such a read fail at once.
fn ignore_terminal_stops() {
    for signal in [Signal::SIGTTIN, Signal::SIGTTOU] {
        // SAFETY: `SIG_IGN` installs no handler, so no code of ours can run
        // in signal context, and no earlier handler for these signals exists
        // that the call could invalidate.
        let _ = unsafe { signal::signal(signal, SigHandler::SigIgn) };
    }
}

/// Ends standby with the signal that ended the command.
///
/// Returns if that is not possible; the caller then exits with 128 plus the
/// signal number.
fn die_of(signal: c_int) {
    // If the signal is one that dumps core, the core would be standby's and
    // would pass for the command's. Without this guarantee, do not raise.
    if setrlimit(Resource::RLIMIT_CORE, 0, 0).is_ok() {
        let _ = signal_hook::low_level::emulate_default_handler(signal);
    }
}

# Security

standby starts and signals processes, so a defect in it can matter. Please report one privately rather than in a public issue.

## Reporting

Use [Report a vulnerability](https://github.com/Epistates/standby/security/advisories/new) on GitHub. Include the version (`standby --version`), the platform, and the steps to reproduce.

## Supported versions

Fixes go into the latest release. Older releases are not patched.

## What standby does and does not guarantee

These are known limits of the design, not vulnerabilities. They are described in the [README](README.md).

- standby does not sandbox the command. It runs with the same user, environment and permissions as standby.
- On Unix, `KILL` cannot be caught. If standby itself is killed with `KILL`, the command keeps running.
- A process that moves itself out of the command's process group or session (on Unix), or out of the job object (on Windows), is not stopped at the time limit.
- On Windows there is a short gap between starting the command and placing it in the job object. A process the command starts in that gap is not covered.
- `standby wait` can tell that a process exited, but not how. The exit status belongs to its parent.

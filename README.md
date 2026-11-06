# repete

![Crates.io License](https://img.shields.io/crates/l/repete)
[![GitHub Tag](https://img.shields.io/github/v/tag/qrichert/repete?sort=semver&filter=*.*.*&label=release)](https://github.com/qrichert/repete/releases/latest)
[![crates.io](https://img.shields.io/crates/d/repete?logo=rust&logoColor=white&color=orange)](https://crates.io/crates/repete)
[![GitHub Actions Workflow Status](https://img.shields.io/github/actions/workflow/status/qrichert/repete/ci.yml?label=tests)](https://github.com/qrichert/repete/actions)

_Run a command repeatedly._

Run a command a fixed number of times, or keep going until you press
Ctrl-C. Commands run one at a time, with an optional interval between
runs.

```console
$ repete 3 echo foo
foo
foo
foo
```

## Get `--help`

```
Run a command repeatedly.

Usage: repete [OPTIONS] [COUNT] COMMAND [ARG ...]

Options:
  -n, --interval <SECONDS>          Wait between commands.
  -N, --precise-interval <SECONDS>  Account for command runtime and drift.

Other:
  -h, --help                        Show this message and exit.
  -V, --version                     Show the version and exit.

For full help, see `repete --help`.
```

### Examples

Give a count to run a command that many times, or leave it out to repeat
until interrupted:

```console
# Run ten times.
$ repete 10 echo foo

# Keep running until Ctrl-C.
$ repete echo foo
```

Commands run one at a time. The first command starts immediately.
Without an interval, the next run starts as soon as the previous one
finishes. A count of zero runs nothing, but still requires a command.

### Intervals

Use `--interval` to wait between commands:

```console
# Wait five seconds after each run.
$ repete 10 --interval 5 echo foo

# Fractional seconds work too.
$ repete --interval 0.5 date
```

Use `--precise-interval` to account for the time spent running the
command and keep the original cadence:

```console
$ repete 10 --precise-interval 5 echo foo
```

If a command overruns its next scheduled tick, the next run starts
immediately, then returns to the original cadence. For example, with a
five-second interval, if the first run takes twelve seconds and the
second takes one second, the runs start at 0, 12, and 15 seconds.
Scheduling and process launch latency can make actual launches late.

Intervals accept finite, nonnegative seconds, including fractions, with
nanosecond resolution. Zero adds no delay. Attached forms such as
`-n0.5`, `-N5`, `--interval=5`, and `--precise-interval=5` work too.
There is no wait after the last run. If you pass several interval
options, the last one wins.

### Commands

Commands run directly, with their arguments passed through unchanged:

```console
$ repete 2 printf '%s\n' 'a b' '' '$HOME'
```

Your calling shell expands unquoted variables and globs before repete
starts. Use an explicit shell for pipelines, redirections, or
expressions that should be evaluated on every run:

```console
$ repete 10 sh -c 'printf "%s\n" "$HOME" | cat'
```

Shell aliases and functions are not standalone executables.

Options can appear on either side of the count, before the command.
Options after the command belong to that command. Use `--` before a
command whose name is a number or starts with a dash:

```console
$ repete 2 -- 123
```

Standard input, standard output, standard error, environment, and
working directory are inherited. Colors, input, and redirected output
work as they do when you run the command directly. There are no banners,
separators, screen clearing, or output capture.

Ctrl-C and Ctrl-Z retain ordinary foreground process-group behavior. A
signal sent only to repete's PID does not guarantee descendant cleanup.
Commands that ignore terminal signals or detach descendants remain
responsible for them.

### Exit status

A nonzero exit does not stop repetition. When a count is given, `repete`
returns the last command's exit code. A command terminated by a signal
stops repetition and returns 128 plus the signal number. An explicit
shell reports its own exit status.

Failure to start the command stops repetition: a missing executable
returns 127, permission or executable-format errors return 126, and
other execution errors return 1. Invalid usage returns 2. Diagnostics go
to standard error. A count of zero returns 0.

Help and version exit successfully without a command.

## Installation

Install the `repete` command from [crates.io] with Cargo:

```shell
cargo install repete
```

Pre-built binaries for Linux and macOS are available on the [latest
GitHub release].

[Documentation] is available on docs.rs.

[crates.io]: https://crates.io/crates/repete
[latest GitHub release]:
  https://github.com/qrichert/repete/releases/latest
[Documentation]: https://docs.rs/repete

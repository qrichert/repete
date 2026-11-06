# repete

Run a command repeatedly, with an optional count and interval.

```text
repete [OPTIONS] [COUNT] COMMAND [ARG ...]
```

```sh
repete 10 echo foo
repete echo foo
repete 10 -n 5 echo foo
repete 10 -N 5 echo foo
repete 10 sh -c 'printf foo | cat'
```

The first command starts immediately. A count runs exactly that many commands;
without one, execution continues until interrupted or an execution failure
occurs. Zero runs no commands, but still requires a command. Commands run one at
a time. Without an interval, there is no added delay; there is also no delay
after the final finite execution.

`-n SECONDS` / `--interval SECONDS` waits after each command finishes.
`-N SECONDS` / `--precise-interval SECONDS` keeps the original launch cadence:
if a command overruns its next scheduled tick, the next command starts
immediately, then returns to the original grid. With a 5-second interval and
runtimes of 12 seconds, then 1 second, launches occur at 0, 12, and 15 seconds.
Scheduling and process launch latency can make actual launches late.

Intervals accept finite, nonnegative fractional seconds, with nanosecond
resolution. Zero adds no delay. Attached forms (`-n0.1`, `-N5`, `--interval=5`,
`--precise-interval=5`) work too. If several timing options occur, the last wins.

Options can appear on either side of the count, before the command. Once the
command appears, all remaining arguments belong to it. Use `--` before a
numeric or dash-prefixed executable name:

```sh
repete 2 printf '%s\n' 'a b' '' '$HOME'
repete 2 -- 123
repete 10 sh -c 'printf "%s\n" "$HOME" | cat'
```

Commands execute directly, preserving argument bytes. Shell syntax requires an
explicit shell. Your calling shell expands unquoted variables and globs before
`repete` starts; use `sh -c` to expand them on every iteration. Shell aliases and
functions are not standalone executables.

Standard input, standard output, standard error, environment, and working
directory are inherited. There are no banners, separators, screen clearing, or
output capture. Terminal detection and color remain the command's decision.
Ctrl-C and Ctrl-Z retain ordinary foreground process-group behavior. A signal
sent only to the wrapper PID does not promise descendant cleanup; commands that
ignore terminal signals or detach descendants remain responsible for them.

Ordinary nonzero child exits do not stop repetition. A finite run returns the
last child's exit code. Direct child termination by a signal stops repetition
and returns 128 plus the signal number. An explicit shell reports its own
status. Launch failures stop immediately: missing executable returns 127,
permission or executable-format failure returns 126, and other execution errors
return 1. Invalid wrapper usage returns 2. Wrapper diagnostics go to stderr.

`repete -h` / `repete --help` prints help; `repete -V` / `repete --version`
prints the version. Both exit successfully without a command.

Build and run with Rust and Cargo on Unix (macOS or Linux):

```sh
cargo build --release
./target/release/repete 10 echo foo
cargo run -- 10 -n 0.1 echo foo
cargo test --all-targets
just lint
```

The program has no Cargo dependencies. `just test` also runs the Rust tests.

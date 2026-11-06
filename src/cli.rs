pub mod ui;

use std::ffi::OsString;
use std::os::unix::ffi::OsStrExt;
use std::time::Duration;

#[derive(Debug, Eq, PartialEq)]
pub struct Config {
    pub count: Option<u64>,
    pub timing: Timing,
    pub command: OsString,
    pub args: Vec<OsString>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Timing {
    Immediate,
    Interval(Duration),
    Precise(Duration),
}

#[derive(Debug, Eq, PartialEq)]
pub enum Action {
    Run(Config),
    Help,
    LongHelp,
    Version,
}

impl Config {
    pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Action, String> {
        let mut args = args.into_iter();
        let mut count = None;
        let mut timing = Timing::Immediate;
        let mut options = true;

        while let Some(arg) = args.next() {
            if options {
                let text = arg.to_str();
                match text {
                    Some("-h") => return Ok(Action::Help),
                    Some("--help") => return Ok(Action::LongHelp),
                    Some("-V" | "--version") => return Ok(Action::Version),
                    Some("--") => {
                        options = false;
                        continue;
                    }
                    Some("-n" | "--interval" | "-N" | "--precise-interval") => {
                        let value = args
                            .next()
                            .ok_or_else(|| format!("Missing value for '{}'", arg.display()))?;
                        timing = parse_timing(text.unwrap(), &value)?;
                        continue;
                    }
                    _ => {}
                }
                if let Some((option, value)) = text.and_then(attached_interval) {
                    timing = parse_timing(option, &OsString::from(value))?;
                    continue;
                }
                let bytes = arg.as_bytes();
                if bytes.starts_with(b"-") {
                    return Err(format!("Unknown option '{}'", arg.display()));
                }
                if count.is_none() && !bytes.is_empty() && bytes.iter().all(u8::is_ascii_digit) {
                    count = Some(
                        text.unwrap()
                            .parse()
                            .map_err(|_| format!("Count is too large: '{}'", arg.display()))?,
                    );
                    continue;
                }
            }
            return Ok(Action::Run(Self {
                count,
                timing,
                command: arg,
                args: args.collect(),
            }));
        }
        Err("Missing command".into())
    }
}

fn attached_interval(arg: &str) -> Option<(&str, &str)> {
    for option in ["--interval=", "--precise-interval=", "-n", "-N"] {
        if let Some(value) = arg.strip_prefix(option) {
            return Some((option.trim_end_matches('='), value));
        }
    }
    None
}

fn parse_timing(option: &str, value: &OsString) -> Result<Timing, String> {
    let invalid = || format!("Invalid interval '{}'", value.display());
    let seconds = value
        .to_str()
        .and_then(|text| text.parse::<f64>().ok())
        .ok_or_else(invalid)?;
    let duration = Duration::try_from_secs_f64(seconds).map_err(|_| invalid())?;
    Ok(if matches!(option, "-n" | "--interval") {
        Timing::Interval(duration)
    } else {
        Timing::Precise(duration)
    })
}

pub fn help_message() -> String {
    format!(
        "\
{description}

Usage: {bin} [OPTIONS] [COUNT] COMMAND [ARG ...]

Options:
  -n, --interval <SECONDS>          Wait between commands.
  -N, --precise-interval <SECONDS>  Account for command runtime and drift.

Other:
  -h, --help                        Show this message and exit.
  -V, --version                     Show the version and exit.
",
        description = env!("CARGO_PKG_DESCRIPTION"),
        bin = env!("CARGO_BIN_NAME"),
    )
}

pub fn longer_help_notice() -> String {
    format!(
        "For full help, see `{bin} --help`.",
        bin = env!("CARGO_BIN_NAME")
    )
}

pub fn long_help_message() -> String {
    format!(
        "\
{help}
Examples:
  Give a count to run a command that many times, or leave it out to
  repeat until interrupted:

      {attenuate}# Run ten times.{reset}
      {highlight}${reset} {bin} 10 echo foo

      {attenuate}# Keep running until Ctrl-C.{reset}
      {highlight}${reset} {bin} echo foo

  Commands run one at a time. The first command starts immediately.
  Without an interval, the next run starts as soon as the previous one
  finishes. A count of zero runs nothing, but still requires a command.

Intervals:
  Use `--interval` to wait between commands:

      {attenuate}# Wait five seconds after each run.{reset}
      {highlight}${reset} {bin} 10 --interval 5 echo foo

      {attenuate}# Fractional seconds work too.{reset}
      {highlight}${reset} {bin} --interval 0.5 date

  Use `--precise-interval` to account for the time spent running the
  command and keep the original cadence:

      {highlight}${reset} {bin} 10 --precise-interval 5 echo foo

  If a command overruns its next scheduled tick, the next run starts
  immediately, then returns to the original cadence. For example, with
  a five-second interval, if the first run takes twelve seconds and the
  second takes one second, the runs start at 0, 12, and 15 seconds.
  Scheduling and process launch latency can make actual launches late.

  Intervals accept finite, nonnegative seconds, including fractions,
  with nanosecond resolution. Zero adds no delay. Attached forms such
  as `-n0.5`, `-N5`, `--interval=5`, and `--precise-interval=5` work too.
  There is no wait after the last run. If you pass several interval
  options, the last one wins.

Commands:
  Commands run directly, with their arguments passed through unchanged:

      {highlight}${reset} {bin} 2 printf '%s\\n' 'a b' '' '$HOME'

  Your calling shell expands unquoted variables and globs before
  {bin} starts. Use an explicit shell for pipelines, redirections,
  or expressions that should be evaluated on every run:

      {highlight}${reset} {bin} 10 sh -c 'printf \"%s\\n\" \"$HOME\" | cat'

  Shell aliases and functions are not standalone executables.

  Options can appear on either side of the count, before the command.
  Options after the command belong to that command. Use `--` before
  a command whose name is a number or starts with a dash:

      {highlight}${reset} {bin} 2 -- 123

  Standard input, standard output, standard error, environment, and
  working directory are inherited. Colors, input, and redirected output
  work as they do when you run the command directly. There are no
  banners, separators, screen clearing, or output capture.

  Ctrl-C and Ctrl-Z retain ordinary foreground process-group behavior.
  A signal sent only to {bin}'s PID does not guarantee descendant
  cleanup. Commands that ignore terminal signals or detach descendants
  remain responsible for them.

Exit status:
  A nonzero exit does not stop repetition. When a count is given,
  {bin} returns the last command's exit code. A command terminated
  by a signal stops repetition and returns 128 plus the signal number.
  An explicit shell reports its own exit status.

  Failure to start the command stops repetition: a missing executable
  returns 127, permission or executable-format errors return 126, and
  other execution errors return 1. Invalid usage returns 2. Diagnostics
  go to standard error. A count of zero returns 0.

  Help and version exit successfully without a command.
",
        help = help_message(),
        bin = env!("CARGO_BIN_NAME"),
        highlight = ui::Color::maybe_color(ui::HIGHLIGHT),
        attenuate = ui::Color::maybe_color(ui::ATTENUATE),
        reset = ui::Color::maybe_color(ui::RESET),
    )
}

pub fn version_message() -> String {
    format!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_and_command_boundary() {
        assert_eq!(config(&["echo"]).count, None);
        assert_eq!(config(&["0", "echo"]).count, Some(0));
        assert_eq!(config(&["10", "echo"]).count, Some(10));
        let parsed = config(&["2", "123", "-n", "--help", "--"]);
        assert_eq!(parsed.command, "123");
        assert_eq!(parsed.args, ["-n", "--help", "--"]);
        assert_eq!(config(&["--", "123"]).command, "123");
        assert_eq!(config(&["--", "-tool"]).command, "-tool");
        assert_eq!(config(&["2", "--", "123"]).count, Some(2));
    }

    #[test]
    fn interval_spellings_and_positions() {
        for args in [
            vec!["-n", "0.1", "2", "echo"],
            vec!["2", "-n0.1", "echo"],
            vec!["2", "--interval", "0.1", "echo"],
            vec!["--interval=0.1", "2", "echo"],
        ] {
            let parsed = config(&args);
            assert_eq!(parsed.count, Some(2));
            assert_eq!(parsed.timing, Timing::Interval(Duration::from_millis(100)));
        }
        for args in [
            vec!["-N", "0.1", "echo"],
            vec!["-N0.1", "echo"],
            vec!["--precise-interval", "0.1", "echo"],
            vec!["--precise-interval=0.1", "echo"],
        ] {
            assert_eq!(
                config(&args).timing,
                Timing::Precise(Duration::from_millis(100))
            );
        }
        assert_eq!(
            config(&["-n0", "echo"]).timing,
            Timing::Interval(Duration::ZERO)
        );
        assert_eq!(
            config(&["-n1", "-N2", "-n3", "echo"]).timing,
            Timing::Interval(Duration::from_secs(3))
        );
    }

    #[test]
    fn invalid_input_is_a_usage_error() {
        for args in [
            vec![],
            vec!["0"],
            vec!["--"],
            vec!["18446744073709551616", "echo"],
            vec!["-x", "echo"],
            vec!["-n"],
            vec!["--interval=", "echo"],
        ] {
            assert!(parse(&args).is_err(), "{args:?}");
        }
        for value in ["-1", "NaN", "inf", "-inf", "1e100", "bad"] {
            assert!(parse(&["-n", value, "echo"]).is_err(), "{value}");
        }
    }

    #[test]
    fn help_and_version_need_no_command() {
        assert_eq!(parse(&["-h"]), Ok(Action::Help));
        assert_eq!(parse(&["--help"]), Ok(Action::LongHelp));
        for option in ["-V", "--version"] {
            assert_eq!(parse(&[option]), Ok(Action::Version));
        }
    }

    fn config(args: &[&str]) -> Config {
        let Action::Run(config) = parse(args).unwrap() else {
            panic!("expected a command");
        };
        config
    }

    fn parse(args: &[&str]) -> Result<Action, String> {
        Config::parse(args.iter().map(OsString::from))
    }
}

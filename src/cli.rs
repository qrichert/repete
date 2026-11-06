use std::ffi::OsString;
use std::os::unix::ffi::OsStrExt;
use std::time::Duration;

#[derive(Debug, PartialEq)]
pub struct Config {
    pub count: Option<u64>,
    pub timing: Timing,
    pub command: OsString,
    pub args: Vec<OsString>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Timing {
    Immediate,
    Interval(Duration),
    Precise(Duration),
}

#[derive(Debug, PartialEq)]
pub enum Action {
    Run(Config),
    Help,
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
                    Some("-h" | "--help") => return Ok(Action::Help),
                    Some("-V" | "--version") => return Ok(Action::Version),
                    Some("--") => {
                        options = false;
                        continue;
                    }
                    Some("-n" | "--interval" | "-N" | "--precise-interval") => {
                        let value = args
                            .next()
                            .ok_or_else(|| format!("missing value for '{}'", arg.display()))?;
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
                    return Err(format!("unknown option '{}'", arg.display()));
                }
                if count.is_none() && !bytes.is_empty() && bytes.iter().all(u8::is_ascii_digit) {
                    count = Some(
                        text.unwrap()
                            .parse()
                            .map_err(|_| format!("count is too large: '{}'", arg.display()))?,
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
        Err("missing command".into())
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
    let invalid = || format!("invalid interval '{}'", value.display());
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

pub fn help_message() -> &'static str {
    "Run a command repeatedly.

Usage: repete [OPTIONS] [COUNT] COMMAND [ARG ...]

Options:
  -n, --interval <SECONDS>          Wait between commands.
  -N, --precise-interval <SECONDS>  Keep the original launch cadence.
  -h, --help                        Show this message and exit.
  -V, --version                     Show the version and exit.

Without COUNT, repeat until interrupted. Zero runs no commands.
Without an interval, commands run without an added delay.
Precise intervals restart immediately on overruns, then resume the cadence.
Intervals accept nonnegative fractional seconds. The last timing option wins.
Options end at COMMAND; use -- to run a numeric or dash-prefixed command.
Commands run directly. Use an explicit shell for shell expressions.

Examples:
  repete 10 echo foo
  repete -n 0.5 echo foo
  repete 10 -N 5 echo foo
  repete 10 sh -c 'printf foo | cat'"
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
        for option in ["-h", "--help"] {
            assert_eq!(parse(&[option]), Ok(Action::Help));
        }
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

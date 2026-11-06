use std::io;
use std::os::unix::process::ExitStatusExt;
use std::process::Command;
use std::time::{Duration, Instant};

use crate::cli::{Config, Timing};

pub struct RunError {
    pub code: i32,
    pub message: String,
}

pub fn run(config: &Config) -> Result<i32, RunError> {
    let mut remaining = config.count;
    let mut last_code = 0;
    let mut command = Command::new(&config.command);
    command.args(&config.args);
    let mut epoch = None;

    while remaining != Some(0) {
        let launched = Instant::now();
        let epoch = *epoch.get_or_insert(launched);
        let started = launched.duration_since(epoch);
        let mut child = command
            .spawn()
            .map_err(|error| execution_error(config, &error))?;
        let status = child.wait().map_err(|error| RunError {
            code: 1,
            message: format!("cannot wait for '{}': {error}", config.command.display()),
        })?;
        if let Some(signal) = status.signal() {
            return Ok(128 + signal);
        }
        last_code = status.code().unwrap_or(1);
        if let Some(count) = &mut remaining {
            *count -= 1;
        }
        if remaining == Some(0) {
            break;
        }

        match config.timing {
            Timing::Immediate => {}
            Timing::Interval(interval) => std::thread::sleep(interval),
            Timing::Precise(_) => loop {
                let delay = next_delay(config.timing, started, epoch.elapsed());
                if delay.is_zero() {
                    break;
                }
                std::thread::sleep(delay);
            },
        }
    }
    Ok(last_code)
}

fn execution_error(config: &Config, error: &io::Error) -> RunError {
    let code = match (error.kind(), error.raw_os_error()) {
        (io::ErrorKind::NotFound, _) => 127,
        (io::ErrorKind::PermissionDenied, _) | (_, Some(8)) => 126, // ENOEXEC.
        #[cfg(target_os = "macos")]
        (_, Some(85 | 86 | 88)) => 126, // EBADEXEC, EBADARCH, EBADMACHO.
        _ => 1,
    };
    RunError {
        code,
        message: format!("cannot execute '{}': {error}", config.command.display()),
    }
}

fn next_delay(timing: Timing, started: Duration, finished: Duration) -> Duration {
    match timing {
        Timing::Immediate => Duration::ZERO,
        Timing::Interval(interval) => interval,
        Timing::Precise(interval) if interval.is_zero() => Duration::ZERO,
        Timing::Precise(interval) => {
            let phase = started.as_nanos() % interval.as_nanos();
            let phase = Duration::new(
                u64::try_from(phase / 1_000_000_000).expect("phase fits in interval"),
                u32::try_from(phase % 1_000_000_000).expect("nanoseconds fit in u32"),
            );
            interval
                .saturating_sub(phase)
                .saturating_sub(finished.saturating_sub(started))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinary_interval_does_not_include_runtime() {
        let timing = Timing::Interval(Duration::from_secs(5));
        assert_eq!(
            next_delay(timing, Duration::ZERO, Duration::from_secs(12)),
            Duration::from_secs(5)
        );
    }

    #[test]
    fn precise_delay_stays_on_original_grid() {
        let timing = Timing::Precise(Duration::from_secs(5));
        for (start, finish, delay) in [
            (0, 2, 3),
            (5, 7, 3),
            (0, 5, 0),
            (0, 12, 0),
            (12, 13, 2),
            (0, 27, 0),
            (27, 28, 2),
        ] {
            assert_eq!(
                next_delay(
                    timing,
                    Duration::from_secs(start),
                    Duration::from_secs(finish)
                ),
                Duration::from_secs(delay)
            );
        }
        assert_eq!(
            next_delay(timing, Duration::from_millis(5200), Duration::from_secs(7)),
            Duration::from_secs(3)
        );
    }

    #[test]
    fn fractional_and_zero_intervals() {
        assert_eq!(
            next_delay(
                Timing::Precise(Duration::from_millis(100)),
                Duration::from_millis(150),
                Duration::from_millis(175)
            ),
            Duration::from_millis(25)
        );
        for timing in [
            Timing::Immediate,
            Timing::Precise(Duration::ZERO),
            Timing::Interval(Duration::ZERO),
        ] {
            assert_eq!(
                next_delay(timing, Duration::ZERO, Duration::from_secs(1)),
                Duration::ZERO
            );
        }
    }
}

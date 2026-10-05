use std::time::Duration;

pub use psst_core::util::is_transient_network_error as transient;

pub fn run<T>(
    mut request: impl FnMut() -> Result<T, ureq::Error>,
    safe: bool,
    mut sleep: impl FnMut(Duration),
) -> Result<T, ureq::Error> {
    for attempt in 0..3 {
        match request() {
            Err(error) if safe && attempt < 2 && transient(&error) => {
                sleep(Duration::from_millis(250 << attempt))
            }
            result => return result,
        }
    }
    unreachable!()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retries_reads_and_stops_after_three_attempts() {
        let mut attempts = 0;
        let mut waits = Vec::new();
        let value = run(
            || {
                attempts += 1;
                if attempts < 3 {
                    Err(ureq::Error::ConnectionFailed)
                } else {
                    Ok(42)
                }
            },
            true,
            |delay| waits.push(delay),
        )
        .unwrap();
        assert_eq!(value, 42);
        assert_eq!(attempts, 3);
        assert_eq!(
            waits,
            vec![Duration::from_millis(250), Duration::from_millis(500)]
        );
        attempts = 0;
        assert!(run::<()>(
            || {
                attempts += 1;
                Err(ureq::Error::ConnectionFailed)
            },
            true,
            |_| {}
        )
        .is_err());
        assert_eq!(attempts, 3);
    }
    #[test]
    fn ambiguous_writes_and_permanent_failures_are_not_retried() {
        let mut attempts = 0;
        assert!(run::<()>(
            || {
                attempts += 1;
                Err(ureq::Error::ConnectionFailed)
            },
            false,
            |_| panic!("must not sleep")
        )
        .is_err());
        assert_eq!(attempts, 1);
        assert!(!transient(&ureq::Error::StatusCode(403)));
    }
}

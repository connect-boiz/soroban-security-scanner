//! Connection retry with exponential backoff.
//!
//! Provides a backoff schedule (base delay, multiplier, cap, max attempts) and
//! a driver that retries a fallible operation. The sleep function is injected
//! so the schedule is fully testable without real time passing.

use serde::{Deserialize, Serialize};

/// Exponential-backoff policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackoffPolicy {
    /// Delay before the first retry, in milliseconds.
    pub base_ms: u64,
    /// Multiplier applied each subsequent attempt.
    pub multiplier: u64,
    /// Maximum delay between attempts, in milliseconds.
    pub max_delay_ms: u64,
    /// Maximum number of retries after the initial attempt.
    pub max_retries: u32,
}

impl Default for BackoffPolicy {
    fn default() -> Self {
        Self {
            base_ms: 100,
            multiplier: 2,
            max_delay_ms: 10_000,
            max_retries: 5,
        }
    }
}

impl BackoffPolicy {
    /// Delay (ms) before the given retry attempt (1-based), capped at `max_delay_ms`.
    pub fn delay_ms(&self, attempt: u32) -> u64 {
        if attempt == 0 {
            return 0;
        }
        // base * multiplier^(attempt-1), saturating, capped.
        let mut delay = self.base_ms;
        for _ in 1..attempt {
            delay = delay.saturating_mul(self.multiplier);
            if delay >= self.max_delay_ms {
                return self.max_delay_ms;
            }
        }
        delay.min(self.max_delay_ms)
    }
}

/// Runs `op`, retrying on `Err` per `policy`. `sleep` is invoked with the delay
/// (ms) before each retry — pass a real sleeper in production, a recorder in
/// tests. `op` receives the attempt number (0 = initial).
pub fn retry_with<T, E, FOp, FSleep>(
    policy: &BackoffPolicy,
    mut op: FOp,
    mut sleep: FSleep,
) -> Result<T, E>
where
    FOp: FnMut(u32) -> Result<T, E>,
    FSleep: FnMut(u64),
{
    let mut attempt = 0;
    loop {
        match op(attempt) {
            Ok(value) => return Ok(value),
            Err(err) => {
                if attempt >= policy.max_retries {
                    return Err(err);
                }
                attempt += 1;
                sleep(policy.delay_ms(attempt));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[test]
    fn backoff_grows_then_caps() {
        let p = BackoffPolicy {
            base_ms: 100,
            multiplier: 2,
            max_delay_ms: 1000,
            max_retries: 10,
        };
        assert_eq!(p.delay_ms(1), 100);
        assert_eq!(p.delay_ms(2), 200);
        assert_eq!(p.delay_ms(3), 400);
        assert_eq!(p.delay_ms(4), 800);
        assert_eq!(p.delay_ms(5), 1000); // capped
        assert_eq!(p.delay_ms(6), 1000);
    }

    #[test]
    fn succeeds_after_transient_failures() {
        let p = BackoffPolicy::default();
        let delays = RefCell::new(Vec::new());
        let result: Result<&str, &str> = retry_with(
            &p,
            |attempt| if attempt < 2 { Err("conn refused") } else { Ok("connected") },
            |d| delays.borrow_mut().push(d),
        );
        assert_eq!(result, Ok("connected"));
        // Two retries → two sleeps with growing delays.
        assert_eq!(delays.borrow().as_slice(), &[100, 200]);
    }

    #[test]
    fn gives_up_after_max_retries() {
        let p = BackoffPolicy {
            max_retries: 3,
            ..BackoffPolicy::default()
        };
        let attempts = RefCell::new(0);
        let result: Result<(), &str> = retry_with(
            &p,
            |_| {
                *attempts.borrow_mut() += 1;
                Err("down")
            },
            |_| {},
        );
        assert_eq!(result, Err("down"));
        // initial + 3 retries
        assert_eq!(*attempts.borrow(), 4);
    }

    #[test]
    fn immediate_success_does_not_sleep() {
        let p = BackoffPolicy::default();
        let slept = RefCell::new(false);
        let result: Result<i32, ()> = retry_with(&p, |_| Ok(7), |_| *slept.borrow_mut() = true);
        assert_eq!(result, Ok(7));
        assert!(!*slept.borrow());
    }
}

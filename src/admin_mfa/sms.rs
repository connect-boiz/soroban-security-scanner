//! SMS-based MFA fallback for users without a hardware token or authenticator.
//!
//! Generates a short numeric OTP, delivers it through a pluggable
//! [`SmsSender`], and verifies it with an expiry and a bounded number of
//! attempts to resist brute force. Only a hash of the OTP is retained.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Default OTP length in digits.
pub const DEFAULT_OTP_DIGITS: u32 = 6;
/// Default OTP lifetime in seconds.
pub const DEFAULT_TTL_SECS: i64 = 300;
/// Default maximum verification attempts before the OTP is invalidated.
pub const DEFAULT_MAX_ATTEMPTS: u32 = 5;

/// Delivers an SMS message. Production wires this to Twilio (a `reqwest`
/// client already lives in the dependency tree); tests use a capturing double.
pub trait SmsSender: Send + Sync {
    /// Sends `message` to `phone`. Returns `Ok(())` on accepted delivery.
    fn send(&self, phone: &str, message: &str) -> Result<(), String>;
}

/// A pending SMS challenge (stores only the OTP hash).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SmsChallenge {
    /// Destination phone number.
    pub phone: String,
    otp_hash: String,
    /// Unix expiry timestamp (seconds).
    pub expires_at: i64,
    attempts: u32,
    max_attempts: u32,
}

/// Outcome of verifying an SMS OTP.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SmsVerifyResult {
    /// The OTP matched.
    Verified,
    /// Wrong code; `remaining` attempts left before lockout.
    Incorrect {
        /// Attempts remaining before the challenge is invalidated.
        remaining: u32,
    },
    /// The OTP has expired.
    Expired,
    /// Too many wrong attempts; the challenge is now invalid.
    TooManyAttempts,
}

impl SmsChallenge {
    /// Generates an OTP, sends it via `sender`, and returns the stored
    /// challenge. The plaintext OTP never leaves this function except over SMS.
    pub fn issue(
        phone: impl Into<String>,
        now: i64,
        rng: &mut impl rand::RngCore,
        sender: &dyn SmsSender,
    ) -> Result<Self, String> {
        let phone = phone.into();
        let otp = generate_otp(DEFAULT_OTP_DIGITS, rng);
        sender.send(&phone, &format!("Your verification code is {otp}"))?;
        Ok(Self {
            phone,
            otp_hash: hash(&otp),
            expires_at: now + DEFAULT_TTL_SECS,
            attempts: 0,
            max_attempts: DEFAULT_MAX_ATTEMPTS,
        })
    }

    /// Verifies a submitted code at `now`.
    pub fn verify(&mut self, code: &str, now: i64) -> SmsVerifyResult {
        if now > self.expires_at {
            return SmsVerifyResult::Expired;
        }
        if self.attempts >= self.max_attempts {
            return SmsVerifyResult::TooManyAttempts;
        }
        self.attempts += 1;
        if constant_time_eq(hash(code.trim()).as_bytes(), self.otp_hash.as_bytes()) {
            SmsVerifyResult::Verified
        } else if self.attempts >= self.max_attempts {
            SmsVerifyResult::TooManyAttempts
        } else {
            SmsVerifyResult::Incorrect {
                remaining: self.max_attempts - self.attempts,
            }
        }
    }
}

fn generate_otp(digits: u32, rng: &mut impl rand::RngCore) -> String {
    let modulo = 10u32.pow(digits);
    let value = rng.next_u32() % modulo;
    format!("{:0width$}", value, width = digits as usize)
}

fn hash(code: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(code.as_bytes());
    hex::encode(hasher.finalize())
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Captures the last sent message so the test can read the OTP.
    #[derive(Default)]
    struct CapturingSender(Mutex<Option<String>>);

    impl SmsSender for CapturingSender {
        fn send(&self, _phone: &str, message: &str) -> Result<(), String> {
            *self.0.lock().unwrap() = Some(message.to_string());
            Ok(())
        }
    }

    fn extract_otp(sender: &CapturingSender) -> String {
        let msg = sender.0.lock().unwrap().clone().unwrap();
        msg.chars().filter(|c| c.is_ascii_digit()).collect()
    }

    #[test]
    fn correct_code_verifies() {
        let mut rng = rand::rngs::OsRng;
        let sender = CapturingSender::default();
        let mut ch = SmsChallenge::issue("+15551234567", 1000, &mut rng, &sender).unwrap();
        let otp = extract_otp(&sender);
        assert_eq!(ch.verify(&otp, 1100), SmsVerifyResult::Verified);
    }

    #[test]
    fn wrong_code_decrements_attempts() {
        let mut rng = rand::rngs::OsRng;
        let sender = CapturingSender::default();
        let mut ch = SmsChallenge::issue("+15551234567", 1000, &mut rng, &sender).unwrap();
        match ch.verify("000000", 1100) {
            // Could coincidentally be correct (1-in-a-million); treat that as fine.
            SmsVerifyResult::Verified => {}
            SmsVerifyResult::Incorrect { remaining } => {
                assert_eq!(remaining, DEFAULT_MAX_ATTEMPTS - 1)
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn expired_code_is_rejected() {
        let mut rng = rand::rngs::OsRng;
        let sender = CapturingSender::default();
        let mut ch = SmsChallenge::issue("+15551234567", 1000, &mut rng, &sender).unwrap();
        let otp = extract_otp(&sender);
        assert_eq!(
            ch.verify(&otp, 1000 + DEFAULT_TTL_SECS + 1),
            SmsVerifyResult::Expired
        );
    }

    #[test]
    fn lockout_after_max_attempts() {
        let mut rng = rand::rngs::OsRng;
        let sender = CapturingSender::default();
        let mut ch = SmsChallenge::issue("+15551234567", 1000, &mut rng, &sender).unwrap();
        // Use an obviously-wrong code repeatedly. Use a value unlikely to match.
        let wrong = "999999";
        let real = extract_otp(&sender);
        let wrong = if wrong == real { "111111" } else { wrong };
        for _ in 0..DEFAULT_MAX_ATTEMPTS {
            let _ = ch.verify(wrong, 1100);
        }
        assert_eq!(ch.verify(wrong, 1100), SmsVerifyResult::TooManyAttempts);
    }

    #[test]
    fn otp_plaintext_is_not_stored() {
        let mut rng = rand::rngs::OsRng;
        let sender = CapturingSender::default();
        let ch = SmsChallenge::issue("+15551234567", 1000, &mut rng, &sender).unwrap();
        let otp = extract_otp(&sender);
        let json = serde_json::to_string(&ch).unwrap();
        assert!(!json.contains(&otp));
    }
}

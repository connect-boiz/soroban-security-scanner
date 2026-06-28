//! TOTP (Time-based One-Time Password) per RFC 6238 / HOTP per RFC 4226.
//!
//! Uses HMAC-SHA1 (the algorithm understood by every mainstream authenticator
//! app) via `ring`, with a configurable digit count, time step and skew
//! window. Verification is anti-replay aware: a verified counter is reported so
//! the caller can refuse to honour the same step twice.

use crate::admin_mfa::base32;
use ring::hmac;
use serde::{Deserialize, Serialize};

/// Default number of digits in a generated code.
pub const DEFAULT_DIGITS: u32 = 6;
/// Default time step in seconds.
pub const DEFAULT_STEP_SECS: u64 = 30;
/// Default verification skew window (steps checked on each side of "now").
pub const DEFAULT_SKEW_STEPS: u64 = 1;
/// Recommended shared-secret length in bytes (160-bit, per RFC 4226).
pub const SECRET_LEN: usize = 20;

/// A TOTP configuration bound to a shared secret.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TotpConfig {
    /// Raw shared secret bytes.
    pub secret: Vec<u8>,
    /// Number of digits per code.
    pub digits: u32,
    /// Time step in seconds.
    pub step_secs: u64,
    /// Number of steps tolerated on each side of the current step.
    pub skew_steps: u64,
}

impl TotpConfig {
    /// Builds a config from raw secret bytes with default parameters.
    pub fn from_secret(secret: Vec<u8>) -> Self {
        Self {
            secret,
            digits: DEFAULT_DIGITS,
            step_secs: DEFAULT_STEP_SECS,
            skew_steps: DEFAULT_SKEW_STEPS,
        }
    }

    /// Generates a fresh random-secret config using the supplied RNG.
    ///
    /// The RNG is injected so callers can use a CSPRNG in production and a
    /// deterministic source in tests.
    pub fn generate(rng: &mut impl rand::RngCore) -> Self {
        let mut secret = vec![0u8; SECRET_LEN];
        rng.fill_bytes(&mut secret);
        Self::from_secret(secret)
    }

    /// Returns the secret as a base32 string for manual entry.
    pub fn secret_base32(&self) -> String {
        base32::encode(&self.secret)
    }

    /// Builds an `otpauth://` provisioning URI for QR-code enrollment.
    pub fn provisioning_uri(&self, issuer: &str, account: &str) -> String {
        let label = format!("{issuer}:{account}");
        format!(
            "otpauth://totp/{}?secret={}&issuer={}&algorithm=SHA1&digits={}&period={}",
            urlencode(&label),
            self.secret_base32(),
            urlencode(issuer),
            self.digits,
            self.step_secs
        )
    }

    /// Computes the code for an explicit unix timestamp (seconds).
    pub fn code_at(&self, unix_secs: u64) -> String {
        let counter = unix_secs / self.step_secs;
        let value = hotp(&self.secret, counter, self.digits);
        zero_pad(value, self.digits)
    }

    /// Verifies a code at `unix_secs`, scanning the skew window.
    ///
    /// Returns the matching counter on success so the caller can persist it and
    /// reject replays of the same or earlier step.
    pub fn verify_at(&self, code: &str, unix_secs: u64) -> Option<u64> {
        let trimmed = code.trim();
        if trimmed.len() != self.digits as usize || !trimmed.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        let center = unix_secs / self.step_secs;
        let skew = self.skew_steps;
        // Iterate oldest→newest within the window.
        for counter in center.saturating_sub(skew)..=center + skew {
            let candidate = zero_pad(hotp(&self.secret, counter, self.digits), self.digits);
            if constant_time_eq(candidate.as_bytes(), trimmed.as_bytes()) {
                return Some(counter);
            }
        }
        None
    }
}

/// RFC 4226 HOTP value for a counter.
fn hotp(secret: &[u8], counter: u64, digits: u32) -> u32 {
    let key = hmac::Key::new(hmac::HMAC_SHA1_FOR_LEGACY_USE_ONLY, secret);
    let tag = hmac::sign(&key, &counter.to_be_bytes());
    let hs = tag.as_ref();
    let offset = (hs[hs.len() - 1] & 0x0f) as usize;
    let bin = (((hs[offset] & 0x7f) as u32) << 24)
        | ((hs[offset + 1] as u32) << 16)
        | ((hs[offset + 2] as u32) << 8)
        | (hs[offset + 3] as u32);
    bin % 10u32.pow(digits)
}

fn zero_pad(value: u32, digits: u32) -> String {
    format!("{:0width$}", value, width = digits as usize)
}

/// Constant-time byte comparison to avoid timing oracles on code checks.
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

/// Percent-encodes the characters that matter inside otpauth labels.
fn urlencode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for b in input.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RFC 6238 Appendix B test vector secret ("12345678901234567890").
    fn rfc_secret() -> Vec<u8> {
        b"12345678901234567890".to_vec()
    }

    #[test]
    fn rfc6238_test_vectors_sha1() {
        // 8-digit codes at the documented timestamps.
        let cfg = TotpConfig {
            secret: rfc_secret(),
            digits: 8,
            step_secs: 30,
            skew_steps: 0,
        };
        assert_eq!(cfg.code_at(59), "94287082");
        assert_eq!(cfg.code_at(1111111109), "07081804");
        assert_eq!(cfg.code_at(1234567890), "89005924");
    }

    #[test]
    fn verify_accepts_current_code() {
        let cfg = TotpConfig::from_secret(rfc_secret());
        let now = 1_700_000_000;
        let code = cfg.code_at(now);
        assert!(cfg.verify_at(&code, now).is_some());
    }

    #[test]
    fn verify_honours_skew_window() {
        let cfg = TotpConfig::from_secret(rfc_secret()); // skew = 1 step
        let now = 1_700_000_000;
        let prev_step = now - DEFAULT_STEP_SECS as u64;
        let code = cfg.code_at(prev_step);
        // A code from the previous step is still accepted within skew.
        assert!(cfg.verify_at(&code, now).is_some());
        // Two steps in the past is rejected.
        let old = cfg.code_at(now - 2 * DEFAULT_STEP_SECS as u64);
        assert!(cfg.verify_at(&old, now).is_none());
    }

    #[test]
    fn verify_rejects_malformed() {
        let cfg = TotpConfig::from_secret(rfc_secret());
        assert!(cfg.verify_at("12345", 0).is_none()); // too short
        assert!(cfg.verify_at("abcdef", 0).is_none()); // non-digit
    }

    #[test]
    fn verify_returns_counter_for_replay_detection() {
        let cfg = TotpConfig::from_secret(rfc_secret());
        let now = 1_700_000_000;
        let counter = cfg.verify_at(&cfg.code_at(now), now).unwrap();
        assert_eq!(counter, now / DEFAULT_STEP_SECS as u64);
    }

    #[test]
    fn provisioning_uri_is_well_formed() {
        let cfg = TotpConfig::from_secret(rfc_secret());
        let uri = cfg.provisioning_uri("Soroban Scanner", "admin@example.com");
        assert!(uri.starts_with("otpauth://totp/"));
        assert!(uri.contains("secret="));
        assert!(uri.contains("algorithm=SHA1"));
        assert!(uri.contains("digits=6"));
        assert!(uri.contains("period=30"));
        // Spaces and @ are percent-encoded.
        assert!(!uri.contains(' '));
    }

    #[test]
    fn generate_produces_verifiable_secret() {
        let mut rng = rand::rngs::OsRng;
        let cfg = TotpConfig::generate(&mut rng);
        assert_eq!(cfg.secret.len(), SECRET_LEN);
        let now = 1_700_000_000;
        assert!(cfg.verify_at(&cfg.code_at(now), now).is_some());
    }
}

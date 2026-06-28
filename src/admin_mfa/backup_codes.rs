//! Single-use recovery backup codes.
//!
//! Codes are shown to the user once at generation and only their SHA-256
//! hashes are retained, so a database leak does not expose usable codes. Each
//! code can be redeemed exactly once.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Default number of backup codes issued per generation.
pub const DEFAULT_COUNT: usize = 10;
/// Number of random bytes behind each code (→ 16 hex chars).
const CODE_BYTES: usize = 8;

/// Plaintext codes returned to the user once, alongside the storable set.
#[derive(Debug, Clone)]
pub struct GeneratedBackupCodes {
    /// Human-readable codes to display exactly once.
    pub plaintext: Vec<String>,
    /// Hashed, storable representation.
    pub store: BackupCodeSet,
}

/// A storable set of hashed backup codes.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupCodeSet {
    /// SHA-256 hex digests of unused codes.
    hashes: Vec<String>,
    /// Count of codes consumed so far.
    used: usize,
}

impl BackupCodeSet {
    /// Generates `count` fresh codes using the supplied RNG, returning both the
    /// plaintext (to show once) and the hashed set (to store).
    pub fn generate(count: usize, rng: &mut impl rand::RngCore) -> GeneratedBackupCodes {
        let mut plaintext = Vec::with_capacity(count);
        let mut hashes = Vec::with_capacity(count);
        for _ in 0..count {
            let mut bytes = [0u8; CODE_BYTES];
            rng.fill_bytes(&mut bytes);
            let code = format_code(&bytes);
            hashes.push(hash(&code));
            plaintext.push(code);
        }
        GeneratedBackupCodes {
            plaintext,
            store: BackupCodeSet { hashes, used: 0 },
        }
    }

    /// Number of unused codes remaining.
    pub fn remaining(&self) -> usize {
        self.hashes.len()
    }

    /// Number of codes consumed.
    pub fn used(&self) -> usize {
        self.used
    }

    /// Redeems a code, consuming it on success. Returns `true` if the code was
    /// valid and previously unused.
    pub fn redeem(&mut self, code: &str) -> bool {
        let target = hash(code.trim());
        if let Some(pos) = self
            .hashes
            .iter()
            .position(|h| constant_time_eq(h.as_bytes(), target.as_bytes()))
        {
            self.hashes.remove(pos);
            self.used += 1;
            true
        } else {
            false
        }
    }
}

fn format_code(bytes: &[u8]) -> String {
    // Group as XXXX-XXXX-XXXX-XXXX for readability.
    let hexed: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    hexed
        .as_bytes()
        .chunks(4)
        .map(|c| std::str::from_utf8(c).unwrap())
        .collect::<Vec<_>>()
        .join("-")
}

fn hash(code: &str) -> String {
    // Normalise away the readability dashes/case before hashing.
    let normalized: String = code
        .chars()
        .filter(|c| *c != '-')
        .collect::<String>()
        .to_lowercase();
    let mut hasher = Sha256::new();
    hasher.update(normalized.as_bytes());
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

    #[test]
    fn generates_requested_count() {
        let mut rng = rand::rngs::OsRng;
        let gen = BackupCodeSet::generate(DEFAULT_COUNT, &mut rng);
        assert_eq!(gen.plaintext.len(), DEFAULT_COUNT);
        assert_eq!(gen.store.remaining(), DEFAULT_COUNT);
    }

    #[test]
    fn code_redeems_once() {
        let mut rng = rand::rngs::OsRng;
        let gen = BackupCodeSet::generate(3, &mut rng);
        let mut set = gen.store;
        let code = &gen.plaintext[0];
        assert!(set.redeem(code));
        assert_eq!(set.remaining(), 2);
        assert_eq!(set.used(), 1);
        // Second attempt with the same code fails.
        assert!(!set.redeem(code));
    }

    #[test]
    fn redeem_ignores_dashes_and_case() {
        let mut rng = rand::rngs::OsRng;
        let gen = BackupCodeSet::generate(1, &mut rng);
        let mut set = gen.store;
        let stripped = gen.plaintext[0].replace('-', "").to_uppercase();
        assert!(set.redeem(&stripped));
    }

    #[test]
    fn unknown_code_is_rejected() {
        let mut rng = rand::rngs::OsRng;
        let mut set = BackupCodeSet::generate(2, &mut rng).store;
        assert!(!set.redeem("0000-0000-0000-0000"));
        assert_eq!(set.remaining(), 2);
    }

    #[test]
    fn only_hashes_are_stored() {
        let mut rng = rand::rngs::OsRng;
        let gen = BackupCodeSet::generate(1, &mut rng);
        // The plaintext must not appear in the serialized store.
        let json = serde_json::to_string(&gen.store).unwrap();
        assert!(!json.contains(&gen.plaintext[0].replace('-', "")));
    }
}

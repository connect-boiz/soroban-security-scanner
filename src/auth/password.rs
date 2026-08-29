use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2, Params,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PasswordError {
    #[error("Password hashing failed: {0}")]
    HashingError(String),
    #[error("Password verification failed: {0}")]
    VerificationError(String),
    #[error("Invalid password format: {0}")]
    InvalidFormat(String),
    #[error("Password too weak: {0}")]
    WeakPassword(String),
}

#[derive(Debug, Clone)]
pub struct PasswordConfig {
    pub salt_length: usize,
    pub hash_length: usize,
    pub time_cost: u32,
    pub memory_cost: u32,
    pub parallelism: u32,
    pub version: argon2::Version,
}

impl Default for PasswordConfig {
    fn default() -> Self {
        Self {
            salt_length: 16,
            hash_length: 32,
            time_cost: 3,
            memory_cost: 65536, // 64 MB
            parallelism: 4,
            version: argon2::Version::V0x13,
        }
    }
}

impl PasswordConfig {
    pub fn high_security() -> Self {
        Self {
            salt_length: 32,
            hash_length: 64,
            time_cost: 4,
            memory_cost: 131072, // 128 MB
            parallelism: 8,
            version: argon2::Version::V0x13,
        }
    }

    pub fn low_memory() -> Self {
        Self {
            salt_length: 16,
            hash_length: 32,
            time_cost: 2,
            memory_cost: 32768, // 32 MB
            parallelism: 2,
            version: argon2::Version::V0x13,
        }
    }
}

pub struct PasswordService {
    config: PasswordConfig,
}

impl Default for PasswordService {
    fn default() -> Self {
        Self::new(PasswordConfig::default())
    }
}

impl PasswordService {
    pub fn new(config: PasswordConfig) -> Self {
        Self { config }
    }

    pub fn hash_password(&self, password: &str) -> Result<String, PasswordError> {
        if password.is_empty() {
            return Err(PasswordError::WeakPassword(
                "Password cannot be empty".to_string(),
            ));
        }

        // Enforce minimum password strength before hashing.
        // Weak passwords (score 0-2) are rejected to prevent credential
        // stuffing and brute-force attacks on stored hashes.
        match self.check_password_strength(password)? {
            PasswordStrength::Weak => {
                return Err(PasswordError::WeakPassword(
                    "Password does not meet the minimum strength requirements. \
                     Use at least 8 characters and include uppercase letters, \
                     lowercase letters, numbers, and special characters."
                        .to_string(),
                ));
            }
            // Medium, Strong, and VeryStrong are accepted.
            _ => {}
        }

        let salt = SaltString::generate(&mut OsRng);

        let params = Params::new(
            self.config.memory_cost,
            self.config.time_cost,
            self.config.parallelism,
            Some(self.config.hash_length),
        )
        .map_err(|e| PasswordError::HashingError(e.to_string()))?;

        let argon2 = Argon2::new(argon2::Algorithm::Argon2id, self.config.version, params);

        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| PasswordError::HashingError(e.to_string()))?;

        Ok(password_hash.to_string())
    }

    pub fn verify_password(&self, password: &str, hash: &str) -> Result<bool, PasswordError> {
        if password.is_empty() {
            return Err(PasswordError::WeakPassword(
                "Password cannot be empty".to_string(),
            ));
        }

        let parsed_hash =
            PasswordHash::new(hash).map_err(|e| PasswordError::InvalidFormat(e.to_string()))?;

        let params = Params::new(
            self.config.memory_cost,
            self.config.time_cost,
            self.config.parallelism,
            Some(self.config.hash_length),
        )
        .map_err(|e| PasswordError::VerificationError(e.to_string()))?;

        let argon2 = Argon2::new(argon2::Algorithm::Argon2id, self.config.version, params);

        match argon2.verify_password(password.as_bytes(), &parsed_hash) {
            Ok(()) => Ok(true),
            Err(argon2::password_hash::Error::Password) => Ok(false),
            Err(e) => Err(PasswordError::VerificationError(e.to_string())),
        }
    }

    pub fn check_password_strength(
        &self,
        password: &str,
    ) -> Result<PasswordStrength, PasswordError> {
        if password.is_empty() {
            return Err(PasswordError::WeakPassword(
                "Password cannot be empty".to_string(),
            ));
        }

        let mut score: u32 = 0;
        let mut feedback = Vec::new();

        // Length check
        if password.len() >= 20 {
            score += 4;
        } else if password.len() >= 12 {
            score += 2;
        } else if password.len() >= 8 {
            score += 1;
        } else {
            feedback.push("Password should be at least 8 characters long".to_string());
        }

        // Character variety checks
        if password.chars().any(|c| c.is_uppercase()) {
            score += 1;
        } else {
            feedback.push("Include uppercase letters".to_string());
        }

        if password.chars().any(|c| c.is_lowercase()) {
            score += 1;
        } else {
            feedback.push("Include lowercase letters".to_string());
        }

        if password.chars().any(|c| c.is_numeric()) {
            score += 1;
        } else {
            feedback.push("Include numbers".to_string());
        }

        if password.chars().any(|c| !c.is_alphanumeric()) {
            score += 2;
        } else {
            feedback.push("Include special characters".to_string());
        }

        // Common patterns penalty
        if self.contains_common_patterns(password) {
            score = score.saturating_sub(2);
            feedback.push("Avoid common patterns or dictionary words".to_string());
        }

        let strength = match score {
            0..=2 => PasswordStrength::Weak,
            3..=5 => PasswordStrength::Medium,
            6..=8 => PasswordStrength::Strong,
            _ => PasswordStrength::VeryStrong,
        };

        Ok(strength)
    }

    fn contains_common_patterns(&self, password: &str) -> bool {
        let common_patterns = vec![
            "123456",
            "password",
            "qwerty",
            "abc123",
            "111111",
            "12345678",
            "admin",
            "letmein",
            "welcome",
            "monkey",
            "1234567890",
            "123123",
        ];

        let password_lower = password.to_lowercase();

        // Check for common patterns
        for pattern in &common_patterns {
            if password_lower.contains(pattern) {
                return true;
            }
        }

        // Check for sequential characters
        let chars: Vec<char> = password_lower.chars().collect();
        if chars.len() >= 3 {
            for i in 0..chars.len() - 2 {
                let current = chars[i] as u8;
                let next = chars[i + 1] as u8;
                let next_next = chars[i + 2] as u8;

                if next == current + 1 && next_next == next + 1 {
                    return true;
                }
                if next == current - 1 && next_next == next - 1 {
                    return true;
                }
            }
        }

        // Check for repeated characters
        if chars.len() >= 3 {
            for i in 0..chars.len() - 2 {
                if chars[i] == chars[i + 1] && chars[i + 1] == chars[i + 2] {
                    return true;
                }
            }
        }

        false
    }

    pub fn generate_secure_password(&self, length: usize) -> String {
        use rand::Rng;

        let charset = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!@#$%^&*()_+-=[]{}|;:,.<>?";
        let mut rng = rand::thread_rng();

        (0..length)
            .map(|_| {
                charset
                    .chars()
                    .nth(rng.gen_range(0..charset.len()))
                    .unwrap()
            })
            .collect()
    }

    pub fn needs_rehash(&self, hash: &str) -> bool {
        let parsed_hash = match PasswordHash::new(hash) {
            Ok(h) => h,
            Err(_) => return true, // Malformed PHC string – must rehash.
        };

        // Reject any algorithm other than Argon2id.
        match argon2::Algorithm::try_from(parsed_hash.algorithm) {
            Ok(argon2::Algorithm::Argon2id) => {}
            _ => return true,
        }

        // Check the version tag embedded in the PHC string.
        // V0x13 = version 19 (0x13).  If the stored hash was produced with an
        // older version it must be re-hashed.
        if let Some(stored_version) = parsed_hash.version {
            let current_version: u32 = match self.config.version {
                argon2::Version::V0x10 => 16,
                argon2::Version::V0x13 => 19,
            };
            if stored_version != current_version {
                return true;
            }
        }

        // Extract the stored Argon2 cost parameters from the PHC string and
        // compare them against the service's current configuration.  Any
        // mismatch means the hash was produced under weaker (or simply
        // different) settings and should be upgraded on the next successful
        // login.
        match Params::try_from(&parsed_hash) {
            Ok(stored_params) => {
                stored_params.m_cost() != self.config.memory_cost
                    || stored_params.t_cost() != self.config.time_cost
                    || stored_params.p_cost() != self.config.parallelism
                    || stored_params.output_len() != Some(self.config.hash_length)
            }
            // Could not decode params from the stored hash – safest to rehash.
            Err(_) => true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PasswordStrength {
    Weak,
    Medium,
    Strong,
    VeryStrong,
}

impl PasswordStrength {
    pub fn as_str(&self) -> &'static str {
        match self {
            PasswordStrength::Weak => "Weak",
            PasswordStrength::Medium => "Medium",
            PasswordStrength::Strong => "Strong",
            PasswordStrength::VeryStrong => "Very Strong",
        }
    }

    pub fn score(&self) -> u8 {
        match self {
            PasswordStrength::Weak => 1,
            PasswordStrength::Medium => 2,
            PasswordStrength::Strong => 3,
            PasswordStrength::VeryStrong => 4,
        }
    }

    pub fn color(&self) -> &'static str {
        match self {
            PasswordStrength::Weak => "red",
            PasswordStrength::Medium => "orange",
            PasswordStrength::Strong => "yellow",
            PasswordStrength::VeryStrong => "green",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_hashing_and_verification() {
        let service = PasswordService::default();
        let password = "test_password_123!";

        let hash = service.hash_password(password).unwrap();
        assert!(hash.starts_with("$argon2id$"));

        let is_valid = service.verify_password(password, &hash).unwrap();
        assert!(is_valid);

        let is_invalid = service.verify_password("wrong_password", &hash).unwrap();
        assert!(!is_invalid);
    }

    #[test]
    fn test_password_strength() {
        let service = PasswordService::default();

        // Weak password
        let weak = service.check_password_strength("123").unwrap();
        assert_eq!(weak, PasswordStrength::Weak);

        // Medium password
        let medium = service.check_password_strength("Passw0rdX9").unwrap();
        assert_eq!(medium, PasswordStrength::Medium);

        // Strong password
        let strong = service.check_password_strength("Str0ngP@ssw0rd!").unwrap();
        assert_eq!(strong, PasswordStrength::Strong);

        // Very strong password
        let very_strong = service
            .check_password_strength("V3ry$tr0ng&P@ssw0rd!2024#")
            .unwrap();
        assert_eq!(very_strong, PasswordStrength::VeryStrong);
    }

    #[test]
    fn test_common_patterns_detection() {
        let service = PasswordService::default();

        assert!(service.contains_common_patterns("password123"));
        assert!(service.contains_common_patterns("qwerty"));
        assert!(service.contains_common_patterns("abc123"));
        assert!(service.contains_common_patterns("123456"));
        assert!(!service.contains_common_patterns("R@nd0mP@ss!"));
    }

    #[test]
    fn test_secure_password_generation() {
        let service = PasswordService::default();
        let password = service.generate_secure_password(16);

        assert_eq!(password.len(), 16);
        assert!(password.chars().any(|c| c.is_uppercase()));
        assert!(password.chars().any(|c| c.is_lowercase()));
        assert!(password.chars().any(|c| c.is_numeric()));
        assert!(password.chars().any(|c| !c.is_alphanumeric()));
    }

    #[test]
    fn test_different_configurations() {
        let default_config = PasswordService::default();
        let high_security = PasswordService::new(PasswordConfig::high_security());
        let low_memory = PasswordService::new(PasswordConfig::low_memory());

        let password = "test_password_123!";

        let default_hash = default_config.hash_password(password).unwrap();
        let high_security_hash = high_security.hash_password(password).unwrap();
        let low_memory_hash = low_memory.hash_password(password).unwrap();

        // All should verify correctly
        assert!(default_config
            .verify_password(password, &default_hash)
            .unwrap());
        assert!(high_security
            .verify_password(password, &high_security_hash)
            .unwrap());
        assert!(low_memory
            .verify_password(password, &low_memory_hash)
            .unwrap());

        // Hashes should be different due to different parameters
        assert_ne!(default_hash, high_security_hash);
        assert_ne!(default_hash, low_memory_hash);
    }

    #[test]
    fn test_empty_password_handling() {
        let service = PasswordService::default();

        assert!(service.hash_password("").is_err());
        assert!(service.verify_password("", "some_hash").is_err());
        assert!(service.check_password_strength("").is_err());
    }

    // -------------------------------------------------------------------------
    // Password strength enforcement at hash time
    // -------------------------------------------------------------------------

    /// Weak passwords must be rejected by hash_password(), not silently stored.
    #[test]
    fn test_hash_password_rejects_weak_passwords() {
        let service = PasswordService::default();

        // Single character – definitely Weak.
        let result = service.hash_password("a");
        assert!(
            result.is_err(),
            "hash_password() must reject a single-character password"
        );
        assert!(matches!(
            result.unwrap_err(),
            PasswordError::WeakPassword(_)
        ));

        // Short numeric-only – Weak.
        let result = service.hash_password("123");
        assert!(
            result.is_err(),
            "hash_password() must reject a numeric-only short password"
        );
        assert!(matches!(
            result.unwrap_err(),
            PasswordError::WeakPassword(_)
        ));
    }

    /// Medium (and above) passwords must be accepted by hash_password().
    #[test]
    fn test_hash_password_accepts_medium_and_stronger_passwords() {
        let service = PasswordService::default();

        // Medium – should succeed.
        assert!(
            service.hash_password("Passw0rdX9").is_ok(),
            "hash_password() must accept a Medium-strength password"
        );

        // Strong – should succeed.
        assert!(
            service.hash_password("Str0ngP@ssw0rd!").is_ok(),
            "hash_password() must accept a Strong password"
        );

        // Very strong – should succeed.
        assert!(
            service.hash_password("V3ry$tr0ng&P@ssw0rd!2024#").is_ok(),
            "hash_password() must accept a VeryStrong password"
        );
    }

    // -------------------------------------------------------------------------
    // needs_rehash() – real parameter comparison
    // -------------------------------------------------------------------------

    /// A hash produced with the default config should not need rehashing when
    /// checked against the same config.
    #[test]
    fn test_needs_rehash_same_config_returns_false() {
        let service = PasswordService::new(PasswordConfig::default());
        let hash = service.hash_password("Str0ngP@ssw0rd!").unwrap();

        assert!(
            !service.needs_rehash(&hash),
            "needs_rehash() must return false when params match the current config"
        );
    }

    /// A hash produced with the default config should need rehashing when
    /// evaluated against a service configured with higher security parameters.
    #[test]
    fn test_needs_rehash_different_config_returns_true() {
        let default_service = PasswordService::new(PasswordConfig::default());
        let hash = default_service.hash_password("Str0ngP@ssw0rd!").unwrap();

        // high_security uses different m_cost, t_cost, p_cost, and output_len.
        let high_security_service = PasswordService::new(PasswordConfig::high_security());
        assert!(
            high_security_service.needs_rehash(&hash),
            "needs_rehash() must return true when stored params differ from current config"
        );
    }

    /// A hash produced with high_security config should not need rehashing when
    /// checked against the same high_security config.
    #[test]
    fn test_needs_rehash_high_security_same_config_returns_false() {
        let service = PasswordService::new(PasswordConfig::high_security());
        let hash = service.hash_password("V3ry$tr0ng&P@ssw0rd!2024#").unwrap();

        assert!(
            !service.needs_rehash(&hash),
            "needs_rehash() must return false for a hash that matches the high-security config"
        );
    }

    /// An entirely invalid hash string must trigger a rehash.
    #[test]
    fn test_needs_rehash_invalid_hash_returns_true() {
        let service = PasswordService::default();
        assert!(
            service.needs_rehash("not-a-valid-phc-hash"),
            "needs_rehash() must return true for a malformed hash"
        );
    }
}

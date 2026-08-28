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
        use rand::seq::SliceRandom;
        use rand::Rng;

        let lowercase = "abcdefghijklmnopqrstuvwxyz";
        let uppercase = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
        let digits = "0123456789";
        let symbols = "!@#$%^&*()_+-=[]{}|;:,.<>?";
        let charset: Vec<char> = [lowercase, uppercase, digits, symbols]
            .concat()
            .chars()
            .collect();
        let mut rng = rand::thread_rng();

        let len = length.max(4);
        let mut password = vec![
            lowercase
                .chars()
                .nth(rng.gen_range(0..lowercase.len()))
                .unwrap(),
            uppercase
                .chars()
                .nth(rng.gen_range(0..uppercase.len()))
                .unwrap(),
            digits.chars().nth(rng.gen_range(0..digits.len())).unwrap(),
            symbols
                .chars()
                .nth(rng.gen_range(0..symbols.len()))
                .unwrap(),
        ];
        password.extend((0..len - 4).map(|_| charset[rng.gen_range(0..charset.len())]));
        password.shuffle(&mut rng);
        password.truncate(length);
        password.into_iter().collect()
    }

    pub fn needs_rehash(&self, hash: &str) -> bool {
        let parsed_hash = match PasswordHash::new(hash) {
            Ok(hash) => hash,
            Err(_) => return true, // Invalid format, needs rehash
        };

        // Check if the hash uses the current algorithm and parameters
        match argon2::Algorithm::try_from(parsed_hash.algorithm) {
            Ok(argon2::Algorithm::Argon2id) => {
                // This is a simplified check - in production, you'd want more detailed comparison
                parsed_hash.params.is_empty() // No params found => needs rehash
            }
            _ => true, // Different algorithm, needs rehash
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
}

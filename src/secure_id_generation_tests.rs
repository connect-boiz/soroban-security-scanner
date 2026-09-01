//! Tests for secure ID generation functionality

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_secure_id_config_default() {
        let config = SecureIdConfig::default();
        
        assert!(config.enabled);
        assert_eq!(config.id_length_bytes, 32);
        assert!(config.enable_collision_detection);
        assert_eq!(config.max_collision_retries, 100);
        assert!(config.enable_uniqueness_validation);
        assert_eq!(config.id_cache_size, 1000);
    }

    #[test]
    fn test_secure_id_generator_creation() {
        let config = SecureIdConfig::default();
        let generator = SecureIdGenerator::new(config);
        
        let stats = generator.get_statistics().unwrap();
        assert_eq!(stats.total_ids_generated, 0);
        assert!(stats.collision_detection_enabled);
        assert!(stats.uniqueness_validation_enabled);
    }

    #[test]
    fn test_bounty_id_generation() {
        let config = SecureIdConfig::default();
        let generator = SecureIdGenerator::new(config);

        let creator = "test_creator";
        let timestamp = 1234567890;

        let id1 = generator.generate_bounty_id(creator, timestamp).unwrap();
        let id2 = generator.generate_bounty_id(creator, timestamp + 1).unwrap();

        assert_ne!(id1, id2); // Should be different due to different timestamp
        
        // Test uniqueness validation
        assert!(generator.validate_id_uniqueness(id1, "bounty").unwrap());
    }

    #[test]
    fn test_secure_nonce_generation() {
        let config = SecureIdConfig::default();
        let generator = SecureIdGenerator::new(config);

        let nonce1 = generator.generate_secure_nonce("user1", "operation1").unwrap();
        let nonce2 = generator.generate_secure_nonce("user1", "operation2").unwrap();
        let nonce3 = generator.generate_secure_nonce("user2", "operation1").unwrap();

        assert_ne!(nonce1, nonce2); // Different operations should produce different nonces
        assert_ne!(nonce1, nonce3); // Different users should produce different nonces
        assert_eq!(nonce1.len(), 32);
        assert_eq!(nonce2.len(), 32);
        assert_eq!(nonce3.len(), 32);
    }

    #[test]
    fn test_transaction_id_generation() {
        let config = SecureIdConfig::default();
        let generator = SecureIdGenerator::new(config);

        let tx_id1 = generator.generate_transaction_id("alice", "bob", 1000).unwrap();
        let tx_id2 = generator.generate_transaction_id("alice", "bob", 2000).unwrap();
        let tx_id3 = generator.generate_transaction_id("alice", "charlie", 1000).unwrap();

        assert_ne!(tx_id1, tx_id2); // Different amounts should produce different IDs
        assert_ne!(tx_id1, tx_id3); // Different recipients should produce different IDs
        assert!(tx_id1.starts_with("tx_"));
        assert!(tx_id2.starts_with("tx_"));
        assert!(tx_id3.starts_with("tx_"));
    }

    #[test]
    fn test_session_id_generation() {
        let config = SecureIdConfig::default();
        let generator = SecureIdGenerator::new(config);

        let session_id1 = generator.generate_session_id("user123").unwrap();
        let session_id2 = generator.generate_session_id("user456").unwrap();

        assert_ne!(session_id1, session_id2); // Different users should produce different session IDs
        assert!(session_id1.starts_with("session_"));
        assert!(session_id2.starts_with("session_"));
        assert!(session_id1.len() > 20);
        assert!(session_id2.len() > 20);
    }

    #[test]
    fn test_collision_detection() {
        let mut config = SecureIdConfig::default();
        config.enable_collision_detection = true;
        let generator = SecureIdGenerator::new(config);

        let creator = "test_creator";
        let timestamp = 1234567890;

        let id1 = generator.generate_bounty_id(creator, timestamp).unwrap();
        
        // Second generation with same parameters should still work due to additional entropy
        let id2 = generator.generate_bounty_id(creator, timestamp).unwrap();
        
        // They should be different due to random entropy
        assert_ne!(id1, id2);
        
        // But both should be valid
        assert!(generator.validate_id_uniqueness(id1, "bounty").unwrap());
        assert!(generator.validate_id_uniqueness(id2, "bounty").unwrap());
    }

    #[test]
    fn test_entropy_sources() {
        let sources = vec![
            EntropySource::MultipleSources,
            EntropySource::LedgerOnly,
            EntropySource::ExternalService,
            EntropySource::UserProvided,
        ];

        for source in sources {
            let mut config = SecureIdConfig::default();
            config.entropy_source = source.clone();
            let generator = SecureIdGenerator::new(config);

            let result = generator.generate_bounty_id("test", 1234567890);
            assert!(result.is_ok(), "Failed with entropy source: {:?}", source);
            
            let stats = generator.get_statistics().unwrap();
            assert_eq!(stats.entropy_source, format!("{:?}", source));
        }
    }

    #[test]
    fn test_id_generation_methods() {
        let methods = vec![
            IdGenerationMethod::CryptographicHash,
            IdGenerationMethod::HardwareRng,
            IdGenerationMethod::CombinedEntropy,
            IdGenerationMethod::TimestampWithEntropy,
        ];

        for method in methods {
            let mut config = SecureIdConfig::default();
            config.method = method.clone();
            let generator = SecureIdGenerator::new(config);

            let result = generator.generate_bounty_id("test", 1234567890);
            assert!(result.is_ok(), "Failed with method: {:?}", method);
            
            let stats = generator.get_statistics().unwrap();
            assert_eq!(stats.generation_method, format!("{:?}", method));
        }
    }

    #[test]
    fn test_concurrent_id_generation() {
        let config = SecureIdConfig::default();
        let generator = Arc::new(SecureIdGenerator::new(config));

        let mut handles = vec![];

        for i in 0..20 {
            let gen_clone = generator.clone();
            let handle = thread::spawn(move || {
                gen_clone.generate_bounty_id(&format!("user{}", i), 1234567890 + i)
            });
            handles.push(handle);
        }

        let mut ids = vec![];
        for handle in handles {
            ids.push(handle.join().unwrap());
        }

        // All IDs should be unique
        let mut unique_ids = HashSet::new();
        for id in ids {
            assert!(!unique_ids.contains(&id), "Duplicate ID detected: {}", id);
            unique_ids.insert(id);
        }
        
        assert_eq!(unique_ids.len(), 20); // All 20 IDs should be unique
    }

    #[test]
    fn test_secure_id_builder() {
        let config = SecureIdConfig::default();
        let generator = Arc::new(SecureIdGenerator::new(config));
        let builder = SecureIdBuilder::new(generator.clone(), "test".to_string())
            .metadata("key1".to_string(), "value1".to_string())
            .metadata("key2".to_string(), "value2".to_string());

        let bounty_id = builder.generate_bounty_id("creator", 1234567890).unwrap();
        assert!(bounty_id > 0);

        let nonce = builder.generate_nonce("user", "operation").unwrap();
        assert_eq!(nonce.len(), 32);

        let tx_id = builder.generate_transaction_id("alice", "bob", 1000).unwrap();
        assert!(tx_id.starts_with("tx_"));
    }

    #[test]
    fn test_id_statistics() {
        let config = SecureIdConfig::default();
        let generator = SecureIdGenerator::new(config);

        // Generate some IDs
        for i in 0..10 {
            generator.generate_bounty_id(&format!("user{}", i), 1234567890 + i).unwrap();
        }

        let stats = generator.get_statistics().unwrap();
        assert_eq!(stats.total_ids_generated, 10);
        assert!(stats.collision_detection_enabled);
        assert!(stats.uniqueness_validation_enabled);
        assert_eq!(stats.cache_size, 1000);
    }

    #[test]
    fn test_collision_detection_disabled() {
        let mut config = SecureIdConfig::default();
        config.enable_collision_detection = false;
        let generator = SecureIdGenerator::new(config);

        let creator = "test_creator";
        let timestamp = 1234567890;

        let id1 = generator.generate_bounty_id(creator, timestamp).unwrap();
        let id2 = generator.generate_bounty_id(creator, timestamp).unwrap();

        // Should still be different due to entropy
        assert_ne!(id1, id2);
        
        let stats = generator.get_statistics().unwrap();
        assert!(!stats.collision_detection_enabled);
    }

    #[test]
    fn test_uniqueness_validation_disabled() {
        let mut config = SecureIdConfig::default();
        config.enable_uniqueness_validation = false;
        let generator = SecureIdGenerator::new(config);

        let id = generator.generate_bounty_id("creator", 1234567890).unwrap();
        
        // Should always return true when validation is disabled
        assert!(generator.validate_id_uniqueness(id, "bounty").unwrap());
        
        let stats = generator.get_statistics().unwrap();
        assert!(!stats.uniqueness_validation_enabled);
    }

    #[test]
    fn test_cache_cleanup() {
        let mut config = SecureIdConfig::default();
        config.id_cache_size = 5; // Small cache for testing
        let generator = SecureIdGenerator::new(config);

        // Generate more IDs than cache size
        for i in 0..10 {
            generator.generate_bounty_id(&format!("user{}", i), 1234567890 + i).unwrap();
        }

        let stats = generator.get_statistics().unwrap();
        // Should not exceed cache size
        assert!(stats.total_ids_generated <= 5);
    }

    #[test]
    fn test_entropy_collection() {
        let config = SecureIdConfig::default();
        let generator = SecureIdGenerator::new(config);

        // Test different entropy scenarios
        let test_cases = vec![
            ("user1", 1234567890, "bounty"),
            ("user2", 1234567891, "session"),
            ("user3", 1234567892, "transaction"),
        ];

        for (user, timestamp, context) in test_cases {
            let id = generator.generate_bounty_id(user, timestamp).unwrap();
            assert!(id > 0, "Failed for user: {}, timestamp: {}, context: {}", user, timestamp, context);
        }
    }

    #[test]
    fn test_hash_entropy_to_bytes() {
        let config = SecureIdConfig::default();
        let generator = SecureIdGenerator::new(config);

        let entropy = "test_entropy_string";
        let bytes = generator.hash_entropy_to_bytes(entropy).unwrap();

        assert_eq!(bytes.len(), 32);
        
        // Same entropy should produce same bytes
        let bytes2 = generator.hash_entropy_to_bytes(entropy).unwrap();
        assert_eq!(bytes, bytes2);
        
        // Different entropy should produce different bytes
        let bytes3 = generator.hash_entropy_to_bytes("different_entropy").unwrap();
        assert_ne!(bytes, bytes3);
    }

    #[test]
    fn test_hash_entropy_to_id() {
        let config = SecureIdConfig::default();
        let generator = SecureIdGenerator::new(config);

        let entropy = "test_entropy_string";
        let id = generator.hash_entropy_to_id(entropy).unwrap();

        assert!(id > 0);
        
        // Same entropy should produce same ID
        let id2 = generator.hash_entropy_to_id(entropy).unwrap();
        assert_eq!(id, id2);
        
        // Different entropy should produce different ID
        let id3 = generator.hash_entropy_to_id("different_entropy").unwrap();
        assert_ne!(id, id3);
    }

    #[test]
    fn test_system_entropy() {
        let config = SecureIdConfig::default();
        let generator = SecureIdGenerator::new(config);

        let entropy1 = generator.get_system_entropy();
        let entropy2 = generator.get_system_entropy();

        // Should be different due to time-based entropy
        assert_ne!(entropy1, entropy2);
        
        // Should contain process information
        assert!(entropy1.len() > 10);
        assert!(entropy2.len() > 10);
    }

    #[test]
    fn test_random_entropy() {
        let config = SecureIdConfig::default();
        let generator = SecureIdGenerator::new(config);

        let entropy1 = generator.get_random_entropy();
        let entropy2 = generator.get_random_entropy();

        // Should be different due to counter increment
        assert_ne!(entropy1, entropy2);
        
        // Should be valid hex strings
        assert!(entropy1.starts_with("0x") || entropy1.chars().all(|c| c.is_ascii_hexdigit()));
        assert!(entropy2.starts_with("0x") || entropy2.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_string_hashing() {
        let config = SecureIdConfig::default();
        let generator = SecureIdGenerator::new(config);

        let hash1 = generator.hash_string("test_string");
        let hash2 = generator.hash_string("test_string");
        let hash3 = generator.hash_string("different_string");

        assert_eq!(hash1, hash2); // Same string should produce same hash
        assert_ne!(hash1, hash3); // Different strings should produce different hashes
    }

    #[test]
    fn test_external_entropy() {
        let config = SecureIdConfig::default();
        let generator = SecureIdGenerator::new(config);

        let entropy = generator.get_external_entropy("seed", 1234567890, "context");
        
        assert!(entropy.contains("seed"));
        assert!(entropy.contains("1234567890"));
        assert!(entropy.contains("context"));
        assert!(entropy.contains("external"));
    }

    #[test]
    fn test_id_validation_edge_cases() {
        let config = SecureIdConfig::default();
        let generator = SecureIdGenerator::new(config);

        // Test with very large IDs
        let large_id = u64::MAX / 2;
        let is_unique = generator.validate_id_uniqueness(large_id, "test").unwrap();
        assert!(is_unique);

        // Test with zero ID
        let zero_id = 0;
        let is_unique = generator.validate_id_uniqueness(zero_id, "test").unwrap();
        assert!(is_unique);
    }

    #[test]
    fn test_config_variations() {
        let test_configs = vec![
            // Default config
            SecureIdConfig::default(),
            // Disabled collision detection
            SecureIdConfig {
                enable_collision_detection: false,
                ..Default::default()
            },
            // Disabled uniqueness validation
            SecureIdConfig {
                enable_uniqueness_validation: false,
                ..Default::default()
            },
            // Small cache size
            SecureIdConfig {
                id_cache_size: 10,
                ..Default::default()
            },
            // Low retry count
            SecureIdConfig {
                max_collision_retries: 5,
                ..Default::default()
            },
        ];

        for (i, config) in test_configs.into_iter().enumerate() {
            let generator = SecureIdGenerator::new(config);
            let result = generator.generate_bounty_id(&format!("user{}", i), 1234567890 + i);
            assert!(result.is_ok(), "Failed with config variation {}", i);
        }
    }

    #[test]
    fn test_error_handling() {
        let config = SecureIdConfig::default();
        let generator = SecureIdGenerator::new(config);

        // Test with invalid inputs (should still work due to robust hashing)
        let test_cases = vec![
            ("", 0),
            ("", u64::MAX),
            ("valid_user", 0),
            ("valid_user", u64::MAX),
        ];

        for (user, timestamp) in test_cases {
            let result = generator.generate_bounty_id(user, timestamp);
            assert!(result.is_ok(), "Failed for user: '{}', timestamp: {}", user, timestamp);
        }
    }

    #[test]
    fn test_performance_benchmark() {
        let config = SecureIdConfig::default();
        let generator = SecureIdGenerator::new(config);

        let start = std::time::Instant::now();
        
        // Generate 1000 IDs
        for i in 0..1000 {
            generator.generate_bounty_id(&format!("user{}", i), 1234567890 + i).unwrap();
        }
        
        let duration = start.elapsed();
        
        // Should complete in reasonable time (less than 1 second for 1000 IDs)
        assert!(duration.as_secs() < 1, "Performance test failed: took {:?}", duration);
        
        let stats = generator.get_statistics().unwrap();
        assert_eq!(stats.total_ids_generated, 1000);
    }
}

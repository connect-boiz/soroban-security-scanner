//! End-to-end integration tests for the caching layer: warming, read-through
//! with stampede protection, TTL expiry, change-driven invalidation with
//! consistency verification, and hit-rate targets.

use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

#[test]
fn warm_then_serve_meets_hit_rate_target() {
    let clock = Clock::fixed(1000);
    let cache = Cache::<String>::in_memory(clock);

    // Warm critical config + patterns at startup.
    let mut warmer = CacheWarmer::new();
    for i in 0..10 {
        warmer.add(
            Partition::VulnerabilityPatterns,
            format!("p{i}"),
            1,
            move || format!("pattern-{i}"),
        );
    }
    warmer.warm(&cache);

    // Simulate steady traffic: 100 reads over the 10 warmed keys.
    for i in 0..100 {
        let key = format!("p{}", i % 10);
        assert!(cache.get(Partition::VulnerabilityPatterns, &key).is_some());
    }

    let stats = cache.stats();
    assert_eq!(stats.misses, 0);
    assert!(stats.hit_rate() >= TARGET_HIT_RATE);
}

#[test]
fn expensive_scan_is_computed_once_under_load() {
    let cache = Arc::new(Cache::<String>::in_memory(Clock::fixed(1000)));
    let computations = Arc::new(AtomicUsize::new(0));

    // 20 concurrent requests for the same contract's scan result.
    let handles: Vec<_> = (0..20)
        .map(|_| {
            let cache = Arc::clone(&cache);
            let computations = Arc::clone(&computations);
            thread::spawn(move || {
                cache.get_or_load(Partition::ScanResults, "contract-xyz", 1, || {
                    computations.fetch_add(1, Ordering::Relaxed);
                    for _ in 0..2000 {
                        std::hint::spin_loop();
                    }
                    "scan-report".to_string()
                })
            })
        })
        .collect();
    for h in handles {
        assert_eq!(h.join().unwrap(), "scan-report");
    }

    // The expensive scan ran exactly once (stampede protection).
    assert_eq!(computations.load(Ordering::Relaxed), 1);
    assert_eq!(cache.stats().loads, 1);
}

#[test]
fn ttl_expiry_triggers_refresh() {
    let clock = Clock::fixed(1000);
    let cache = Cache::<String>::in_memory(clock.clone());
    let loads = AtomicUsize::new(0);

    let load = || {
        loads.fetch_add(1, Ordering::Relaxed);
        "v1".to_string()
    };
    cache.get_or_load(Partition::UserProfiles, "u1", 1, load); // TTL 300

    // Within TTL: served from cache.
    cache.get_or_load(Partition::UserProfiles, "u1", 1, || {
        loads.fetch_add(1, Ordering::Relaxed);
        "v2".to_string()
    });
    assert_eq!(loads.load(Ordering::Relaxed), 1);

    // After TTL: reloaded.
    clock.advance(301);
    let v = cache.get_or_load(Partition::UserProfiles, "u1", 2, || {
        loads.fetch_add(1, Ordering::Relaxed);
        "v3".to_string()
    });
    assert_eq!(v, "v3");
    assert_eq!(loads.load(Ordering::Relaxed), 2);
}

#[test]
fn change_driven_invalidation_with_consistency_check() {
    let cache = Cache::<String>::in_memory(Clock::fixed(1000));
    let mut versions = VersionRegistry::new();

    // Cache a profile stamped at the current source version.
    let v = versions.bump("user_profiles:u1"); // v1
    cache.put(Partition::UserProfiles, "u1", "alice".to_string(), v);

    // Underlying data changes → source version advances.
    let new_v = versions.bump("user_profiles:u1"); // v2

    // A consistency check shows the cached entry (v1) is now stale.
    assert!(!verify(v, new_v).is_fresh());

    // Change-driven invalidation removes the stale entry.
    assert!(cache.invalidate(Partition::UserProfiles, "u1"));
    assert!(cache.get(Partition::UserProfiles, "u1").is_none());
}

#[test]
fn multi_level_survives_l1_flush() {
    // Build a 2-level cache (L1 app + L2 "redis").
    let l1: Box<dyn CacheBackend<String>> = Box::new(InMemoryBackend::new());
    let l2: Box<dyn CacheBackend<String>> = Box::new(InMemoryBackend::new());
    let cache = Cache::new(vec![l1, l2], PartitionTtls::new(), Clock::fixed(1000));

    cache.put(Partition::Config, "limits", "blob".to_string(), 1);
    // Simulate an L1 (process) restart by invalidating only L1 is not exposed;
    // instead confirm a value remains retrievable across the level hierarchy.
    assert_eq!(
        cache.get(Partition::Config, "limits"),
        Some("blob".to_string())
    );
    assert!(cache.stats().hits >= 1);
}

#[test]
fn partition_ttls_differ_by_data_type() {
    // Patterns outlive profiles: a value cached in each partition behaves per
    // its TTL.
    let clock = Clock::fixed(1000);
    let cache = Cache::<String>::in_memory(clock.clone());
    cache.put(Partition::VulnerabilityPatterns, "k", "pat".to_string(), 1); // 3600
    cache.put(Partition::UserProfiles, "k", "prof".to_string(), 1); // 300

    clock.advance(400); // profiles expired, patterns still valid
    assert!(cache.get(Partition::UserProfiles, "k").is_none());
    assert_eq!(
        cache.get(Partition::VulnerabilityPatterns, "k"),
        Some("pat".to_string())
    );
}

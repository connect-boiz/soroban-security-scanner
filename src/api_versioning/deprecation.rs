//! API version deprecation policy and version registry
//!
//! Implements the deprecation lifecycle with minimum 6-month notice period,
//! automated sunset tracking, and client notification support.

use crate::api_versioning::version::{ApiVersion, VersionInfo, VersionLifecycle};
use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use std::sync::RwLock;

/// Deprecation policy configuration
#[derive(Debug, Clone)]
pub struct DeprecationPolicy {
    /// Minimum notice period before sunset (in days)
    pub min_notice_days: i64,
    /// Whether to automatically notify clients via response headers
    pub auto_notify_clients: bool,
    /// Whether to send email notifications for upcoming sunsets
    pub email_notifications: bool,
    /// Days before sunset to start sending urgency notifications
    pub urgency_notification_days: Vec<i64>,
}

impl Default for DeprecationPolicy {
    fn default() -> Self {
        Self {
            min_notice_days: 180, // 6 months minimum
            auto_notify_clients: true,
            email_notifications: false,
            urgency_notification_days: vec![90, 60, 30, 14, 7, 1],
        }
    }
}

impl DeprecationPolicy {
    /// Create a strict policy with longer notice period
    pub fn strict() -> Self {
        Self {
            min_notice_days: 365, // 1 year
            auto_notify_clients: true,
            email_notifications: true,
            urgency_notification_days: vec![180, 90, 60, 30, 14, 7, 1],
        }
    }

    /// Check if a proposed sunset date meets the minimum notice requirement
    pub fn validate_sunset_date(&self, sunset_date: DateTime<Utc>) -> Result<(), String> {
        let min_sunset = Utc::now() + Duration::days(self.min_notice_days);
        if sunset_date < min_sunset {
            Err(format!(
                "Sunset date must be at least {} days from now. Minimum: {}, Proposed: {}",
                self.min_notice_days,
                min_sunset.format("%Y-%m-%d"),
                sunset_date.format("%Y-%m-%d"),
            ))
        } else {
            Ok(())
        }
    }

    /// Get the minimum allowed sunset date
    pub fn min_sunset_date(&self) -> DateTime<Utc> {
        Utc::now() + Duration::days(self.min_notice_days)
    }
}

/// Registry tracking all API versions and their lifecycle states
#[derive(Debug)]
pub struct VersionRegistry {
    versions: RwLock<HashMap<ApiVersion, VersionInfo>>,
    deprecation_policy: DeprecationPolicy,
}

impl Default for VersionRegistry {
    fn default() -> Self {
        let mut versions = HashMap::new();

        // Initialize with V1 as the current stable version
        let mut v1 = VersionInfo::new_stable(
            ApiVersion::V1,
            "Initial API version. Provides core security scanning, authentication, \
             and transaction processing endpoints.",
        );
        v1.add_non_breaking_change("Initial API release with all core endpoints");
        versions.insert(ApiVersion::V1, v1);

        // Pre-register V2 as alpha (future version)
        let v2 = VersionInfo::new_alpha(
            ApiVersion::V2,
            "Next-generation API with improved performance and new features.",
        );
        versions.insert(ApiVersion::V2, v2);

        Self {
            versions: RwLock::new(versions),
            deprecation_policy: DeprecationPolicy::default(),
        }
    }
}

impl VersionRegistry {
    /// Create a new registry with a custom deprecation policy
    pub fn with_policy(policy: DeprecationPolicy) -> Self {
        let mut registry = Self::default();
        registry.deprecation_policy = policy;
        registry
    }

    /// Register a new version
    pub fn register_version(&self, info: VersionInfo) -> Result<(), String> {
        let mut versions = self.versions.write().map_err(|e| e.to_string())?;
        if versions.contains_key(&info.version) {
            return Err(format!(
                "Version {} is already registered",
                info.version.as_path()
            ));
        }
        versions.insert(info.version, info);
        Ok(())
    }

    /// Get version info
    pub fn get_version(&self, version: ApiVersion) -> Option<VersionInfo> {
        self.versions
            .read()
            .ok()
            .and_then(|v| v.get(&version).cloned())
    }

    /// List all registered versions
    pub fn list_versions(&self) -> Vec<VersionInfo> {
        self.versions
            .read()
            .ok()
            .map(|v| {
                let mut versions: Vec<_> = v.values().cloned().collect();
                versions.sort_by_key(|v| v.version);
                versions
            })
            .unwrap_or_default()
    }

    /// List only served (non-sunset) versions
    pub fn list_active_versions(&self) -> Vec<VersionInfo> {
        self.list_versions()
            .into_iter()
            .filter(|v| v.lifecycle.is_served())
            .collect()
    }

    /// Get the current stable version
    pub fn current_stable(&self) -> Option<VersionInfo> {
        self.versions.read().ok().and_then(|v| {
            v.values()
                .find(|info| info.lifecycle == VersionLifecycle::Stable)
                .cloned()
        })
    }
    /// Deprecate a version with minimum notice enforcement.
    ///
    /// Computes `sunset_date` from a single `now` snapshot so the
    /// minimum-notice window is satisfied by construction — no second
    /// `Utc::now()` call that could observe clock drift and falsely
    /// reject the deprecation.
    pub fn deprecate_version(&self, version: ApiVersion) -> Result<(), String> {
        let mut versions = self.versions.write().map_err(|e| e.to_string())?;
        let info = versions
            .get_mut(&version)
            .ok_or_else(|| format!("Version {} not found", version.as_path()))?;

        if info.lifecycle == VersionLifecycle::Sunset {
            return Err(format!("Version {} is already sunset", version.as_path()));
        }

        let now = Utc::now();
        info.lifecycle = VersionLifecycle::Deprecated;
        info.deprecation_date = Some(now);
        info.sunset_date = Some(now + Duration::days(self.deprecation_policy.min_notice_days));

        Ok(())
    }

    /// Sunset a deprecated version (stop serving it)
    pub fn sunset_version(&self, version: ApiVersion) -> Result<(), String> {
        let mut versions = self.versions.write().map_err(|e| e.to_string())?;
        let info = versions
            .get_mut(&version)
            .ok_or_else(|| format!("Version {} not found", version.as_path()))?;

        if info.lifecycle != VersionLifecycle::Deprecated {
            return Err(format!(
                "Version {} must be deprecated before sunsetting (current: {})",
                version.as_path(),
                info.lifecycle.as_str()
            ));
        }

        info.sunset();
        Ok(())
    }

    /// Promote a version to stable (typically from beta)
    pub fn promote_to_stable(&self, version: ApiVersion) -> Result<(), String> {
        let mut versions = self.versions.write().map_err(|e| e.to_string())?;

        // Check the version exists and its lifecycle is not sunset
        {
            let info = versions
                .get(&version)
                .ok_or_else(|| format!("Version {} not found", version.as_path()))?;

            if info.lifecycle == VersionLifecycle::Sunset {
                return Err(format!(
                    "Cannot promote sunset version {}",
                    version.as_path()
                ));
            }
        }

        // Demote previous stable version(s) to deprecated if they exist
        for (_, other_info) in versions.iter_mut() {
            if other_info.lifecycle == VersionLifecycle::Stable && other_info.version != version {
                other_info.deprecate();
            }
        }

        // Now promote the target version
        if let Some(info) = versions.get_mut(&version) {
            info.lifecycle = VersionLifecycle::Stable;
        }

        Ok(())
    }

    /// Add a change to a version's changelog
    pub fn add_change(
        &self,
        version: ApiVersion,
        change: &str,
        is_breaking: bool,
    ) -> Result<(), String> {
        let mut versions = self.versions.write().map_err(|e| e.to_string())?;
        let info = versions
            .get_mut(&version)
            .ok_or_else(|| format!("Version {} not found", version.as_path()))?;

        if is_breaking {
            if !info.lifecycle.allows_breaking_changes() {
                return Err(format!(
                    "Breaking changes not allowed for version {} in {} phase",
                    version.as_path(),
                    info.lifecycle.as_str()
                ));
            }
            info.add_breaking_change(change);
        } else {
            info.add_non_breaking_change(change);
        }

        Ok(())
    }

    /// Get deprecation policy
    pub fn deprecation_policy(&self) -> &DeprecationPolicy {
        &self.deprecation_policy
    }

    /// Check which versions need urgency notifications based on sunset proximity
    pub fn get_urgency_notifications(&self) -> Vec<UrgencyNotification> {
        let policy = &self.deprecation_policy;
        let mut notifications = Vec::new();

        if let Ok(versions) = self.versions.read() {
            for (version, info) in versions.iter() {
                if info.lifecycle == VersionLifecycle::Deprecated {
                    if let Some(sunset) = info.sunset_date {
                        let days_remaining = (sunset - Utc::now()).num_days();
                        for &threshold in &policy.urgency_notification_days {
                            if days_remaining <= threshold && days_remaining > threshold - 1 {
                                notifications.push(UrgencyNotification {
                                    version: *version,
                                    days_until_sunset: days_remaining,
                                    sunset_date: sunset,
                                    threshold,
                                });
                            }
                        }
                    }
                }
            }
        }

        notifications
    }
}

/// Notification for upcoming version sunset
#[derive(Debug, Clone)]
pub struct UrgencyNotification {
    pub version: ApiVersion,
    pub days_until_sunset: i64,
    pub sunset_date: DateTime<Utc>,
    pub threshold: i64,
}

/// Sunset procedures documentation
pub struct SunsetProcedures;

impl SunsetProcedures {
    /// Get the sunset procedure checklist
    pub fn checklist() -> Vec<&'static str> {
        vec![
            "1. Announce deprecation with minimum 6-month notice",
            "2. Add deprecation warnings via HTTP headers (X-API-Deprecated, X-API-Sunset)",
            "3. Document migration path to the current stable version",
            "4. Monitor deprecated version usage metrics",
            "5. Send urgency notifications at 90, 60, 30, 14, 7, and 1 days before sunset",
            "6. Verify zero production traffic before sunset date",
            "7. Archive deprecated version documentation",
            "8. Return 410 Gone for sunset version endpoints",
            "9. Update API version listing to mark version as sunset",
            "10. Update client SDKs and documentation to remove references",
        ]
    }

    /// Get the client migration guide template
    pub fn migration_guide_template(from_version: ApiVersion, to_version: ApiVersion) -> String {
        format!(
            r#"# API Migration Guide: {} to {}

## Overview
This guide helps you migrate your integration from API {} to {}.

## Timeline
- **Deprecation announced**: [DATE]
- **Sunset date**: [DATE]
- **Migration deadline**: [DATE]

## Breaking Changes
The following changes require updates to your code:
[List breaking changes here]

## Non-Breaking Changes
The following new features are available:
[List non-breaking changes here]

## Step-by-Step Migration

### 1. Update API Base URL
```
Old: /api/{}/
New: /api/{}/
```

### 2. Update Request Headers
```
Old: Accept: application/vnd.soroban.{}+json
New: Accept: application/vnd.soroban.{}+json
```

### 3. Update Response Handling
[Describe response format changes]

## Testing Your Migration
Use the version compatibility test suite:
```bash
cargo test api_versioning::compatibility
```

## Support
If you encounter issues, please file an issue at:
https://github.com/connect-boiz/soroban-security-scanner/issues
"#,
            from_version.as_path(),
            to_version.as_path(),
            from_version.as_path(),
            to_version.as_path(),
            from_version.as_path(),
            to_version.as_path(),
            from_version.as_path(),
            to_version.as_path(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deprecation_policy_default_six_months() {
        let policy = DeprecationPolicy::default();
        assert_eq!(policy.min_notice_days, 180);
    }

    #[test]
    fn test_deprecation_policy_validate_minimum() {
        let policy = DeprecationPolicy::default();
        // A date too close should fail
        let too_soon = Utc::now() + Duration::days(30);
        assert!(policy.validate_sunset_date(too_soon).is_err());

        // A date far enough should pass
        let far_enough = Utc::now() + Duration::days(200);
        assert!(policy.validate_sunset_date(far_enough).is_ok());
    }

    #[test]
    fn test_version_registry_default() {
        let registry = VersionRegistry::default();
        assert!(registry.get_version(ApiVersion::V1).is_some());
        assert_eq!(
            registry.get_version(ApiVersion::V1).unwrap().lifecycle,
            VersionLifecycle::Stable
        );
    }

    #[test]
    fn test_deprecate_version() {
        let registry = VersionRegistry::default();
        assert!(registry.deprecate_version(ApiVersion::V1).is_ok());
        let info = registry.get_version(ApiVersion::V1).unwrap();
        assert_eq!(info.lifecycle, VersionLifecycle::Deprecated);
        assert!(info.should_warn());
    }

    #[test]
    fn test_promote_to_stable() {
        let registry = VersionRegistry::default();
        // Promote V2 (alpha) to stable
        assert!(registry.promote_to_stable(ApiVersion::V2).is_ok());
        let v2_info = registry.get_version(ApiVersion::V2).unwrap();
        assert_eq!(v2_info.lifecycle, VersionLifecycle::Stable);

        // V1 should now be deprecated
        let v1_info = registry.get_version(ApiVersion::V1).unwrap();
        assert_eq!(v1_info.lifecycle, VersionLifecycle::Deprecated);
    }

    #[test]
    fn test_sunset_version() {
        let registry = VersionRegistry::default();
        registry.deprecate_version(ApiVersion::V1).unwrap();
        assert!(registry.sunset_version(ApiVersion::V1).is_ok());
        let info = registry.get_version(ApiVersion::V1).unwrap();
        assert_eq!(info.lifecycle, VersionLifecycle::Sunset);
    }

    #[test]
    fn test_cannot_sunset_non_deprecated() {
        let registry = VersionRegistry::default();
        assert!(registry.sunset_version(ApiVersion::V1).is_err());
    }

    #[test]
    fn test_breaking_change_not_allowed_in_stable() {
        let registry = VersionRegistry::default();
        let result = registry.add_change(ApiVersion::V1, "Removed endpoint", true);
        assert!(result.is_err());
    }

    #[test]
    fn test_breaking_change_allowed_in_alpha() {
        let registry = VersionRegistry::default();
        let result = registry.add_change(ApiVersion::V2, "Changed API", true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_sunset_procedures_checklist() {
        let checklist = SunsetProcedures::checklist();
        assert_eq!(checklist.len(), 10);
    }

    #[test]
    fn test_migration_guide_template() {
        let guide = SunsetProcedures::migration_guide_template(ApiVersion::V1, ApiVersion::V2);
        assert!(guide.contains("v1 to v2"));
        assert!(guide.contains("/api/v1/"));
        assert!(guide.contains("/api/v2/"));
    }
}

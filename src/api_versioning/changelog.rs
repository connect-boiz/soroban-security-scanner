//! API change log system
//!
//! Tracks all API changes with classification (breaking vs non-breaking),
//! timestamps, and version associations. Supports generating changelog
//! reports in multiple formats.

use crate::api_versioning::version::ApiVersion;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;

/// Classification of an API change
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeType {
    /// Breaking change - requires client updates
    Breaking,
    /// Non-breaking addition - new functionality
    Addition,
    /// Non-breaking change - improvement or bug fix
    Improvement,
    /// Deprecation notice
    Deprecation,
    /// Security fix
    Security,
    /// Documentation update
    Documentation,
    /// Performance improvement
    Performance,
}

impl ChangeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ChangeType::Breaking => "breaking",
            ChangeType::Addition => "addition",
            ChangeType::Improvement => "improvement",
            ChangeType::Deprecation => "deprecation",
            ChangeType::Security => "security",
            ChangeType::Documentation => "documentation",
            ChangeType::Performance => "performance",
        }
    }

    pub fn is_breaking(&self) -> bool {
        matches!(self, ChangeType::Breaking)
    }

    pub fn emoji(&self) -> &'static str {
        match self {
            ChangeType::Breaking => "💥",
            ChangeType::Addition => "✨",
            ChangeType::Improvement => "🔧",
            ChangeType::Deprecation => "⚠️ ",
            ChangeType::Security => "🔒",
            ChangeType::Documentation => "📚",
            ChangeType::Performance => "⚡",
        }
    }
}

/// A single API change entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeEntry {
    /// Unique change ID
    pub id: String,
    /// API version this change applies to
    pub version: ApiVersion,
    /// Type of change
    pub change_type: ChangeType,
    /// Short summary of the change
    pub summary: String,
    /// Detailed description
    pub description: String,
    /// Affected endpoints
    pub affected_endpoints: Vec<String>,
    /// Migration guidance (for breaking changes)
    pub migration_guide: Option<String>,
    /// When the change was made
    pub timestamp: DateTime<Utc>,
    /// Author/committer
    pub author: String,
}

impl ChangeEntry {
    /// Create a new change entry
    pub fn new(
        version: ApiVersion,
        change_type: ChangeType,
        summary: &str,
        description: &str,
        author: &str,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            version,
            change_type,
            summary: summary.to_string(),
            description: description.to_string(),
            affected_endpoints: Vec::new(),
            migration_guide: None,
            timestamp: Utc::now(),
            author: author.to_string(),
        }
    }

    /// Add affected endpoints
    pub fn with_endpoints(mut self, endpoints: Vec<&str>) -> Self {
        self.affected_endpoints = endpoints.into_iter().map(String::from).collect();
        self
    }

    /// Add migration guidance
    pub fn with_migration_guide(mut self, guide: &str) -> Self {
        self.migration_guide = Some(guide.to_string());
        self
    }
}

/// The API change log
#[derive(Debug)]
pub struct ApiChangeLog {
    entries: RwLock<Vec<ChangeEntry>>,
}

impl ApiChangeLog {
    /// Create a new empty change log
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(Vec::new()),
        }
    }

    /// Add a change entry
    pub fn add_entry(&self, entry: ChangeEntry) -> Result<(), String> {
        let mut entries = self.entries.write().map_err(|e| e.to_string())?;
        entries.push(entry);
        Ok(())
    }

    /// Get all entries
    pub fn get_all(&self) -> Vec<ChangeEntry> {
        self.entries
            .read()
            .ok()
            .map(|e| e.clone())
            .unwrap_or_default()
    }

    /// Get entries for a specific version
    pub fn get_by_version(&self, version: ApiVersion) -> Vec<ChangeEntry> {
        self.get_all()
            .into_iter()
            .filter(|e| e.version == version)
            .collect()
    }

    /// Get only breaking changes
    pub fn get_breaking_changes(&self) -> Vec<ChangeEntry> {
        self.get_all()
            .into_iter()
            .filter(|e| e.change_type.is_breaking())
            .collect()
    }

    /// Get breaking changes for a specific version
    pub fn get_breaking_changes_for_version(&self, version: ApiVersion) -> Vec<ChangeEntry> {
        self.get_all()
            .into_iter()
            .filter(|e| e.version == version && e.change_type.is_breaking())
            .collect()
    }

    /// Generate a markdown changelog
    pub fn generate_markdown(&self) -> String {
        let mut output = String::new();
        output.push_str("# API Change Log\n\n");

        // Group by version
        let mut by_version: HashMap<ApiVersion, Vec<ChangeEntry>> = HashMap::new();
        for entry in self.get_all() {
            by_version.entry(entry.version).or_default().push(entry);
        }

        let mut versions: Vec<_> = by_version.keys().cloned().collect();
        versions.sort();
        versions.reverse(); // Newest first

        for version in &versions {
            let entries = by_version.get(version).unwrap();
            output.push_str(&format!("## API {}\n\n", version.as_path()));

            // Group by change type
            let change_types = [
                ChangeType::Breaking,
                ChangeType::Security,
                ChangeType::Addition,
                ChangeType::Improvement,
                ChangeType::Performance,
                ChangeType::Deprecation,
                ChangeType::Documentation,
            ];

            for change_type in &change_types {
                let typed_entries: Vec<_> = entries
                    .iter()
                    .filter(|e| e.change_type == *change_type)
                    .collect();

                if !typed_entries.is_empty() {
                    output.push_str(&format!(
                        "### {} {}\n\n",
                        change_type.emoji(),
                        match change_type {
                            ChangeType::Breaking => "Breaking Changes",
                            ChangeType::Addition => "New Features",
                            ChangeType::Improvement => "Improvements",
                            ChangeType::Deprecation => "Deprecations",
                            ChangeType::Security => "Security Fixes",
                            ChangeType::Documentation => "Documentation",
                            ChangeType::Performance => "Performance",
                        }
                    ));

                    for entry in &typed_entries {
                        output.push_str(&format!(
                            "- **{}** - {}\n",
                            entry.summary, entry.description
                        ));

                        if !entry.affected_endpoints.is_empty() {
                            output.push_str(&format!(
                                "  - Affected endpoints: {}\n",
                                entry.affected_endpoints.join(", ")
                            ));
                        }

                        if let Some(ref guide) = entry.migration_guide {
                            output.push_str(&format!("  - Migration: {}\n", guide));
                        }
                    }
                    output.push('\n');
                }
            }
        }

        output
    }

    /// Generate a JSON changelog
    pub fn generate_json(&self) -> String {
        let entries = self.get_all();
        serde_json::to_string_pretty(&entries).unwrap_or_default()
    }

    /// Generate a brief summary for a specific version transition
    pub fn generate_migration_summary(
        &self,
        from_version: ApiVersion,
        to_version: ApiVersion,
    ) -> String {
        let breaking = self.get_breaking_changes_for_version(to_version);

        if breaking.is_empty() {
            format!(
                "No breaking changes when migrating from API {} to API {}.",
                from_version.as_path(),
                to_version.as_path()
            )
        } else {
            let mut summary = format!(
                "## Breaking Changes when migrating from API {} to API {}\n\n",
                from_version.as_path(),
                to_version.as_path()
            );

            for entry in &breaking {
                summary.push_str(&format!("- **{}**: {}\n", entry.summary, entry.description));
                if let Some(ref guide) = entry.migration_guide {
                    summary.push_str(&format!("  - *How to migrate:* {}\n", guide));
                }
            }

            summary
        }
    }
}

impl Default for ApiChangeLog {
    fn default() -> Self {
        let log = Self::new();

        // Add initial V1 release entries
        let _ = log.add_entry(
            ChangeEntry::new(
                ApiVersion::V1,
                ChangeType::Addition,
                "Initial API Release",
                "First stable release of the Soroban Security Scanner API with core \
                 security scanning, authentication, and transaction processing endpoints.",
                "connect-boiz",
            )
            .with_endpoints(vec![
                "/api/v1/transactions",
                "/api/v1/queue/stats",
                "/api/v1/monitoring/snapshot",
                "/api/v1/state/export",
                "/auth/login",
                "/auth/register",
                "/api/v1/profile",
                "/api/v1/admin/users",
                "/api/v1/admin/stats",
            ]),
        );

        let _ = log.add_entry(
            ChangeEntry::new(
                ApiVersion::V1,
                ChangeType::Addition,
                "API Versioning System",
                "Introduced API versioning with URL-based version prefixes (/api/v1/), \
                 Accept header negotiation, deprecation policies with 6-month notice, \
                 and sunset procedures.",
                "connect-boiz",
            )
            .with_endpoints(vec!["/api/versions", "/api"]),
        );

        log
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_retrieve_entries() {
        let log = ApiChangeLog::new();
        let entry = ChangeEntry::new(
            ApiVersion::V1,
            ChangeType::Addition,
            "Test feature",
            "Added test feature",
            "developer",
        );
        log.add_entry(entry).unwrap();
        assert_eq!(log.get_all().len(), 1);
    }

    #[test]
    fn test_get_by_version() {
        let log = ApiChangeLog::new();
        log.add_entry(ChangeEntry::new(
            ApiVersion::V1,
            ChangeType::Addition,
            "V1 feature",
            "V1 addition",
            "dev",
        ))
        .unwrap();
        log.add_entry(ChangeEntry::new(
            ApiVersion::V2,
            ChangeType::Addition,
            "V2 feature",
            "V2 addition",
            "dev",
        ))
        .unwrap();

        assert_eq!(log.get_by_version(ApiVersion::V1).len(), 1);
        assert_eq!(log.get_by_version(ApiVersion::V2).len(), 1);
    }

    #[test]
    fn test_get_breaking_changes() {
        let log = ApiChangeLog::new();
        log.add_entry(ChangeEntry::new(
            ApiVersion::V1,
            ChangeType::Addition,
            "Feature",
            "Non-breaking",
            "dev",
        ))
        .unwrap();
        log.add_entry(ChangeEntry::new(
            ApiVersion::V1,
            ChangeType::Breaking,
            "Breaking change",
            "Breaking",
            "dev",
        ))
        .unwrap();

        assert_eq!(log.get_breaking_changes().len(), 1);
    }

    #[test]
    fn test_generate_markdown() {
        let log = ApiChangeLog::new();
        log.add_entry(ChangeEntry::new(
            ApiVersion::V1,
            ChangeType::Addition,
            "Test",
            "Desc",
            "dev",
        ))
        .unwrap();
        let md = log.generate_markdown();
        assert!(md.contains("# API Change Log"));
        assert!(md.contains("## API v1"));
        assert!(md.contains("Test"));
    }

    #[test]
    fn test_generate_migration_summary_no_breaking() {
        let log = ApiChangeLog::new();
        let summary = log.generate_migration_summary(ApiVersion::V1, ApiVersion::V2);
        assert!(summary.contains("No breaking changes"));
    }

    #[test]
    fn test_change_type_classification() {
        assert!(ChangeType::Breaking.is_breaking());
        assert!(!ChangeType::Addition.is_breaking());
        assert!(!ChangeType::Improvement.is_breaking());
        assert!(!ChangeType::Documentation.is_breaking());
    }
}

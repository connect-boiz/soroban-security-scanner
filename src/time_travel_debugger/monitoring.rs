use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessPattern {
    pub user_id: String,
    pub timestamps: Vec<Instant>,
    pub operations: Vec<String>,
    pub contracts_accessed: Vec<String>,
    pub ledger_sequences: Vec<u32>,
    pub ip_addresses: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuspiciousPattern {
    pub pattern_type: SuspiciousPatternType,
    pub user_id: String,
    pub severity: SuspiciousSeverity,
    pub description: String,
    pub detected_at: Instant,
    pub details: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SuspiciousPatternType {
    RapidFireAccess,
    UnusualTimeAccess,
    SequentialLedgerScan,
    UnknownContractAccess,
    ExcessiveFailedAttempts,
    OffHoursOperation,
    GeographicAnomaly,
    ConcurrentMultiSession,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SuspiciousSeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    pub rapid_fire_threshold_ms: u64,
    pub sequential_scan_threshold: u32,
    pub max_failed_attempts: u32,
    pub failed_attempts_window_seconds: u64,
    pub unusual_hours_start: u8,
    pub unusual_hours_end: u8,
    pub max_concurrent_sessions: u32,
    pub alert_threshold: u32,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            rapid_fire_threshold_ms: 100,
            sequential_scan_threshold: 50,
            max_failed_attempts: 5,
            failed_attempts_window_seconds: 300,
            unusual_hours_start: 0,
            unusual_hours_end: 6,
            max_concurrent_sessions: 3,
            alert_threshold: 10,
        }
    }
}

pub struct MonitoringEngine {
    config: MonitoringConfig,
    access_patterns: Arc<RwLock<HashMap<String, AccessPattern>>>,
    suspicious_events: Arc<RwLock<VecDeque<SuspiciousPattern>>>,
    failed_attempts: Arc<RwLock<HashMap<String, Vec<Instant>>>>,
    max_tracked_events: usize,
}

impl MonitoringEngine {
    pub fn new(config: MonitoringConfig) -> Self {
        Self {
            config,
            access_patterns: Arc::new(RwLock::new(HashMap::new())),
            suspicious_events: Arc::new(RwLock::new(VecDeque::new())),
            failed_attempts: Arc::new(RwLock::new(HashMap::new())),
            max_tracked_events: 1000,
        }
    }

    pub async fn record_access(
        &self,
        user_id: &str,
        operation: &str,
        contract_id: Option<&str>,
        ledger_sequence: Option<u32>,
        ip_address: Option<&str>,
    ) -> Option<SuspiciousPattern> {
        let mut patterns = self.access_patterns.write().await;
        let now = Instant::now();

        let pattern = patterns.entry(user_id.to_string()).or_insert_with(|| AccessPattern {
            user_id: user_id.to_string(),
            timestamps: Vec::new(),
            operations: Vec::new(),
            contracts_accessed: Vec::new(),
            ledger_sequences: Vec::new(),
            ip_addresses: Vec::new(),
        });

        pattern.timestamps.push(now);
        pattern.operations.push(operation.to_string());
        if let Some(cid) = contract_id {
            pattern.contracts_accessed.push(cid.to_string());
        }
        if let Some(seq) = ledger_sequence {
            pattern.ledger_sequences.push(seq);
        }
        if let Some(ip) = ip_address {
            pattern.ip_addresses.push(ip.to_string());
        }

        if pattern.timestamps.len() > 1000 {
            pattern.timestamps.drain(..500);
            pattern.operations.drain(..500);
            pattern.contracts_accessed.drain(..500);
            pattern.ledger_sequences.drain(..500);
            pattern.ip_addresses.drain(..500);
        }

        self.detect_rapid_fire(user_id).await
            .or_else(|| self.detect_sequential_scan(user_id).await)
            .or_else(|| self.detect_off_hours(user_id).await)
    }

    pub async fn record_failed_attempt(&self, user_id: &str) -> Option<SuspiciousPattern> {
        let mut attempts = self.failed_attempts.write().await;
        let now = Instant::now();
        let window = Duration::from_secs(self.config.failed_attempts_window_seconds);

        let user_attempts = attempts.entry(user_id.to_string()).or_insert_with(Vec::new);
        user_attempts.push(now);
        user_attempts.retain(|t| now.duration_since(*t) < window);

        if user_attempts.len() > self.config.max_failed_attempts as usize {
            Some(SuspiciousPattern {
                pattern_type: SuspiciousPatternType::ExcessiveFailedAttempts,
                user_id: user_id.to_string(),
                severity: SuspiciousSeverity::High,
                description: format!(
                    "User {} has {} failed attempts in the last {} seconds",
                    user_id,
                    user_attempts.len(),
                    self.config.failed_attempts_window_seconds
                ),
                detected_at: Instant::now(),
                details: {
                    let mut d = HashMap::new();
                    d.insert("count".to_string(), user_attempts.len().to_string());
                    d
                },
            })
        } else {
            None
        }
    }

    pub async fn detect_rapid_fire(&self, user_id: &str) -> Option<SuspiciousPattern> {
        let patterns = self.access_patterns.read().await;
        if let Some(pattern) = patterns.get(user_id) {
            let recent: Vec<&Instant> = pattern.timestamps.iter().rev().take(10).collect();
            if recent.len() >= 5 {
                let gaps: Vec<Duration> = recent
                    .windows(2)
                    .map(|w| w[0].duration_since(*w[1]))
                    .collect();
                let all_rapid = gaps
                    .iter()
                    .all(|g| g.as_millis() < self.config.rapid_fire_threshold_ms as u128);
                if all_rapid {
                    return Some(SuspiciousPattern {
                        pattern_type: SuspiciousPatternType::RapidFireAccess,
                        user_id: user_id.to_string(),
                        severity: SuspiciousSeverity::Medium,
                        description: format!(
                            "Rapid fire access detected for user {} - {} requests in rapid succession",
                            user_id,
                            recent.len()
                        ),
                        detected_at: Instant::now(),
                        details: {
                            let mut d = HashMap::new();
                            d.insert("request_count".to_string(), recent.len().to_string());
                            d
                        },
                    });
                }
            }
        }
        None
    }

    pub async fn detect_sequential_scan(&self, user_id: &str) -> Option<SuspiciousPattern> {
        let patterns = self.access_patterns.read().await;
        if let Some(pattern) = patterns.get(user_id) {
            if pattern.ledger_sequences.len() >= self.config.sequential_scan_threshold as usize {
                let recent: Vec<u32> = pattern
                    .ledger_sequences
                    .iter()
                    .rev()
                    .take(self.config.sequential_scan_threshold as usize)
                    .cloned()
                    .collect();

                if recent.len() >= 3 {
                    let sequential = recent.windows(3).all(|w| w[0] == w[1] + 1 && w[1] == w[2] + 1);
                    if sequential {
                        return Some(SuspiciousPattern {
                            pattern_type: SuspiciousPatternType::SequentialLedgerScan,
                            user_id: user_id.to_string(),
                            severity: SuspiciousSeverity::High,
                            description: format!(
                                "Sequential ledger scan detected for user {} - scanning {} consecutive ledgers",
                                user_id,
                                recent.len()
                            ),
                            detected_at: Instant::now(),
                            details: {
                                let mut d = HashMap::new();
                                d.insert("ledger_count".to_string(), recent.len().to_string());
                                d.insert("start".to_string(), recent.last().unwrap_or(&0).to_string());
                                d.insert("end".to_string(), recent.first().unwrap_or(&0).to_string());
                                d
                            },
                        });
                    }
                }
            }
        }
        None
    }

    pub async fn detect_off_hours(&self, user_id: &str) -> Option<SuspiciousPattern> {
        let patterns = self.access_patterns.read().await;
        if let Some(pattern) = patterns.get(user_id) {
            if let Some(latest) = pattern.timestamps.last() {
                let elapsed = latest.elapsed();
                let hours_since_midnight = (elapsed.as_secs() % 86400) / 3600;
                let unusual = hours_since_midnight >= self.config.unusual_hours_start as u64
                    && hours_since_midnight <= self.config.unusual_hours_end as u64;

                if unusual && pattern.timestamps.len() > 10 {
                    let recent_count = pattern
                        .timestamps
                        .iter()
                        .rev()
                        .take(10)
                        .filter(|t| {
                            let h = (t.elapsed().as_secs() % 86400) / 3600;
                            h >= self.config.unusual_hours_start as u64
                                && h <= self.config.unusual_hours_end as u64
                        })
                        .count();

                    if recent_count >= 8 {
                        return Some(SuspiciousPattern {
                            pattern_type: SuspiciousPatternType::OffHoursOperation,
                            user_id: user_id.to_string(),
                            severity: SuspiciousSeverity::Medium,
                            description: format!(
                                "Off-hours operation detected for user {} - {} operations during unusual hours",
                                user_id, recent_count
                            ),
                            detected_at: Instant::now(),
                            details: {
                                let mut d = HashMap::new();
                                d.insert("operations_count".to_string(), recent_count.to_string());
                                d
                            },
                        });
                    }
                }
            }
        }
        None
    }

    pub async fn log_suspicious_event(&self, event: SuspiciousPattern) {
        let mut events = self.suspicious_events.write().await;
        if events.len() >= self.max_tracked_events {
            events.pop_front();
        }
        events.push_back(event);
    }

    pub async fn get_suspicious_events(
        &self,
        min_severity: Option<SuspiciousSeverity>,
        limit: usize,
    ) -> Vec<SuspiciousPattern> {
        let events = self.suspicious_events.read().await;
        events
            .iter()
            .filter(|e| {
                if let Some(ref severity) = min_severity {
                    severity_severity(&e.severity) >= severity_severity(severity)
                } else {
                    true
                }
            })
            .cloned()
            .take(limit)
            .collect()
    }

    pub async fn get_user_pattern_summary(&self, user_id: &str) -> Option<UserPatternSummary> {
        let patterns = self.access_patterns.read().await;
        patterns.get(user_id).map(|p| {
            let total_ops = p.operations.len();
            let unique_contracts: std::collections::HashSet<&str> =
                p.contracts_accessed.iter().map(|s| s.as_str()).collect();
            UserPatternSummary {
                total_operations: total_ops,
                unique_contracts: unique_contracts.len(),
                unique_ledgers: {
                    let mut seqs = std::collections::HashSet::new();
                    for seq in &p.ledger_sequences {
                        seqs.insert(*seq);
                    }
                    seqs.len()
                },
                last_activity: p.timestamps.last().copied(),
            }
        })
    }

    pub async fn should_alert(&self) -> bool {
        let events = self.suspicious_events.read().await;
        let recent = events
            .iter()
            .filter(|e| e.detected_at.elapsed() < Duration::from_secs(3600))
            .count();
        recent >= self.config.alert_threshold as usize
    }
}

impl Default for MonitoringEngine {
    fn default() -> Self {
        Self::new(MonitoringConfig::default())
    }
}

fn severity_severity(s: &SuspiciousSeverity) -> u8 {
    match s {
        SuspiciousSeverity::Low => 0,
        SuspiciousSeverity::Medium => 1,
        SuspiciousSeverity::High => 2,
        SuspiciousSeverity::Critical => 3,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPatternSummary {
    pub total_operations: usize,
    pub unique_contracts: usize,
    pub unique_ledgers: usize,
    pub last_activity: Option<Instant>,
}

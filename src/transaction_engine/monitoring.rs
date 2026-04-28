//! Transaction monitoring and dashboard system

use crate::transaction_engine::{
    Transaction, TransactionState, TransactionType, TransactionPriority,
    QueueStats, ProcessorMetrics, RetryStats, TransactionFilter
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{Utc, DateTime, Duration};
use anyhow::Result;

/// Monitoring configuration
#[derive(Debug, Clone)]
pub struct MonitoringConfig {
    /// Enable real-time monitoring
    pub enable_real_time: bool,
    /// Metrics collection interval in seconds
    pub metrics_interval_seconds: u64,
    /// Alert thresholds
    pub alert_thresholds: AlertThresholds,
    /// Dashboard update interval in seconds
    pub dashboard_interval_seconds: u64,
    /// Enable performance tracking
    pub enable_performance_tracking: bool,
    /// Maximum history to keep (in hours)
    pub max_history_hours: u64,
}

/// Alert thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertThresholds {
    /// Queue size threshold for alerts
    pub queue_size_warning: usize,
    pub queue_size_critical: usize,
    /// Processing time thresholds (milliseconds)
    pub processing_time_warning: u64,
    pub processing_time_critical: u64,
    /// Failure rate thresholds (percentage)
    pub failure_rate_warning: f64,
    pub failure_rate_critical: f64,
    /// Retry rate thresholds
    pub retry_rate_warning: f64,
    pub retry_rate_critical: f64,
}

impl Default for AlertThresholds {
    fn default() -> Self {
        AlertThresholds {
            queue_size_warning: 1000,
            queue_size_critical: 5000,
            processing_time_warning: 5000,
            processing_time_critical: 15000,
            failure_rate_warning: 5.0,
            failure_rate_critical: 15.0,
            retry_rate_warning: 10.0,
            retry_rate_critical: 25.0,
        }
    }
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        MonitoringConfig {
            enable_real_time: true,
            metrics_interval_seconds: 5,
            alert_thresholds: AlertThresholds::default(),
            dashboard_interval_seconds: 1,
            enable_performance_tracking: true,
            max_history_hours: 24,
        }
    }
}

/// Monitoring metrics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringSnapshot {
    pub timestamp: DateTime<Utc>,
    pub queue_stats: QueueStats,
    pub processor_metrics: Vec<ProcessorMetrics>,
    pub retry_stats: RetryStats,
    pub transaction_stats: TransactionStats,
    pub performance_metrics: PerformanceMetrics,
    pub alerts: Vec<Alert>,
}

/// Performance metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub throughput_tps: f64, // Transactions per second
    pub average_processing_time_ms: f64,
    pub p95_processing_time_ms: f64,
    pub p99_processing_time_ms: f64,
    pub success_rate: f64,
    pub failure_rate: f64,
    pub retry_rate: f64,
    pub queue_depth: usize,
    pub worker_utilization: f64,
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
}

/// Alert information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: Uuid,
    pub level: AlertLevel,
    pub message: String,
    pub details: HashMap<String, String>,
    pub timestamp: DateTime<Utc>,
    pub resolved: bool,
    pub resolved_at: Option<DateTime<Utc>>,
}

/// Alert levels
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertLevel {
    Info,
    Warning,
    Error,
    Critical,
}

/// Transaction monitoring system
pub struct TransactionMonitor {
    config: MonitoringConfig,
    metrics_history: Arc<RwLock<Vec<MonitoringSnapshot>>>,
    active_alerts: Arc<RwLock<HashMap<Uuid, Alert>>>,
    notification_sender: mpsc::UnboundedSender<MonitoringNotification>,
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

/// Monitoring notifications
#[derive(Debug, Clone)]
pub enum MonitoringNotification {
    MetricsUpdated { snapshot: MonitoringSnapshot },
    AlertTriggered { alert: Alert },
    AlertResolved { alert_id: Uuid },
    PerformanceIssue { issue: String },
    SystemHealth { healthy: bool, issues: Vec<String> },
}

impl TransactionMonitor {
    /// Create a new transaction monitor
    pub fn new(config: MonitoringConfig) -> (Self, mpsc::UnboundedReceiver<MonitoringNotification>) {
        let (notification_sender, notification_receiver) = mpsc::unbounded_channel();
        
        let monitor = TransactionMonitor {
            config,
            metrics_history: Arc::new(RwLock::new(Vec::new())),
            active_alerts: Arc::new(RwLock::new(HashMap::new())),
            notification_sender,
            shutdown_tx: None,
        };

        (monitor, notification_receiver)
    }

    /// Start the monitoring system
    pub async fn start(&mut self) -> Result<()> {
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel();
        self.shutdown_tx = Some(shutdown_tx);

        let metrics_history = self.metrics_history.clone();
        let active_alerts = self.active_alerts.clone();
        let config = self.config.clone();
        let notification_sender = self.notification_sender.clone();

        // Spawn monitoring task
        tokio::spawn(async move {
            let mut metrics_interval = tokio::time::interval(
                std::time::Duration::from_secs(config.metrics_interval_seconds)
            );
            let mut dashboard_interval = tokio::time::interval(
                std::time::Duration::from_secs(config.dashboard_interval_seconds)
            );

            loop {
                tokio::select! {
                    _ = metrics_interval.tick() => {
                        if let Err(e) = Self::collect_metrics(
                            &metrics_history,
                            &active_alerts,
                            &config,
                            &notification_sender,
                        ).await {
                            eprintln!("Metrics collection error: {}", e);
                        }
                    }
                    _ = dashboard_interval.tick() => {
                        // Dashboard updates would be handled here
                    }
                    _ = &mut shutdown_rx => {
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    /// Stop the monitoring system
    pub async fn stop(&mut self) -> Result<()> {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }
        Ok(())
    }

    /// Get current monitoring snapshot
    pub async fn get_current_snapshot(&self) -> Result<Option<MonitoringSnapshot>> {
        let history = self.metrics_history.read().await;
        Ok(history.last().cloned())
    }

    /// Get metrics history
    pub async fn get_metrics_history(&self, limit: Option<usize>) -> Result<Vec<MonitoringSnapshot>> {
        let history = self.metrics_history.read().await;
        let mut history_vec = history.clone();
        
        // Sort by timestamp (newest first)
        history_vec.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        
        if let Some(limit) = limit {
            history_vec.truncate(limit);
        }
        
        Ok(history_vec)
    }

    /// Get active alerts
    pub async fn get_active_alerts(&self) -> Result<Vec<Alert>> {
        let alerts = self.active_alerts.read().await;
        Ok(alerts.values().cloned().collect())
    }

    /// Create manual alert
    pub async fn create_alert(&self, level: AlertLevel, message: String, details: HashMap<String, String>) -> Result<()> {
        let alert = Alert {
            id: Uuid::new_v4(),
            level,
            message,
            details,
            timestamp: Utc::now(),
            resolved: false,
            resolved_at: None,
        };

        let mut alerts = self.active_alerts.write().await;
        alerts.insert(alert.id, alert.clone());

        let _ = self.notification_sender.send(MonitoringNotification::AlertTriggered {
            alert: alert.clone(),
        });

        Ok(())
    }

    /// Resolve alert
    pub async fn resolve_alert(&self, alert_id: Uuid) -> Result<bool> {
        let mut alerts = self.active_alerts.write().await;
        
        if let Some(alert) = alerts.get_mut(&alert_id) {
            alert.resolved = true;
            alert.resolved_at = Some(Utc::now());
            
            let _ = self.notification_sender.send(MonitoringNotification::AlertResolved {
                alert_id,
            });
            
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Collect metrics and create snapshot
    async fn collect_metrics(
        metrics_history: &Arc<RwLock<Vec<MonitoringSnapshot>>>,
        active_alerts: &Arc<RwLock<HashMap<Uuid, Alert>>>,
        config: &MonitoringConfig,
        notification_sender: &mpsc::UnboundedSender<MonitoringNotification>,
    ) -> Result<()> {
        // This would collect real metrics from the actual components
        // For now, we'll create a sample snapshot
        
        let snapshot = MonitoringSnapshot {
            timestamp: Utc::now(),
            queue_stats: QueueStats::default(), // Would get from actual queue
            processor_metrics: Vec::new(),       // Would get from actual processors
            retry_stats: RetryStats::default(),  // Would get from actual retry manager
            transaction_stats: TransactionStats::default(), // Would calculate from transactions
            performance_metrics: PerformanceMetrics::default(),
            alerts: Vec::new(),
        };

        // Check for alerts
        let new_alerts = Self::check_alert_conditions(&snapshot, &config.alert_thresholds);
        
        {
            let mut alerts = active_alerts.write().await;
            for alert in new_alerts {
                alerts.insert(alert.id, alert.clone());
                let _ = notification_sender.send(MonitoringNotification::AlertTriggered {
                    alert,
                });
            }
        }

        // Add to history
        {
            let mut history = metrics_history.write().await;
            history.push(snapshot.clone());
            
            // Cleanup old history
            let cutoff = Utc::now() - Duration::hours(config.max_history_hours as i64);
            history.retain(|s| s.timestamp > cutoff);
        }

        // Send notification
        let _ = notification_sender.send(MonitoringNotification::MetricsUpdated {
            snapshot,
        });

        Ok(())
    }

    /// Check alert conditions
    fn check_alert_conditions(
        snapshot: &MonitoringSnapshot,
        thresholds: &AlertThresholds,
    ) -> Vec<Alert> {
        let mut alerts = Vec::new();

        // Check queue size
        if snapshot.queue_stats.total_queued >= thresholds.queue_size_critical {
            alerts.push(Alert {
                id: Uuid::new_v4(),
                level: AlertLevel::Critical,
                message: format!("Queue size critical: {} transactions", snapshot.queue_stats.total_queued),
                details: HashMap::from([
                    ("queue_size".to_string(), snapshot.queue_stats.total_queued.to_string()),
                    ("threshold".to_string(), thresholds.queue_size_critical.to_string()),
                ]),
                timestamp: Utc::now(),
                resolved: false,
                resolved_at: None,
            });
        } else if snapshot.queue_stats.total_queued >= thresholds.queue_size_warning {
            alerts.push(Alert {
                id: Uuid::new_v4(),
                level: AlertLevel::Warning,
                message: format!("Queue size warning: {} transactions", snapshot.queue_stats.total_queued),
                details: HashMap::from([
                    ("queue_size".to_string(), snapshot.queue_stats.total_queued.to_string()),
                    ("threshold".to_string(), thresholds.queue_size_warning.to_string()),
                ]),
                timestamp: Utc::now(),
                resolved: false,
                resolved_at: None,
            });
        }

        // Check processing time
        if let Some(avg_time) = snapshot.performance_metrics.average_processing_time_ms {
            if avg_time >= thresholds.processing_time_critical as f64 {
                alerts.push(Alert {
                    id: Uuid::new_v4(),
                    level: AlertLevel::Critical,
                    message: format!("Processing time critical: {:.2}ms", avg_time),
                    details: HashMap::from([
                        ("avg_processing_time".to_string(), avg_time.to_string()),
                        ("threshold".to_string(), thresholds.processing_time_critical.to_string()),
                    ]),
                    timestamp: Utc::now(),
                    resolved: false,
                    resolved_at: None,
                });
            } else if avg_time >= thresholds.processing_time_warning as f64 {
                alerts.push(Alert {
                    id: Uuid::new_v4(),
                    level: AlertLevel::Warning,
                    message: format!("Processing time warning: {:.2}ms", avg_time),
                    details: HashMap::from([
                        ("avg_processing_time".to_string(), avg_time.to_string()),
                        ("threshold".to_string(), thresholds.processing_time_warning.to_string()),
                    ]),
                    timestamp: Utc::now(),
                    resolved: false,
                    resolved_at: None,
                });
            }
        }

        alerts
    }

    /// Get system health status
    pub async fn get_system_health(&self) -> Result<SystemHealth> {
        let alerts = self.active_alerts.read().await;
        let critical_alerts = alerts.values().filter(|a| a.level == AlertLevel::Critical && !a.resolved).count();
        let error_alerts = alerts.values().filter(|a| a.level == AlertLevel::Error && !a.resolved).count();
        let warning_alerts = alerts.values().filter(|a| a.level == AlertLevel::Warning && !a.resolved).count();

        let health = SystemHealth {
            healthy: critical_alerts == 0 && error_alerts == 0,
            status: if critical_alerts > 0 {
                HealthStatus::Critical
            } else if error_alerts > 0 {
                HealthStatus::Error
            } else if warning_alerts > 0 {
                HealthStatus::Warning
            } else {
                HealthStatus::Healthy
            },
            active_alerts: alerts.values().cloned().collect(),
            uptime_seconds: 0, // Would calculate from start time
            last_check: Utc::now(),
        };

        Ok(health)
    }
}

/// System health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemHealth {
    pub healthy: bool,
    pub status: HealthStatus,
    pub active_alerts: Vec<Alert>,
    pub uptime_seconds: u64,
    pub last_check: DateTime<Utc>,
}

/// Health status levels
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Warning,
    Error,
    Critical,
}

/// Dashboard data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardData {
    pub current_snapshot: MonitoringSnapshot,
    pub system_health: SystemHealth,
    pub recent_transactions: Vec<Transaction>,
    pub performance_chart: Vec<PerformanceDataPoint>,
    pub alert_summary: AlertSummary,
}

/// Performance data point for charts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceDataPoint {
    pub timestamp: DateTime<Utc>,
    pub throughput: f64,
    pub processing_time: f64,
    pub success_rate: f64,
    pub queue_depth: usize,
}

/// Alert summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertSummary {
    pub total: usize,
    pub critical: usize,
    pub error: usize,
    pub warning: usize,
    pub info: usize,
    pub resolved: usize,
}

/// Dashboard provider
pub struct DashboardProvider {
    monitor: Arc<TransactionMonitor>,
}

impl DashboardProvider {
    /// Create new dashboard provider
    pub fn new(monitor: Arc<TransactionMonitor>) -> Self {
        Self { monitor }
    }

    /// Get dashboard data
    pub async fn get_dashboard_data(&self) -> Result<DashboardData> {
        let current_snapshot = self.monitor.get_current_snapshot().await?
            .ok_or_else(|| anyhow::anyhow!("No monitoring data available"))?;
        
        let system_health = self.monitor.get_system_health().await?;
        
        let performance_chart = self.monitor.get_metrics_history(Some(100)).await?
            .into_iter()
            .map(|snapshot| PerformanceDataPoint {
                timestamp: snapshot.timestamp,
                throughput: snapshot.performance_metrics.throughput_tps,
                processing_time: snapshot.performance_metrics.average_processing_time_ms,
                success_rate: snapshot.performance_metrics.success_rate,
                queue_depth: snapshot.performance_metrics.queue_depth,
            })
            .collect();

        let active_alerts = self.monitor.get_active_alerts().await?;
        let alert_summary = AlertSummary {
            total: active_alerts.len(),
            critical: active_alerts.iter().filter(|a| a.level == AlertLevel::Critical).count(),
            error: active_alerts.iter().filter(|a| a.level == AlertLevel::Error).count(),
            warning: active_alerts.iter().filter(|a| a.level == AlertLevel::Warning).count(),
            info: active_alerts.iter().filter(|a| a.level == AlertLevel::Info).count(),
            resolved: active_alerts.iter().filter(|a| a.resolved).count(),
        };

        Ok(DashboardData {
            current_snapshot,
            system_health,
            recent_transactions: Vec::new(), // Would get from state manager
            performance_chart,
            alert_summary,
        })
    }
}

// Re-export TransactionStats from state module
use crate::transaction_engine::state::TransactionStats;

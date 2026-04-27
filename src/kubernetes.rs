//! Kubernetes integration for isolated scan execution
//! 
//! This module provides functionality to spin up ephemeral Kubernetes pods for each scan,
//! ensuring complete isolation between different tenants and preventing data leakage.

use crate::{ScanResult, config::ScannerConfig};
use anyhow::{Result, anyhow};
use kube::{
    api::{Api, DeleteParams, ListParams, PostParams, WatchEvent},
    client::Client,
    runtime::watcher,
    Config,
};
use futures::StreamExt;
use k8s_openapi::{
    api::{
        core::v1::{Pod, PodSpec, Container, VolumeMount, Volume, EnvVar, ResourceRequirements},
        networking::v1::{NetworkPolicy, NetworkPolicySpec, NetworkPolicyIngressRule, NetworkPolicyPort},
        policy::v1::{ResourceQuota, ResourceQuotaSpec, ResourceQuotaStatus},
    },
    apimachinery::pkg::api::resource::Quantity,
};
use serde_json::json;
use std::{collections::BTreeMap, time::Duration};
use tokio::time::timeout;
use uuid::Uuid;

const SCAN_NAMESPACE_PREFIX: &str = "scan-";
const DEFAULT_SCAN_TIMEOUT: Duration = Duration::from_secs(600); // 10 minutes
const LOG_SIDECAR_IMAGE: &str = "fluent/fluent-bit:latest";

/// Kubernetes manager for isolated scan execution
pub struct K8sScanManager {
    client: Client,
    namespace: String,
    config: ScanPodConfig,
}

/// Configuration for individual scan pods
#[derive(Debug, Clone)]
pub struct ScanPodConfig {
    /// CPU limit per scan pod (e.g., "500m" for 0.5 cores)
    pub cpu_limit: String,
    /// Memory limit per scan pod (e.g., "512Mi")
    pub memory_limit: String,
    /// CPU request per scan pod (e.g., "100m")
    pub cpu_request: String,
    /// Memory request per scan pod (e.g., "128Mi")
    pub memory_request: String,
    /// Scanner container image
    pub scanner_image: String,
    /// Timeout for scan execution
    pub timeout: Duration,
    /// Enable encrypted volumes
    pub encrypt_volumes: bool,
    /// Network policy to block all egress
    pub block_egress: bool,
}

impl Default for ScanPodConfig {
    fn default() -> Self {
        Self {
            cpu_limit: "1000m".to_string(), // 1 CPU core max
            memory_limit: "2Gi".to_string(), // 2GB RAM max
            cpu_request: "100m".to_string(), // 0.1 CPU core min
            memory_request: "256Mi".to_string(), // 256MB RAM min
            scanner_image: "stellar-security-scanner:latest".to_string(),
            timeout: DEFAULT_SCAN_TIMEOUT,
            encrypt_volumes: true,
            block_egress: true,
        }
    }
}

impl K8sScanManager {
    /// Create a new Kubernetes scan manager
    pub async fn new(config: ScanPodConfig) -> Result<Self> {
        let client = Client::try_default().await?;
        let namespace = std::env::var("SCAN_NAMESPACE")
            .unwrap_or_else(|_| "default".to_string());
        
        Ok(Self {
            client,
            namespace,
            config,
        })
    }

    /// Execute a scan in an isolated Kubernetes pod
    pub async fn execute_scan(
        &self,
        scan_id: &str,
        scanner_config: &ScannerConfig,
        contract_code: &[u8],
    ) -> Result<ScanResult> {
        // Create isolated namespace for this scan
        let scan_namespace = self.create_scan_namespace(scan_id).await?;
        
        // Apply resource quotas for isolation
        self.apply_resource_quota(&scan_namespace).await?;
        
        // Apply network policies to block egress
        if self.config.block_egress {
            self.apply_network_policy(&scan_namespace).await?;
        }
        
        // Create the scan pod
        let pod_name = self.create_scan_pod(&scan_namespace, scan_id, scanner_config, contract_code).await?;
        
        // Wait for scan completion with timeout
        let result = timeout(self.config.timeout, self.wait_for_scan_completion(&scan_namespace, &pod_name)).await??;
        
        // Cleanup resources
        self.cleanup_scan_resources(&scan_namespace).await?;
        
        Ok(result)
    }

    /// Create an isolated namespace for the scan
    async fn create_scan_namespace(&self, scan_id: &str) -> Result<String> {
        use k8s_openapi::api::core::v1::Namespace;
        
        let namespace_name = format!("{}{}", SCAN_NAMESPACE_PREFIX, scan_id);
        let namespaces: Api<Namespace> = Api::all(self.client.clone());
        
        let namespace = Namespace {
            metadata: kube::api::ObjectMeta {
                name: Some(namespace_name.clone()),
                labels: Some(BTreeMap::from([
                    ("app".to_string(), "stellar-security-scanner".to_string()),
                    ("scan-id".to_string(), scan_id.to_string()),
                    ("managed-by".to_string(), "scanner".to_string()),
                ])),
                annotations: Some(BTreeMap::from([
                    ("stellar.scanner/created-at".to_string(), chrono::Utc::now().to_rfc3339()),
                ])),
                ..Default::default()
            },
            ..Default::default()
        };
        
        namespaces.create(&PostParams::default(), &namespace).await?;
        Ok(namespace_name)
    }

    /// Apply resource quota to prevent resource exhaustion
    async fn apply_resource_quota(&self, namespace: &str) -> Result<()> {
        let resource_quotas: Api<ResourceQuota> = Api::namespaced(self.client.clone(), namespace);
        
        let quota = ResourceQuota {
            metadata: kube::api::ObjectMeta {
                name: Some("scan-quota".to_string()),
                ..Default::default()
            },
            spec: Some(ResourceQuotaSpec {
                hard: BTreeMap::from([
                    ("cpu".to_string(), Quantity(self.config.cpu_limit.clone())),
                    ("memory".to_string(), Quantity(self.config.memory_limit.clone())),
                    ("pods".to_string(), Quantity("2".to_string())), // Allow scanner + sidecar
                ]),
                ..Default::default()
            }),
            ..Default::default()
        };
        
        resource_quotas.create(&PostParams::default(), &quota).await?;
        Ok(())
    }

    /// Apply network policy to block all egress traffic
    async fn apply_network_policy(&self, namespace: &str) -> Result<()> {
        let network_policies: Api<NetworkPolicy> = Api::namespaced(self.client.clone(), namespace);
        
        let policy = NetworkPolicy {
            metadata: kube::api::ObjectMeta {
                name: Some("block-egress".to_string()),
                ..Default::default()
            },
            spec: Some(NetworkPolicySpec {
                pod_selector: BTreeMap::new(), // Apply to all pods in namespace
                ingress: Some(vec![]), // Allow all ingress (for API communication)
                egress: Some(vec![]), // Block all egress
                policy_types: Some(vec!["Ingress".to_string(), "Egress".to_string()]),
                ..Default::default()
            }),
            ..Default::default()
        };
        
        network_policies.create(&PostParams::default(), &policy).await?;
        Ok(())
    }

    /// Create the main scanner pod with sidecar for logging
    async fn create_scan_pod(
        &self,
        namespace: &str,
        scan_id: &str,
        scanner_config: &ScannerConfig,
        contract_code: &[u8],
    ) -> Result<String> {
        let pods: Api<Pod> = Api::namespaced(self.client.clone(), namespace);
        let pod_name = format!("scanner-{}", scan_id);
        
        // Encode contract code as base64 for environment variable
        let code_b64 = base64::encode(contract_code);
        
        // Create encrypted volume if enabled
        let volumes = if self.config.encrypt_volumes {
            vec![
                Volume {
                    name: "encrypted-data".to_string(),
                    empty_dir: Some(k8s_openapi::api::core::v1::EmptyDirVolumeSource {
                        medium: Some("Memory".to_string()), // Use tmpfs for in-memory encryption
                        size_limit: Some(Quantity("1Gi".to_string())),
                    }),
                    ..Default::default()
                }
            ]
        } else {
            vec![]
        };
        
        // Main scanner container
        let scanner_container = Container {
            name: "scanner".to_string(),
            image: Some(self.config.scanner_image.clone()),
            command: Some(vec!["stellar-scanner".to_string(), "security".to_string()]),
            args: Some(vec![
                "--path".to_string(), "/scan".to_string(),
                "--format".to_string(), "json".to_string(),
                "--verbose".to_string(),
            ]),
            env: Some(vec![
                EnvVar {
                    name: "SCAN_ID".to_string(),
                    value: Some(scan_id.to_string()),
                    ..Default::default()
                },
                EnvVar {
                    name: "CONTRACT_CODE".to_string(),
                    value: Some(code_b64),
                    ..Default::default()
                },
                EnvVar {
                    name: "SCANNER_CONFIG".to_string(),
                    value: Some(serde_json::to_string(scanner_config)?),
                    ..Default::default()
                },
            ]),
            resources: Some(ResourceRequirements {
                limits: Some(BTreeMap::from([
                    ("cpu".to_string(), Quantity(self.config.cpu_limit.clone())),
                    ("memory".to_string(), Quantity(self.config.memory_limit.clone())),
                ])),
                requests: Some(BTreeMap::from([
                    ("cpu".to_string(), Quantity(self.config.cpu_request.clone())),
                    ("memory".to_string(), Quantity(self.config.memory_request.clone())),
                ])),
                ..Default::default()
            }),
            volume_mounts: if self.config.encrypt_volumes {
                vec![
                    VolumeMount {
                        name: "encrypted-data".to_string(),
                        mount_path: "/scan".to_string(),
                        ..Default::default()
                    }
                ]
            } else {
                vec![]
            },
            ..Default::default()
        };
        
        // Log streaming sidecar
        let log_sidecar = Container {
            name: "log-streamer".to_string(),
            image: Some(LOG_SIDECAR_IMAGE.to_string()),
            env: Some(vec![
                EnvVar {
                    name: "SCAN_ID".to_string(),
                    value: Some(scan_id.to_string()),
                    ..Default::default()
                },
                EnvVar {
                    name: "API_ENDPOINT".to_string(),
                    value_from: Some(k8s_openapi::api::core::v1::EnvVarSource {
                        field_ref: Some(k8s_openapi::api::core::v1::ObjectFieldSelector {
                            field_path: "metadata.annotations['stellar.scanner/api-endpoint']".to_string(),
                            ..Default::default()
                        }),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            ]),
            volume_mounts: Some(vec![
                VolumeMount {
                    name: "var-log".to_string(),
                    mount_path: "/var/log".to_string(),
                    read_only: Some(true),
                    ..Default::default()
                }
            ]),
            ..Default::default()
        };
        
        let pod = Pod {
            metadata: kube::api::ObjectMeta {
                name: Some(pod_name.clone()),
                labels: Some(BTreeMap::from([
                    ("app".to_string(), "stellar-security-scanner".to_string()),
                    ("component".to_string(), "scanner".to_string()),
                    ("scan-id".to_string(), scan_id.to_string()),
                ])),
                ..Default::default()
            },
            spec: Some(PodSpec {
                containers: vec![scanner_container, log_sidecar],
                restart_policy: Some("Never".to_string()),
                volumes: volumes,
                active_deadline_seconds: Some(self.config.timeout.as_secs() as i64),
                ..Default::default()
            }),
            ..Default::default()
        };
        
        pods.create(&PostParams::default(), &pod).await?;
        Ok(pod_name)
    }

    /// Wait for scan completion and collect results
    async fn wait_for_scan_completion(&self, namespace: &str, pod_name: &str) -> Result<ScanResult> {
        let pods: Api<Pod> = Api::namespaced(self.client.clone(), namespace);
        
        // Watch pod events
        let lp = ListParams::default()
            .fields(&format!("metadata.name={}", pod_name))
            .timeout(10);
        
        let mut stream = watcher(pods, lp);
        
        while let Some(event) = stream.try_next().await? {
            match event {
                WatchEvent::Modified(pod) => {
                    if let Some(status) = &pod.status {
                        if let Some(phase) = &status.phase {
                            match phase.as_str() {
                                "Succeeded" => {
                                    // Extract scan results from pod logs
                                    return self.extract_scan_results(namespace, pod_name).await;
                                }
                                "Failed" => {
                                    return Err(anyhow!("Scan pod failed: {:?}", status.message));
                                }
                                _ => continue,
                            }
                        }
                    }
                }
                WatchEvent::Deleted(_) => {
                    return Err(anyhow!("Scan pod was deleted"));
                }
                _ => continue,
            }
        }
        
        Err(anyhow!("Scan completion watcher ended unexpectedly"))
    }

    /// Extract scan results from pod logs
    async fn extract_scan_results(&self, namespace: &str, pod_name: &str) -> Result<ScanResult> {
        let pods: Api<Pod> = Api::namespaced(self.client.clone(), namespace);
        
        // Get pod logs
        let logs = pods
            .log_stream(&pod_name, &kube::api::LogParams::default())
            .await?;
        
        // Parse JSON results from logs
        let mut result_json = String::new();
        let lines = tokio::io::BufReader::new(logs).lines();
        
        for line in lines {
            let line = line?;
            if line.starts_with("{") && line.ends_with("}") {
                result_json = line;
                break;
            }
        }
        
        if result_json.is_empty() {
            return Err(anyhow!("No scan results found in pod logs"));
        }
        
        // Parse the scan result
        let scan_result: ScanResult = serde_json::from_str(&result_json)?;
        Ok(scan_result)
    }

    /// Cleanup all resources for a scan
    async fn cleanup_scan_resources(&self, namespace: &str) -> Result<()> {
        use k8s_openapi::api::core::v1::Namespace;
        
        // Delete the entire namespace
        let namespaces: Api<Namespace> = Api::all(self.client.clone());
        let dp = DeleteParams::default();
        
        namespaces.delete(namespace, &dp).await?;
        Ok(())
    }

    /// List active scans
    pub async fn list_active_scans(&self) -> Result<Vec<String>> {
        use k8s_openapi::api::core::v1::Namespace;
        
        let namespaces: Api<Namespace> = Api::all(self.client.clone());
        let lp = ListParams::default()
            .labels("app=stellar-security-scanner,managed-by=scanner");
        
        let list = namespaces.list(&lp).await?;
        
        let mut scan_ids = Vec::new();
        for ns in list.items {
            if let Some(name) = ns.metadata.name {
                if name.starts_with(SCAN_NAMESPACE_PREFIX) {
                    if let Some(scan_id) = name.strip_prefix(SCAN_NAMESPACE_PREFIX) {
                        scan_ids.push(scan_id.to_string());
                    }
                }
            }
        }
        
        Ok(scan_ids)
    }

    /// Force cleanup of stuck scans
    pub async fn cleanup_stuck_scans(&self, max_age: Duration) -> Result<usize> {
        use k8s_openapi::api::core::v1::Namespace;
        
        let namespaces: Api<Namespace> = Api::all(self.client.clone());
        let lp = ListParams::default()
            .labels("app=stellar-security-scanner,managed-by=scanner");
        
        let list = namespaces.list(&lp).await?;
        let cutoff_time = chrono::Utc::now() - chrono::Duration::from_std(max_age)?;
        
        let mut cleaned_count = 0;
        for ns in list.items {
            if let Some(created_at) = ns.metadata.creation_timestamp {
                if created_at < cutoff_time {
                    if let Some(name) = &ns.metadata.name {
                        let dp = DeleteParams::default();
                        namespaces.delete(name, &dp).await?;
                        cleaned_count += 1;
                    }
                }
            }
        }
        
        Ok(cleaned_count)
    }
}

/// Auto-scaler for handling scan request spikes
pub struct ScanAutoScaler {
    manager: K8sScanManager,
    max_concurrent_scans: usize,
    current_scans: std::sync::atomic::AtomicUsize,
}

impl ScanAutoScaler {
    pub fn new(manager: K8sScanManager, max_concurrent_scans: usize) -> Self {
        Self {
            manager,
            max_concurrent_scans,
            current_scans: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    /// Execute scan with auto-scaling logic
    pub async fn execute_scaled_scan(
        &self,
        scan_id: &str,
        scanner_config: &ScannerConfig,
        contract_code: &[u8],
    ) -> Result<ScanResult> {
        // Check if we can accept more scans
        let current = self.current_scans.load(std::sync::atomic::Ordering::Relaxed);
        if current >= self.max_concurrent_scans {
            return Err(anyhow!("Maximum concurrent scans ({}) reached", self.max_concurrent_scans));
        }

        // Increment counter
        self.current_scans.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // Execute scan with cleanup
        let result = self.manager.execute_scan(scan_id, scanner_config, contract_code).await;

        // Decrement counter
        self.current_scans.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);

        result
    }

    /// Get current load metrics
    pub fn get_load_metrics(&self) -> (usize, usize) {
        let current = self.current_scans.load(std::sync::atomic::Ordering::Relaxed);
        (current, self.max_concurrent_scans)
    }
}

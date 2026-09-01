//! Example usage of Kubernetes-based isolated scanning
//! 
//! This example demonstrates how to use the stellar-security-scanner
//! with complete Kubernetes isolation for tenant separation.

use anyhow::Result;
use std::time::Duration;
use stellar_security_scanner::{
    kubernetes::{K8sScanManager, ScanPodConfig, ScanAutoScaler},
    config::ScannerConfig,
};
use tokio;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Stellar Security Scanner - Kubernetes Isolated Example");

    // 1. Configure scan pod settings
    let scan_config = ScanPodConfig {
        cpu_limit: "500m".to_string(),      // 0.5 CPU cores max
        memory_limit: "1Gi".to_string(),    // 1GB RAM max
        cpu_request: "100m".to_string(),     // 0.1 CPU cores min
        memory_request: "256Mi".to_string(), // 256MB RAM min
        scanner_image: "stellar-security-scanner:latest".to_string(),
        timeout: Duration::from_secs(300),   // 5 minute timeout
        encrypt_volumes: true,               // Enable encryption
        block_egress: true,                  // Block all external traffic
    };

    // 2. Create Kubernetes scan manager
    let manager = K8sScanManager::new(scan_config).await?;
    println!("✅ Kubernetes manager initialized");

    // 3. Setup auto-scaler for handling multiple concurrent scans
    let auto_scaler = ScanAutoScaler::new(manager.clone(), 5); // Max 5 concurrent scans
    println!("📊 Auto-scaler configured (max concurrent: 5)");

    // 4. Load scanner configuration
    let scanner_config = ScannerConfig::default();

    // 5. Example contract code (in practice, load from file)
    let contract_code = r#"
        pub struct TokenContract {
            admin: Address,
            total_supply: i128,
            balances: Map<Address, i128>,
        }

        impl TokenContract {
            pub fn new(admin: Address) -> Self {
                Self {
                    admin,
                    total_supply: 0,
                    balances: Map::new(),
                }
            }

            pub fn mint(&mut self, to: Address, amount: i128) {
                // VULNERABILITY: No access control!
                self.total_supply += amount;
                let balance = self.balances.get(to).unwrap_or(0);
                self.balances.set(to, balance + amount);
            }

            pub fn transfer(&mut self, from: Address, to: Address, amount: i128) -> Result<(), Error> {
                // VULNERABILITY: No overflow protection!
                let from_balance = self.balances.get(from).unwrap_or(0);
                let to_balance = self.balances.get(to).unwrap_or(0);
                
                self.balances.set(from, from_balance - amount);
                self.balances.set(to, to_balance + amount);
                Ok(())
            }
        }
    "#;

    // 6. Execute multiple scans concurrently with auto-scaling
    let mut scan_tasks = Vec::new();
    
    for i in 1..=3 {
        let scan_id = format!("example-scan-{}", i);
        let contract_bytes = contract_code.as_bytes().to_vec();
        let config = scanner_config.clone();
        let scaler = auto_scaler.clone();

        let task = tokio::spawn(async move {
            println!("🔍 Starting scan {}...", scan_id);
            
            let start_time = std::time::Instant::now();
            match scaler.execute_scaled_scan(&scan_id, &config, &contract_bytes).await {
                Ok(result) => {
                    let duration = start_time.elapsed();
                    println!("✅ Scan {} completed in {:.2}s", scan_id, duration.as_secs_f64());
                    println!("   Vulnerabilities found: {}", result.vulnerabilities.len());
                    println!("   Invariant violations: {}", result.invariant_violations.len());
                    
                    // Print specific findings
                    for vuln in &result.vulnerabilities {
                        println!("   🚨 {}: {}", vuln, vuln.description());
                    }
                    
                    Ok::<_, anyhow::Error>(result)
                }
                Err(e) => {
                    println!("❌ Scan {} failed: {}", scan_id, e);
                    Err(e)
                }
            }
        });
        
        scan_tasks.push(task);
    }

    // 7. Wait for all scans to complete
    println!("⏳ Waiting for scans to complete...");
    let mut completed = 0;
    let mut failed = 0;

    for task in scan_tasks {
        match task.await? {
            Ok(_) => completed += 1,
            Err(_) => failed += 1,
        }
    }

    println!("📈 Scan Summary:");
    println!("   ✅ Completed: {}", completed);
    println!("   ❌ Failed: {}", failed);

    // 8. Monitor system load
    let (current_load, max_capacity) = auto_scaler.get_load_metrics();
    println!("📊 Current System Load: {}/{}", current_load, max_capacity);

    // 9. List remaining active scans
    let active_scans = manager.list_active_scans().await?;
    if !active_scans.is_empty() {
        println!("🔄 Active scans: {:?}", active_scans);
    }

    // 10. Cleanup demonstration (optional)
    println!("🧹 Performing cleanup of old scans...");
    let cleaned_count = manager.cleanup_stuck_scans(Duration::from_secs(1800)).await?; // 30 minutes
    if cleaned_count > 0 {
        println!("   Cleaned up {} stuck scans", cleaned_count);
    } else {
        println!("   No stuck scans found");
    }

    println!("🎉 Example completed successfully!");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_kubernetes_manager_creation() {
        let config = ScanPodConfig::default();
        
        // This test requires a running Kubernetes cluster
        // In CI/CD, use a test cluster like kind or minikube
        if std::env::var("KUBE_TEST").is_ok() {
            let manager = K8sScanManager::new(config).await;
            assert!(manager.is_ok(), "Failed to create Kubernetes manager");
        }
    }

    #[tokio::test]
    async fn test_auto_scaler_load_metrics() {
        let config = ScanPodConfig::default();
        let max_concurrent = 10;
        
        if std::env::var("KUBE_TEST").is_ok() {
            let manager = K8sScanManager::new(config).await.unwrap();
            let scaler = ScanAutoScaler::new(manager, max_concurrent);
            
            let (current, max) = scaler.get_load_metrics();
            assert_eq!(max, max_concurrent);
            assert!(current <= max);
        }
    }

    #[test]
    fn test_scan_pod_config_defaults() {
        let config = ScanPodConfig::default();
        
        assert_eq!(config.cpu_limit, "1000m");
        assert_eq!(config.memory_limit, "2Gi");
        assert_eq!(config.timeout, Duration::from_secs(600));
        assert!(config.encrypt_volumes);
        assert!(config.block_egress);
    }
}

# Security scanning infrastructure - Terraform configuration
# This file defines security scanning infrastructure for the CI/CD pipeline

terraform {
  required_version = ">= 1.0"
  required_providers {
    kubernetes = {
      source  = "hashicorp/kubernetes"
      version = "~> 2.0"
    }
    docker = {
      source  = "kreuzwerker/docker"
      version = "~> 3.0"
    }
  }
}

# Security scanning namespace
resource "kubernetes_namespace" "security_scanner" {
  metadata {
    name = "security-scanning"
    labels = {
      name        = "security-scanning"
      environment = "ci"
      managed-by  = "terraform"
    }
  }
}

# Security scanner service account (least privilege)
resource "kubernetes_service_account" "scanner_sa" {
  metadata {
    name      = "security-scanner"
    namespace = kubernetes_namespace.security_scanner.metadata[0].name
    annotations = {
      "kubernetes.io/enforce-mountable-secrets" = "true"
    }
  }
  automount_service_account_token = false
}

# Pod Security Policy for scanner pods
resource "kubernetes_pod_security_policy" "scanner_psp" {
  metadata {
    name = "security-scanner-restricted"
    annotations = {
      "seccomp.security.alpha.kubernetes.io/allowedProfileNames" = "runtime/default"
      "apparmor.security.beta.kubernetes.io/allowedProfileNames"  = "runtime/default"
      "seccomp.security.alpha.kubernetes.io/defaultProfileName"  = "runtime/default"
      "apparmor.security.beta.kubernetes.io/defaultProfileName"  = "runtime/default"
    }
  }
  spec {
    privileged                 = false
    allow_privilege_escalation = false
    read_only_root_filesystem  = true
    run_as_user {
      rule = "MustRunAsNonRoot"
    }
    run_as_group {
      rule = "MustRunAs"
      ranges {
        min = 1000
        max = 65535
      }
    }
    se_linux {
      rule = "RunAsAny"
    }
    supplemental_groups {
      rule = "MustRunAs"
      ranges {
        min = 1000
        max = 65535
      }
    }
    fs_group {
      rule = "MustRunAs"
      ranges {
        min = 1000
        max = 65535
      }
    }
    volumes = ["configMap", "secret", "emptyDir", "persistentVolumeClaim"]
    allowed_volumes = ["configMap", "secret", "emptyDir"]
  }
}

# Security scan job template
resource "kubernetes_cron_job" "security_scan" {
  metadata {
    name      = "weekly-security-scan"
    namespace = kubernetes_namespace.security_scanner.metadata[0].name
  }
  spec {
    concurrency_policy            = "Forbid"
    failed_jobs_history_limit     = 3
    successful_jobs_history_limit = 3
    schedule                      = "0 6 * * 1"
    job_template {
      metadata {}
      spec {
        template {
          metadata {}
          spec {
            service_account_name            = kubernetes_service_account.scanner_sa.metadata[0].name
            automount_service_account_token = false
            restart_policy                  = "OnFailure"
            security_context {
              run_as_non_root = true
              run_as_user     = 1000
              run_as_group    = 1000
              fs_group        = 1000
            }
            container {
              name  = "security-scanner"
              image = "connect-boiz/soroban-security-scanner:latest"
              args  = ["scan", "--format", "json", "--output", "/reports/scan-result.json"]
              security_context {
                allow_privilege_escalation = false
                privileged                 = false
                read_only_root_filesystem  = true
                capabilities {
                  drop = ["ALL"]
                }
              }
              resource {
                limits = {
                  cpu    = "2"
                  memory = "4Gi"
                }
                requests = {
                  cpu    = "500m"
                  memory = "1Gi"
                }
              }
              volume_mount {
                name      = "reports"
                mount_path = "/reports"
              }
            }
            volume {
              name = "reports"
              empty_dir {}
            }
          }
        }
      }
    }
  }
}

# Network policy to restrict scanner pods
resource "kubernetes_network_policy" "scanner_network_policy" {
  metadata {
    name      = "security-scanner-network-policy"
    namespace = kubernetes_namespace.security_scanner.metadata[0].name
  }
  spec {
    policy_types = ["Ingress", "Egress"]
    pod_selector {
      match_labels = {
        "app" = "security-scanner"
      }
    }
    ingress {
      from {
        namespace_selector {
          match_labels = {
            "kubernetes.io/metadata.name" = "security-scanning"
          }
        }
      }
    }
    egress {
      to {
        ip_block {
          cidr = "0.0.0.0/0"
          except = [
            "10.0.0.0/8",
            "172.16.0.0/12",
            "192.168.0.0/16"
          ]
        }
      }
    }
  }
}

# Outputs for integration with CI/CD
output "scanning_namespace" {
  value = kubernetes_namespace.security_scanner.metadata[0].name
}

output "scan_service_account" {
  value = kubernetes_service_account.scanner_sa.metadata[0].name
}

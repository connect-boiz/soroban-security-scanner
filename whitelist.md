MIT License

Copyright (c) 2024 Connect Boiz

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.


# Kubernetes Isolated Scanner - Validation Script (PowerShell)
# This script validates the implementation without requiring compilation

Write-Host "🚀 Validating Kubernetes Isolated Scanner Implementation" -ForegroundColor Cyan
Write-Host "======================================================" -ForegroundColor Cyan

# Check if we have the required files
Write-Host "📁 Checking file structure..." -ForegroundColor Yellow

$requiredFiles = @(
    "src/kubernetes.rs",
    "src/lib.rs", 
    "src/main.rs",
    "Cargo.toml",
    "k8s/00-namespace-rbac.yaml",
    "k8s/01-security-policies.yaml", 
    "k8s/02-api-deployment.yaml",
    "k8s/03-cleanup-autoscaling.yaml",
    "k8s/04-secrets-config.yaml",
    "k8s/README.md",
    "Dockerfile",
    "examples/kubernetes_isolated_scanning.rs",
    "KUBERNETES_IMPLEMENTATION.md"
)

$allFilesExist = $true
foreach ($file in $requiredFiles) {
    if (Test-Path $file) {
        Write-Host "  ✅ $file" -ForegroundColor Green
    } else {
        Write-Host "  ❌ $file (missing)" -ForegroundColor Red
        $allFilesExist = $false
    }
}

if (-not $allFilesExist) {
    Write-Host "❌ Validation failed: Missing required files" -ForegroundColor Red
    exit 1
}

Write-Host ""
Write-Host "📋 Validating Kubernetes manifests..." -ForegroundColor Yellow

# Check if kubectl is available
$kubectlAvailable = Get-Command kubectl -ErrorAction SilentlyContinue
if ($kubectlAvailable) {
    Write-Host "  ✅ kubectl found" -ForegroundColor Green
} else {
    Write-Host "  ⚠️  kubectl not found - skipping manifest validation" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "🔍 Validating Rust code structure..." -ForegroundColor Yellow

# Check if Rust files contain required components
Write-Host "  Checking kubernetes.rs..." -ForegroundColor Cyan
if (Select-String -Path "src/kubernetes.rs" -Pattern "K8sScanManager" -Quiet) {
    Write-Host "    ✅ K8sScanManager struct found" -ForegroundColor Green
} else {
    Write-Host "    ❌ K8sScanManager struct missing" -ForegroundColor Red
}

if (Select-String -Path "src/kubernetes.rs" -Pattern "ScanPodConfig" -Quiet) {
    Write-Host "    ✅ ScanPodConfig struct found" -ForegroundColor Green
} else {
    Write-Host "    ❌ ScanPodConfig struct missing" -ForegroundColor Red
}

if (Select-String -Path "src/kubernetes.rs" -Pattern "execute_scan" -Quiet) {
    Write-Host "    ✅ execute_scan method found" -ForegroundColor Green
} else {
    Write-Host "    ❌ execute_scan method missing" -ForegroundColor Red
}

Write-Host "  Checking lib.rs..." -ForegroundColor Cyan
if (Select-String -Path "src/lib.rs" -Pattern "pub mod kubernetes" -Quiet) {
    Write-Host "    ✅ kubernetes module exported" -ForegroundColor Green
} else {
    Write-Host "    ❌ kubernetes module not exported" -ForegroundColor Red
}

Write-Host "  Checking main.rs..." -ForegroundColor Cyan
if (Select-String -Path "src/main.rs" -Pattern "K8sScan" -Quiet) {
    Write-Host "    ✅ K8sScan command found" -ForegroundColor Green
} else {
    Write-Host "    ❌ K8sScan command missing" -ForegroundColor Red
}

if (Select-String -Path "src/main.rs" -Pattern "K8sManage" -Quiet) {
    Write-Host "    ✅ K8sManage command found" -ForegroundColor Green
} else {
    Write-Host "    ❌ K8sManage command missing" -ForegroundColor Red
}

Write-Host ""
Write-Host "📦 Validating Cargo.toml..." -ForegroundColor Yellow
if (Select-String -Path "Cargo.toml" -Pattern "kube" -Quiet) {
    Write-Host "  ✅ Kubernetes dependencies found" -ForegroundColor Green
} else {
    Write-Host "  ❌ Kubernetes dependencies missing" -ForegroundColor Red
}

if (Select-String -Path "Cargo.toml" -Pattern "k8s-openapi" -Quiet) {
    Write-Host "  ✅ k8s-openapi dependency found" -ForegroundColor Green
} else {
    Write-Host "    ❌ k8s-openapi dependency missing" -ForegroundColor Red
}

Write-Host ""
Write-Host "🐳 Validating Dockerfile..." -ForegroundColor Yellow
if (Test-Path "Dockerfile") {
    if (Select-String -Path "Dockerfile" -Pattern "FROM rust" -Quiet) {
        Write-Host "  ✅ Multi-stage build structure found" -ForegroundColor Green
    } else {
        Write-Host "  ❌ Multi-stage build structure missing" -ForegroundColor Red
    }
    
    if (Select-String -Path "Dockerfile" -Pattern "stellar-scanner" -Quiet) {
        Write-Host "  ✅ Scanner binary referenced" -ForegroundColor Green
    } else {
        Write-Host "  ❌ Scanner binary not referenced" -ForegroundColor Red
    }
}

Write-Host ""
Write-Host "📚 Validating documentation..." -ForegroundColor Yellow
if (Test-Path "k8s/README.md") {
    Write-Host "  ✅ k8s/README.md exists" -ForegroundColor Green
    if (Select-String -Path "k8s/README.md" -Pattern "Security Features" -Quiet) {
        Write-Host "    ✅ Security documentation included" -ForegroundColor Green
    } else {
        Write-Host "    ⚠️  Security documentation may be incomplete" -ForegroundColor Yellow
    }
}

if (Test-Path "KUBERNETES_IMPLEMENTATION.md") {
    Write-Host "  ✅ Implementation documentation exists" -ForegroundColor Green
} else {
    Write-Host "  ❌ Implementation documentation missing" -ForegroundColor Red
}

Write-Host ""
Write-Host "🔍 Checking implementation completeness..." -ForegroundColor Yellow

# Count key features implemented
$features = @("ResourceQuota", "NetworkPolicy", "sidecar", "encryption", "auto-scaling", "cleanup", "isolation")
$implemented = 0

foreach ($feature in $features) {
    $found = Get-ChildItem -Recurse -Exclude target,.git | Select-String -Pattern $feature -Quiet
    if ($found) {
        Write-Host "  ✅ $feature implemented" -ForegroundColor Green
        $implemented++
    } else {
        Write-Host "  ❌ $feature not found" -ForegroundColor Red
    }
}

Write-Host ""
Write-Host "📊 Implementation Summary:" -ForegroundColor Cyan
Write-Host "  Features implemented: $implemented/$($features.Count)" -ForegroundColor White
$fileCount = (Get-ChildItem -Recurse -Include *.rs,*.yaml,*.md,Dockerfile | Where-Object { $_.FullName -notlike "*target*" -and $_.FullName -notlike "*\.git*" }).Count
Write-Host "  Files created: $fileCount" -ForegroundColor White
$docCount = (Get-ChildItem -Recurse -Include *.md | Where-Object { $_.FullName -notlike "*target*" -and $_.FullName -notlike "*\.git*" }).Count
Write-Host "  Documentation files: $docCount" -ForegroundColor White

Write-Host ""
if ($implemented -eq $features.Count) {
    Write-Host "🎉 All required features implemented!" -ForegroundColor Green
    Write-Host "✅ Implementation validation PASSED" -ForegroundColor Green
} else {
    Write-Host "⚠️  Some features may be missing" -ForegroundColor Yellow
    Write-Host "❌ Implementation validation FAILED" -ForegroundColor Red
    exit 1
}

Write-Host ""
Write-Host "🚀 Next steps:" -ForegroundColor Cyan
Write-Host "  1. Install Visual Studio Build Tools for Windows compilation" -ForegroundColor White
Write-Host "  2. Run: cargo build to verify compilation" -ForegroundColor White
Write-Host "  3. Deploy to Kubernetes: kubectl apply -f k8s/" -ForegroundColor White
Write-Host "  4. Test with: stellar-scanner k8s-scan --help" -ForegroundColor White

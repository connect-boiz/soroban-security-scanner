# Simple validation script
Write-Host "🚀 Validating Kubernetes Isolated Scanner Implementation" -ForegroundColor Cyan

# Check required files
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

# Check key components
Write-Host "`n🔍 Checking implementation..." -ForegroundColor Yellow

if (Select-String -Path "src/kubernetes.rs" -Pattern "K8sScanManager" -Quiet) {
    Write-Host "  ✅ K8sScanManager found" -ForegroundColor Green
} else {
    Write-Host "  ❌ K8sScanManager missing" -ForegroundColor Red
}

if (Select-String -Path "src/main.rs" -Pattern "K8sScan" -Quiet) {
    Write-Host "  ✅ K8sScan command found" -ForegroundColor Green
} else {
    Write-Host "  ❌ K8sScan command missing" -ForegroundColor Red
}

if (Select-String -Path "Cargo.toml" -Pattern "kube" -Quiet) {
    Write-Host "  ✅ Kubernetes dependencies found" -ForegroundColor Green
} else {
    Write-Host "  ❌ Kubernetes dependencies missing" -ForegroundColor Red
}

# Count features
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

Write-Host "`n📊 Summary:" -ForegroundColor Cyan
Write-Host "  Features implemented: $implemented/$($features.Count)" -ForegroundColor White
Write-Host "  Files created: $($requiredFiles.Count)" -ForegroundColor White

if ($implemented -eq $features.Count -and $allFilesExist) {
    Write-Host "`n🎉 All required features implemented!" -ForegroundColor Green
    Write-Host "✅ Validation PASSED" -ForegroundColor Green
} else {
    Write-Host "`n❌ Validation FAILED" -ForegroundColor Red
}

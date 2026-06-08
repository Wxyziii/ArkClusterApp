<#
.SYNOPSIS
    Package the Windows node agent setup bundle.
    Run from the repo root or scripts/windows/.
    Output: dist/ArkClusterNodeSetup.zip
#>

param(
    [string]$AgentExeSource = "services\node-agent\target\release\ark-node-agent.exe",
    [string]$OutDir = "dist",
    [string]$OutZip = "ArkClusterNodeSetup.zip"
)

$ErrorActionPreference = "Stop"
$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..\..")
$AgentExe = Join-Path $RepoRoot $AgentExeSource

Write-Host "Packaging ARK Cluster Node Setup bundle..." -ForegroundColor Cyan

# Check agent binary exists
if (-not (Test-Path $AgentExe)) {
    Write-Host "Building agent binary first..." -ForegroundColor Yellow
    Push-Location (Join-Path $RepoRoot "services\node-agent")
    cargo build --release
    Pop-Location
}
if (-not (Test-Path $AgentExe)) {
    Write-Error "Agent binary not found: $AgentExe"
    exit 1
}

# Staging directory
$staging = Join-Path $env:TEMP "ArkClusterNodeSetup_staging"
Remove-Item $staging -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory $staging | Out-Null

# Copy files
Copy-Item $AgentExe (Join-Path $staging "ark-node-agent.exe")
Copy-Item (Join-Path $PSScriptRoot "setup.ps1") (Join-Path $staging "setup.ps1")
Copy-Item (Join-Path $PSScriptRoot "README.txt") (Join-Path $staging "README.txt")
Copy-Item (Join-Path $PSScriptRoot "node-agent-config.example.json") (Join-Path $staging "node-agent-config.example.json")

# Generate checksums
$checksums = @()
foreach ($file in Get-ChildItem $staging) {
    $hash = (Get-FileHash $file.FullName -Algorithm SHA256).Hash
    $checksums += "$hash  $($file.Name)"
}
Set-Content (Join-Path $staging "checksums.sha256") ($checksums -join "`n")

# Create output directory
$outDirFull = Join-Path $RepoRoot $OutDir
New-Item -ItemType Directory -Force $outDirFull | Out-Null
$outZipFull = Join-Path $outDirFull $OutZip

# Zip
Compress-Archive -Path "$staging\*" -DestinationPath $outZipFull -Force

# Cleanup
Remove-Item $staging -Recurse -Force

# Report
$size = [math]::Round((Get-Item $outZipFull).Length / 1024 / 1024, 1)
Write-Host ""
Write-Host "Bundle created: $outZipFull ($size MB)" -ForegroundColor Green
Write-Host "Contents:"
foreach ($c in $checksums) { Write-Host "  $c" }
Write-Host ""
Write-Host "Share dist\ArkClusterNodeSetup.zip with your friend." -ForegroundColor Cyan

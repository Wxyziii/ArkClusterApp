#Requires -RunAsAdministrator
<#
.SYNOPSIS
    ARK Cluster Node setup script.
    Run this on a Windows PC that will act as a travel node.
    You need: Tailscale installed + joined, and a pairing code from /node invite.
#>

param(
    [string]$ManagerUrl,
    [string]$PairingCode,
    [string]$NodeName,
    [string]$NodeId,
    [string]$ArkDir = "",
    [string]$DataDir = "C:\ProgramData\ArkClusterNode",
    [string]$ClusterSharePath = "Z:\ark-cluster-main",
    [string]$UbuntuTailscaleIp = "100.68.7.42",
    [string]$ShareName = "ark-cluster-main",
    [int]$GamePort = 7789,
    [int]$RawPort = 7790,
    [int]$QueryPort = 27018,
    [int]$RconPort = 27023
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$AgentExe = Join-Path $PSScriptRoot "ark-node-agent.exe"
$ConfigFile = Join-Path $DataDir "config.json"
$ServiceName = "ArkClusterNodeAgent"

function Write-Step { param([string]$Msg) Write-Host "`n==> $Msg" -ForegroundColor Cyan }
function Write-OK   { param([string]$Msg) Write-Host "    [OK] $Msg" -ForegroundColor Green }
function Write-Warn { param([string]$Msg) Write-Host "    [WARN] $Msg" -ForegroundColor Yellow }
function Write-Fail { param([string]$Msg) Write-Host "`n[FAIL] $Msg" -ForegroundColor Red; exit 1 }

# ── Interactive prompts if not provided ──────────────────────────────────────

if (-not $ManagerUrl) {
    $ManagerUrl = Read-Host "Manager URL (e.g. http://100.68.7.42:8788)"
}
if (-not $PairingCode) {
    $PairingCode = Read-Host "Pairing code (e.g. ABCD-1234, from /node invite in Discord)"
}
if (-not $NodeName) {
    $NodeName = Read-Host "Node display name (e.g. Marcel PC)"
}
if (-not $NodeId) {
    $NodeId = ($NodeName.ToLower() -replace '[^a-z0-9]', '-').Trim('-')
    $NodeId = Read-Host "Node ID (default: $NodeId, press Enter to accept)"
    if (-not $NodeId) { $NodeId = ($NodeName.ToLower() -replace '[^a-z0-9]', '-').Trim('-') }
}

$ManagerUrl = $ManagerUrl.TrimEnd('/')

# ── Auto-detect ArkDir ────────────────────────────────────────────────────────

if (-not $ArkDir) {
    $candidates = @(
        "D:\ARK-Dedicated",
        "C:\ARK-Dedicated",
        "C:\Program Files (x86)\Steam\steamapps\common\ARK",
        "D:\Steam\steamapps\common\ARK",
        "E:\Steam\steamapps\common\ARK",
        "D:\SteamLibrary\steamapps\common\ARK",
        "E:\SteamLibrary\steamapps\common\ARK"
    )
    foreach ($c in $candidates) {
        if (Test-Path (Join-Path $c "ShooterGame\Binaries\Win64\ShooterGameServer.exe")) {
            $ArkDir = $c
            Write-Host "    Auto-detected ARK at: $ArkDir" -ForegroundColor Cyan
            break
        }
    }
    if (-not $ArkDir) { $ArkDir = "D:\ARK-Dedicated" }
}

# ── Step 1: Check Tailscale ───────────────────────────────────────────────────

Write-Step "Checking Tailscale"
$ts = Get-Command "tailscale" -ErrorAction SilentlyContinue
if (-not $ts) {
    Write-Fail "Tailscale not found. Install from https://tailscale.com/download and join your tailnet first."
}
$tsStatus = & tailscale status --json 2>&1
try {
    $tsJson = $tsStatus | ConvertFrom-Json
    if ($tsJson.BackendState -ne "Running") {
        Write-Fail "Tailscale is not running (BackendState=$($tsJson.BackendState)). Run 'tailscale up' first."
    }
    Write-OK "Tailscale running, IP: $($tsJson.TailscaleIPs[0])"
} catch {
    Write-Warn "Could not parse Tailscale status. Continuing anyway."
}

# ── Step 2: Verify manager reachable ─────────────────────────────────────────

Write-Step "Checking manager reachable"
try {
    $health = Invoke-RestMethod -Uri "$ManagerUrl/health" -TimeoutSec 5
    Write-OK "Manager reachable: status=$($health.status)"
} catch {
    Write-Fail "Cannot reach manager at $ManagerUrl/health. Check Tailscale and manager URL."
}

# ── Step 3: Check/install SteamCMD ───────────────────────────────────────────

Write-Step "Checking SteamCMD"
$SteamCmdDir = "C:\steamcmd"
$SteamCmd = Join-Path $SteamCmdDir "steamcmd.exe"
if (-not (Test-Path $SteamCmd)) {
    Write-Host "    Downloading SteamCMD..."
    New-Item -ItemType Directory -Force $SteamCmdDir | Out-Null
    $steamZip = Join-Path $env:TEMP "steamcmd.zip"
    Invoke-WebRequest -Uri "https://steamcdn-a.akamaihd.net/client/installer/steamcmd.zip" -OutFile $steamZip
    Expand-Archive -Path $steamZip -DestinationPath $SteamCmdDir -Force
    Remove-Item $steamZip
    Write-OK "SteamCMD downloaded to $SteamCmdDir"
} else {
    Write-OK "SteamCMD found at $SteamCmd"
}

# ── Step 4: Install/update ARK Dedicated Server ───────────────────────────────

Write-Step "Checking ARK Dedicated Server"
$arkExe = Join-Path $ArkDir "ShooterGame\Binaries\Win64\ShooterGameServer.exe"
if (Test-Path $arkExe) {
    Write-OK "ARK already installed at $ArkDir — skipping download"
} else {
    Write-Host "    ShooterGameServer.exe not found at $ArkDir"
    Write-Host "    Downloading via SteamCMD (App 376030). This may take 30-60 min..." -ForegroundColor Yellow
    New-Item -ItemType Directory -Force $ArkDir | Out-Null
    & $SteamCmd "+force_install_dir" $ArkDir "+login" "anonymous" "+app_update" "376030" "validate" "+quit"
    if (-not (Test-Path $arkExe)) {
        Write-Host ""
        Write-Host "    If ARK is already installed elsewhere, re-run with:" -ForegroundColor Yellow
        Write-Host "    .\setup.ps1 -ArkDir 'C:\Program Files (x86)\Steam\steamapps\common\ARK'" -ForegroundColor Yellow
        Write-Fail "ARK install failed. ShooterGameServer.exe not found at: $arkExe"
    }
    Write-OK "ARK Dedicated Server installed at $ArkDir"
}

# ── Step 5: Create working directories ───────────────────────────────────────

Write-Step "Creating node directories"
foreach ($dir in @($DataDir, "$DataDir\logs", "$DataDir\backups", "$DataDir\config")) {
    New-Item -ItemType Directory -Force $dir | Out-Null
}
Write-OK "Directories created under $DataDir"

# ── Step 6: Mount cluster share ───────────────────────────────────────────────

Write-Step "Mounting cluster share $ClusterSharePath"
$shareLetter = ($ClusterSharePath -split '\\')[0].TrimEnd(':')
$shareUncPath = "\\$UbuntuTailscaleIp\$ShareName"
$existing = & net use 2>&1 | Select-String "${shareLetter}:"
if ($existing) {
    Write-OK "Share already mounted at ${shareLetter}:"
} else {
    Write-Host "    Mounting $shareUncPath -> ${shareLetter}:"
    Write-Host ""
    Write-Host "    NOTE: You need SMB credentials for the share." -ForegroundColor Yellow
    Write-Host "    The server admin must have run setup_smb_share.sh first." -ForegroundColor Yellow
    $cred = Get-Credential -Message "SMB credentials for $shareUncPath (use the smb user from setup_smb_share.sh)"
    $netResult = & net use "${shareLetter}:" $shareUncPath /user:$($cred.UserName) $cred.GetNetworkCredential().Password /persistent:yes 2>&1
    if ($LASTEXITCODE -ne 0) {
        Write-Fail "Failed to mount share: $netResult`nEnsure Ubuntu has SMB set up (run setup_smb_share.sh on Ubuntu)."
    }
    Write-OK "Cluster share mounted at ${shareLetter}:"
}

# ── Step 7: Verify share is writable ─────────────────────────────────────────

Write-Step "Verifying cluster share writable"
$testFile = Join-Path $ClusterSharePath ".ark_node_write_test"
try {
    Set-Content -Path $testFile -Value "write_test" -ErrorAction Stop
    Remove-Item $testFile -ErrorAction SilentlyContinue
    Write-OK "Cluster share is writable"
} catch {
    Write-Fail "Cannot write to cluster share $ClusterSharePath. Check SMB permissions."
}

# ── Step 8: Complete pairing with manager ────────────────────────────────────

Write-Step "Pairing with manager"
$pairBody = @{
    code             = $PairingCode
    nodeId           = $NodeId
    nodeName         = $NodeName
    nodeType         = "external-windows"
} | ConvertTo-Json

try {
    $pairResult = Invoke-RestMethod -Method POST -Uri "$ManagerUrl/api/nodes/pair/complete" `
        -ContentType "application/json" -Body $pairBody -TimeoutSec 15
} catch {
    $errMsg = $_.ErrorDetails.Message
    try { $errJson = $errMsg | ConvertFrom-Json; $errMsg = $errJson.reason ?? $errJson.error.message ?? $errMsg } catch {}
    Write-Fail "Pairing failed: $errMsg`nCheck the pairing code (it may have expired). Use /node invite in Discord."
}

if (-not $pairResult.nodeToken) {
    Write-Fail "Pairing response missing nodeToken: $($pairResult | ConvertTo-Json)"
}

$nodeToken = $pairResult.nodeToken
Write-OK "Paired! Node ID: $($pairResult.nodeId)"

# ── Step 9: Write config ──────────────────────────────────────────────────────

Write-Step "Writing node config"
$config = @{
    nodeId               = $NodeId
    nodeName             = $NodeName
    ownerDiscordUserId   = ""
    managerUrl           = $ManagerUrl
    nodeToken            = $nodeToken
    clusterSharePath     = $ClusterSharePath
    arkDedicatedDir      = $ArkDir
    modIds               = @(731604991,761535755,1609138312,1522327484,1445395055,1428596566,751991809,2080911395,2007481691,1551907938,1404697612,1591643730,1837445660,569786012,693416678,924933745,2278341478)
    gamePort             = $GamePort
    rawPort              = $RawPort
    queryPort            = $QueryPort
    rconPort             = $RconPort
    serverAdminPassword  = ""
    logDir               = "$DataDir\logs"
    backupDir            = "$DataDir\backups"
    heartbeatIntervalSecs = 30
    taskPollIntervalSecs  = 5
    maxRamMbBeforeBlock   = 4096
} | ConvertTo-Json -Depth 5

Set-Content -Path $ConfigFile -Value $config -Encoding UTF8
Write-OK "Config written to $ConfigFile"
Write-Host "    IMPORTANT: Edit $ConfigFile to set 'serverAdminPassword' before starting!" -ForegroundColor Yellow

# ── Step 10: Copy agent exe ───────────────────────────────────────────────────

Write-Step "Installing agent binary"
if (-not (Test-Path $AgentExe)) {
    Write-Fail "ark-node-agent.exe not found at $AgentExe. Ensure it's in the same folder as setup.ps1."
}
$agentDest = Join-Path $DataDir "ark-node-agent.exe"
Copy-Item $AgentExe $agentDest -Force
Write-OK "Agent copied to $agentDest"

# ── Step 11: Windows Firewall rules ──────────────────────────────────────────

Write-Step "Adding Windows Firewall rules (private + Tailscale only)"
$fwRules = @(
    @{ Name = "ARK-Node-UDP-Game";  Port = $GamePort;  Protocol = "UDP" },
    @{ Name = "ARK-Node-UDP-Raw";   Port = $RawPort;   Protocol = "UDP" },
    @{ Name = "ARK-Node-UDP-Query"; Port = $QueryPort; Protocol = "UDP" },
    @{ Name = "ARK-Node-TCP-RCON";  Port = $RconPort;  Protocol = "TCP" }
)
foreach ($rule in $fwRules) {
    $existing = Get-NetFirewallRule -DisplayName $rule.Name -ErrorAction SilentlyContinue
    if ($existing) {
        Write-OK "Firewall rule already exists: $($rule.Name)"
    } else {
        New-NetFirewallRule -DisplayName $rule.Name -Direction Inbound `
            -Action Allow -Protocol $rule.Protocol -LocalPort $rule.Port `
            -Profile Private | Out-Null
        Write-OK "Created firewall rule: $($rule.Name) $($rule.Protocol)/$($rule.Port)"
    }
}

# ── Step 12: Install as Windows Service ──────────────────────────────────────

Write-Step "Installing Windows service: $ServiceName"
$existingSvc = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
if ($existingSvc) {
    Write-Host "    Service exists, stopping and updating..."
    Stop-Service $ServiceName -Force -ErrorAction SilentlyContinue
    & sc.exe delete $ServiceName | Out-Null
    Start-Sleep 2
}

$svcBinPath = "`"$agentDest`" `"$ConfigFile`""
& sc.exe create $ServiceName binPath= $svcBinPath start= auto obj= "LocalSystem" DisplayName= "ARK Cluster Node Agent" | Out-Null
& sc.exe description $ServiceName "Connects to ARK Cluster Manager and manages local travel ARK server" | Out-Null

Write-Host "    Starting service..."
Start-Service $ServiceName
$svc = Get-Service $ServiceName
if ($svc.Status -ne "Running") {
    Write-Fail "Service failed to start. Check Event Viewer > Windows Logs > Application for errors."
}
Write-OK "Service $ServiceName is running"

# ── Done ──────────────────────────────────────────────────────────────────────

Write-Host ""
Write-Host "==========================================================" -ForegroundColor Green
Write-Host " ARK Cluster Node Setup Complete!" -ForegroundColor Green
Write-Host "==========================================================" -ForegroundColor Green
Write-Host ""
Write-Host " Node ID:   $NodeId"
Write-Host " Node name: $NodeName"
Write-Host " Manager:   $ManagerUrl"
Write-Host " Config:    $ConfigFile"
Write-Host ""
Write-Host " IMPORTANT NEXT STEPS:" -ForegroundColor Yellow
Write-Host "   1. Edit $ConfigFile — set 'serverAdminPassword'"
Write-Host "   2. Restart the service: Restart-Service $ServiceName"
Write-Host "   3. Wait ~30 seconds for first heartbeat"
Write-Host "   4. In Discord: /node status — node should appear online"
Write-Host "   5. Test: /travel valguero (or any map)"
Write-Host ""
Write-Host " Logs: Get-EventLog -LogName Application -Source $ServiceName -Newest 20"
Write-Host ""

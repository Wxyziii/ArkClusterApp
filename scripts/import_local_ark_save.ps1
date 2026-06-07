#Requires -Version 5
<#
.SYNOPSIS
    Stage local Windows ARK singleplayer saves for server import.

.DESCRIPTION
    Copies save files and config from the local Windows ARK installation to a
    staging directory. Never moves or deletes local files. Optionally uploads
    the staged files to the server via scp.

    Does NOT copy character profile (LocalPlayer.arkprofile) — handle character
    transfer separately via in-game ARK upload/download.

.PARAMETER Map
    Which map to export: TheIsland (default) or Fjordur.

.PARAMETER StagingDir
    Where to write staged files (default: .\import-staging\<timestamp>).

.PARAMETER Upload
    If set, upload staged files to the server via scp.

.PARAMETER ServerHost
    SSH/SCP target (default: marcel@100.68.7.42).

.PARAMETER SshKey
    Path to SSH private key (default: $HOME\.ssh\homeops_ed25519).

.EXAMPLE
    .\import_local_ark_save.ps1 -Map TheIsland
    .\import_local_ark_save.ps1 -Map Fjordur -Upload
#>

param(
    [ValidateSet("TheIsland", "Fjordur")]
    [string]$Map = "TheIsland",

    [string]$StagingDir = "",

    [switch]$Upload,

    [string]$ServerHost = "marcel@100.68.7.42",

    [string]$SshKey = "$HOME\.ssh\homeops_ed25519"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

# ── Constants ────────────────────────────────────────────────────────────────
$ArkBase = "C:\Program Files (x86)\Steam\steamapps\common\ARK\ShooterGame\Saved"
$SaveDirMap = @{
    TheIsland = "$ArkBase\SavedArksLocal"
    Fjordur   = "$ArkBase\FjordurSavedArksLocal"
}
$ArkMapFile = @{
    TheIsland = "TheIsland.ark"
    Fjordur   = "Fjordur.ark"
}
$ServerSlotSaveDir = "/srv/ark/server/ShooterGame/Saved/home"

# ── Check ARK not running ────────────────────────────────────────────────────
$arkProc = Get-Process -Name "ShooterGame" -ErrorAction SilentlyContinue
if ($arkProc) {
    Write-Error "ARK is running (process: ShooterGame PID $($arkProc.Id)). Close ARK before importing. Re-run with -Force to skip this check."
    exit 1
}

# ── Resolve paths ────────────────────────────────────────────────────────────
$SaveDir = $SaveDirMap[$Map]
$ArkFile = "$SaveDir\$($ArkMapFile[$Map])"

if (-not (Test-Path $ArkFile)) {
    Write-Error "Save file not found: $ArkFile"
    exit 1
}

$ConfigDir = "$ArkBase\Config\WindowsNoEditor"
$GameIni = "$ConfigDir\Game.ini"
$GusIni  = "$ConfigDir\GameUserSettings.ini"

# ── Staging directory ────────────────────────────────────────────────────────
if ($StagingDir -eq "") {
    $ts = Get-Date -Format "yyyyMMdd_HHmmss"
    $StagingDir = ".\import-staging\${Map}_${ts}"
}
$StagingDir = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($StagingDir)
New-Item -ItemType Directory -Path $StagingDir -Force | Out-Null
New-Item -ItemType Directory -Path "$StagingDir\saves" -Force | Out-Null
New-Item -ItemType Directory -Path "$StagingDir\config" -Force | Out-Null

Write-Host "Staging directory: $StagingDir"

# ── Copy world save ──────────────────────────────────────────────────────────
$srcArk = $ArkFile
$dstArk = "$StagingDir\saves\$($ArkMapFile[$Map])"
Copy-Item -Path $srcArk -Destination $dstArk
$sizeMb = [math]::Round((Get-Item $dstArk).Length / 1MB, 1)
Write-Host "World save: $($ArkMapFile[$Map]) ($sizeMb MB)"

# ── Copy tribe files (skip character profiles) ───────────────────────────────
$tribeFiles = Get-ChildItem -Path $SaveDir -File | Where-Object {
    $_.Extension -in ".arktribe", ".tribebak"
}
foreach ($f in $tribeFiles) {
    Copy-Item -Path $f.FullName -Destination "$StagingDir\saves\$($f.Name)"
    Write-Host "Tribe file: $($f.Name)"
}
Write-Host "(Skipped LocalPlayer.arkprofile — handle character transfer in-game)"

# ── Copy config ───────────────────────────────────────────────────────────────
foreach ($src in @($GameIni, $GusIni)) {
    if (Test-Path $src) {
        $name = Split-Path $src -Leaf
        Copy-Item -Path $src -Destination "$StagingDir\config\$name"
        Write-Host "Config: $name"
    }
}

# ── Write manifest ────────────────────────────────────────────────────────────
$manifest = @{
    map          = $Map
    arkFile      = $ArkMapFile[$Map]
    exportedAt   = (Get-Date -Format "o")
    windowsUser  = $env:USERNAME
    sourceDir    = $SaveDir
    serverTarget = $ServerSlotSaveDir
    files        = @((Get-ChildItem -Recurse -File $StagingDir | Select-Object -ExpandProperty Name))
}
$manifest | ConvertTo-Json -Depth 5 | Set-Content "$StagingDir\manifest.json"
Write-Host ""
Write-Host "Staged $($manifest.files.Count) files to $StagingDir"

# ── Optional upload ───────────────────────────────────────────────────────────
if ($Upload) {
    if (-not (Test-Path $SshKey)) {
        Write-Error "SSH key not found: $SshKey"
        exit 1
    }
    Write-Host ""
    Write-Host "Uploading to $ServerHost ..."

    $remoteStaging = "/tmp/ark-import-$(Get-Date -Format 'yyyyMMdd_HHmmss')"
    # Create remote staging dir
    & ssh -i $SshKey -o StrictHostKeyChecking=no $ServerHost "mkdir -p $remoteStaging"

    # Upload saves
    & scp -i $SshKey -r "$StagingDir\saves" "${ServerHost}:$remoteStaging/"
    & scp -i $SshKey -r "$StagingDir\config" "${ServerHost}:$remoteStaging/"
    & scp -i $SshKey "$StagingDir\manifest.json" "${ServerHost}:$remoteStaging/"

    Write-Host ""
    Write-Host "Upload complete. Remote staging: $remoteStaging"
    Write-Host ""
    Write-Host "On the server, run:"
    Write-Host "  sudo /opt/ark-cluster-app/scripts/import_ark_save_server.sh $remoteStaging"
}
else {
    Write-Host ""
    Write-Host "Next steps:"
    Write-Host "  1. Upload $StagingDir to the server:"
    Write-Host "     scp -i $SshKey -r $StagingDir ${ServerHost}:/tmp/ark-import"
    Write-Host "  2. On server, run:"
    Write-Host "     sudo /opt/ark-cluster-app/scripts/import_ark_save_server.sh /tmp/ark-import"
}

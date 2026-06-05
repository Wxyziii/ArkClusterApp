#!/usr/bin/env bash
set -euo pipefail

ARK_ROOT="${ARK_ROOT:-/srv/ark}"
ARK_USER="${ARK_USER:-ark}"
ARK_SERVER_DIR="$ARK_ROOT/server"
ARK_EXE="$ARK_SERVER_DIR/ShooterGame/Binaries/Linux/ShooterGameServer"

if ! command -v steamcmd >/dev/null 2>&1; then
  echo "SteamCMD missing. Run scripts/server/install_steamcmd.sh first." >&2
  exit 1
fi

sudo install -o "$ARK_USER" -g "$ARK_USER" -m 0750 -d \
  "$ARK_ROOT" \
  "$ARK_SERVER_DIR" \
  "$ARK_ROOT/clusters/main" \
  "$ARK_ROOT/backups" \
  "$ARK_ROOT/logs"

sudo -u "$ARK_USER" -H steamcmd \
  +@sSteamCmdForcePlatformType linux \
  +force_install_dir "$ARK_SERVER_DIR" \
  +login anonymous \
  +app_update "376030 -beta preaquatica" validate \
  +quit

if [[ ! -x "$ARK_EXE" ]]; then
  sudo chmod +x "$ARK_EXE" 2>/dev/null || true
fi

test -x "$ARK_EXE"
echo "ARK server ready: $ARK_EXE"

#!/usr/bin/env bash
set -euo pipefail

ARK_ROOT="${ARK_ROOT:-/srv/ark}"
ARK_USER="${ARK_USER:-ark}"
CONFIG_DIR="$ARK_ROOT/server/ShooterGame/Saved/Config/LinuxServer"
SAVED_ARCSS="$ARK_ROOT/server/ShooterGame/Saved/SavedArks"

sudo install -o "$ARK_USER" -g "$ARK_USER" -m 0750 -d \
  "$ARK_ROOT/slots/home" "$ARK_ROOT/slots/travel-a" "$ARK_ROOT/slots/travel-b" \
  "$ARK_ROOT/clusters/main" "$ARK_ROOT/profiles/maps" "$ARK_ROOT/profiles/slots" \
  "$ARK_ROOT/backups" "$ARK_ROOT/logs" "$CONFIG_DIR" \
  "$SAVED_ARCSS/home" "$SAVED_ARCSS/travel-a" "$SAVED_ARCSS/travel-b"

if [[ -f "$CONFIG_DIR/GameUserSettings.ini" ]]; then
  sudo cp -n "$CONFIG_DIR/GameUserSettings.ini" "$CONFIG_DIR/GameUserSettings.ini.pre-manager" || true
else
  sudo tee "$CONFIG_DIR/GameUserSettings.ini" >/dev/null <<'INI'
[ServerSettings]
ServerPassword=
ServerAdminPassword=
RCONEnabled=True
RCONPort=27020
RCONServerGameLogBuffer=600
ActiveMods=
AllowThirdPersonPlayer=True
ShowMapPlayerLocation=True
ServerCrosshair=True
DifficultyOffset=1.000000
OverrideOfficialDifficulty=5.000000
PreventDownloadSurvivors=False
PreventDownloadItems=False
PreventDownloadDinos=False
PreventUploadSurvivors=False
PreventUploadItems=False
PreventUploadDinos=False
NoTributeDownloads=False
INI
fi

if [[ -f "$CONFIG_DIR/Game.ini" ]]; then
  sudo cp -n "$CONFIG_DIR/Game.ini" "$CONFIG_DIR/Game.ini.pre-manager" || true
else
  sudo tee "$CONFIG_DIR/Game.ini" >/dev/null <<'INI'
[/script/shootergame.shootergamemode]
INI
fi

sudo chown -R "$ARK_USER:$ARK_USER" "$ARK_ROOT"
sudo chmod 0640 "$CONFIG_DIR/GameUserSettings.ini" "$CONFIG_DIR/Game.ini"

echo "ARK runtime layout ready under $ARK_ROOT"

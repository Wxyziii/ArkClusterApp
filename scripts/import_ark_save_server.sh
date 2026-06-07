#!/usr/bin/env bash
# import_ark_save_server.sh — Import a staged ARK save into the home slot.
#
# Usage: sudo ./import_ark_save_server.sh <staging-dir>
#
# The staging directory must contain:
#   saves/          — .ark and .arktribe files
#   config/         — Game.ini and/or GameUserSettings.ini
#   manifest.json   — written by import_local_ark_save.ps1
#
# What this script does:
#   1. Verify home server is stopped (refuse if running)
#   2. Back up current saves + config to timestamped archive
#   3. Import .ark and .arktribe files
#   4. Merge config (preserve server-critical values, import gameplay values)
#   5. Fix ownership/permissions
#   6. Print next steps (manual start required)
#
# What this script does NOT do:
#   - Start or stop the server automatically
#   - Import character profiles (user handles this in-game)
#   - Modify systemd configuration
#   - Print secrets

set -euo pipefail

# ──────────────────────────────────────────────────────────────────────────────
# Merge functions (must be defined before main logic)
# ──────────────────────────────────────────────────────────────────────────────

merge_game_ini() {
    local src="$1"
    local dst="$2"
    echo "  merging Game.ini ..."

    python3 << PYEOF
import sys

src_path = "$src"
dst_path = "$dst"
skip_keys = {"busesingleplayersettings", "bdisablegenesismissions"}

def parse_ini(path):
    sections = {}
    current = None
    try:
        with open(path, encoding='utf-8', errors='replace') as f:
            for line in f:
                stripped = line.rstrip('\n')
                m_sec = stripped.strip()
                if m_sec.startswith('[') and m_sec.endswith(']'):
                    current = m_sec
                    sections.setdefault(current, [])
                elif current is not None:
                    sections[current].append(stripped)
    except FileNotFoundError:
        pass
    return sections

src_sec = parse_ini(src_path)
dst_sec = parse_ini(dst_path)

imported = 0
for section, lines in src_sec.items():
    dst_sec.setdefault(section, [])
    existing = set()
    for l in dst_sec[section]:
        if '=' in l.strip():
            existing.add(l.strip().split('=', 1)[0].strip().lower())
    for l in lines:
        stripped = l.strip()
        if not stripped or stripped.startswith(';'):
            continue
        if '=' in stripped:
            key = stripped.split('=', 1)[0].strip().lower()
            if key in skip_keys:
                continue
            if key in existing:
                continue
            dst_sec[section].append(l)
            existing.add(key)
            imported += 1

with open(dst_path, 'w', encoding='utf-8') as f:
    for section, lines in dst_sec.items():
        f.write(section + '\n')
        for l in lines:
            f.write(l + '\n')

print(f"  Game.ini: imported {imported} new keys")
PYEOF
}

merge_gus() {
    local src="$1"
    local dst="$2"
    echo "  merging GameUserSettings.ini ..."

    python3 << PYEOF
import sys

src_path  = "$src"
dst_path  = "$dst"

# Server-critical: never overwrite from Windows source
preserve = {
    "rconport", "rconenabled", "rconservergamelogbuffer",
    "serveradminpassword", "serverpassword", "serverpveenabled",
    "activemods", "port", "queryport", "sessionname",
    "clusterdiroverride", "preventofflinepvp", "battleye",
    "autosaveperiodminutes",
}
# Singleplayer-only: skip entirely
skip = {"bislocalmode", "playertamedinocount", "localport"}

def parse_ordered(path):
    """Returns list of (section_header, [lines])."""
    result, cur_sec, cur_lines = [], None, []
    try:
        with open(path, encoding='utf-8', errors='replace') as f:
            for raw in f:
                line = raw.rstrip('\n')
                s = line.strip()
                if s.startswith('[') and s.endswith(']'):
                    if cur_sec is not None:
                        result.append((cur_sec, cur_lines))
                    cur_sec, cur_lines = s, []
                elif cur_sec is not None:
                    cur_lines.append(line)
    except FileNotFoundError:
        return []
    if cur_sec is not None:
        result.append((cur_sec, cur_lines))
    return result

def kv_map(lines):
    m = {}
    for l in lines:
        s = l.strip()
        if '=' in s:
            k = s.split('=', 1)[0].strip().lower()
            m[k] = l
    return m

src_ordered = parse_ordered(src_path)
dst_ordered = parse_ordered(dst_path)

if not dst_ordered:
    # server GUS missing; start from source, dropping skip keys
    new_ordered = []
    for sec, lines in src_ordered:
        kept = [l for l in lines
                if '=' not in l.strip()
                or l.strip().split('=',1)[0].strip().lower() not in skip]
        new_ordered.append((sec, kept))
    dst_ordered = new_ordered
else:
    src_map = dict(src_ordered)
    new_dst = []
    seen_secs = set()

    for sec, dst_lines in dst_ordered:
        seen_secs.add(sec)
        src_lines = src_map.get(sec, [])
        src_kv = kv_map(src_lines)
        # Update existing dst lines with src values (unless preserve/skip)
        updated = []
        for l in dst_lines:
            s = l.strip()
            if '=' in s:
                k = s.split('=', 1)[0].strip().lower()
                if k not in preserve and k not in skip and k in src_kv:
                    updated.append(src_kv[k])
                    continue
            updated.append(l)
        # Add keys present in src but not in dst (not preserve/skip)
        dst_kv = kv_map(dst_lines)
        for k, l in src_kv.items():
            if k in dst_kv or k in preserve or k in skip:
                continue
            updated.append(l)
        new_dst.append((sec, updated))

    # Add sections from src not in dst
    for sec, lines in src_ordered:
        if sec in seen_secs:
            continue
        kept = [l for l in lines
                if '=' not in l.strip()
                or l.strip().split('=',1)[0].strip().lower() not in preserve | skip]
        new_dst.append((sec, kept))

    dst_ordered = new_dst

with open(dst_path, 'w', encoding='utf-8') as f:
    for sec, lines in dst_ordered:
        f.write(sec + '\n')
        for l in lines:
            f.write(l + '\n')

print("  GameUserSettings.ini: merged (server-critical values preserved)")
PYEOF
}

# ──────────────────────────────────────────────────────────────────────────────
# Main
# ──────────────────────────────────────────────────────────────────────────────

STAGING="${1:-}"
SAVE_DIR="/srv/ark/server/ShooterGame/Saved/home"
CONFIG_DIR="/srv/ark/server/ShooterGame/Saved/Config/LinuxServer"
BACKUP_ROOT="/srv/ark/backups/import-backup"
ARK_USER="ark"
ARK_GROUP="ark"

# ── Validation ────────────────────────────────────────────────────────────────
if [[ -z "$STAGING" ]]; then
    echo "Usage: sudo $0 <staging-dir>" >&2
    exit 1
fi
if [[ ! -d "$STAGING" ]]; then
    echo "Staging dir not found: $STAGING" >&2
    exit 1
fi
if [[ "$EUID" -ne 0 ]]; then
    echo "Must run as root (use sudo)" >&2
    exit 1
fi

SAVES_SRC="$STAGING/saves"
CONFIG_SRC="$STAGING/config"

if [[ ! -d "$SAVES_SRC" ]]; then
    echo "No saves/ directory in staging: $STAGING" >&2
    exit 1
fi

# ── Check home server is stopped ─────────────────────────────────────────────
echo "Checking home server state..."
HOME_STATE=$(systemctl is-active ark-server@home.service 2>/dev/null || echo "inactive")
if [[ "$HOME_STATE" == "active" ]]; then
    echo "ERROR: ark-server@home.service is running. Stop it first:" >&2
    echo "  sudo systemctl stop ark-server@home.service" >&2
    exit 1
fi
echo "  home server: $HOME_STATE (safe to import)"

# ── Backup current state ──────────────────────────────────────────────────────
TS=$(date +%Y%m%d_%H%M%S)
BACKUP_DIR="$BACKUP_ROOT/$TS"
mkdir -p "$BACKUP_DIR/saves" "$BACKUP_DIR/config"

echo "Backing up current saves and config to $BACKUP_DIR ..."
find "$SAVE_DIR" -maxdepth 1 -type f \( -name "*.ark" -o -name "*.arktribe" -o -name "*.tribebak" \) \
    -exec cp -p {} "$BACKUP_DIR/saves/" \; 2>/dev/null || true
cp -p "$CONFIG_DIR/Game.ini" "$BACKUP_DIR/config/" 2>/dev/null || true
cp -p "$CONFIG_DIR/GameUserSettings.ini" "$BACKUP_DIR/config/" 2>/dev/null || true
echo "  backup done: $BACKUP_DIR"

# ── Import world save (.ark) ──────────────────────────────────────────────────
echo ""
echo "Importing save files..."
ARK_FILE=$(find "$SAVES_SRC" -maxdepth 1 -name "*.ark" | sort | tail -1)
if [[ -z "$ARK_FILE" ]]; then
    echo "ERROR: no .ark file found in $SAVES_SRC" >&2
    exit 1
fi
ARK_NAME=$(basename "$ARK_FILE")
cp -p "$ARK_FILE" "$SAVE_DIR/$ARK_NAME"
SIZE=$(du -sh "$SAVE_DIR/$ARK_NAME" | cut -f1)
echo "  world save: $ARK_NAME ($SIZE)"

# ── Import tribe files ────────────────────────────────────────────────────────
TRIBE_COUNT=0
for f in "$SAVES_SRC"/*.arktribe "$SAVES_SRC"/*.tribebak; do
    [[ -f "$f" ]] || continue
    cp -p "$f" "$SAVE_DIR/$(basename "$f")"
    echo "  tribe: $(basename "$f")"
    TRIBE_COUNT=$((TRIBE_COUNT + 1))
done
echo "  $TRIBE_COUNT tribe file(s) imported"
echo "  (character profiles skipped — transfer in-game via ARK upload/download)"

# ── Fix save permissions ──────────────────────────────────────────────────────
chown -R "$ARK_USER:$ARK_GROUP" "$SAVE_DIR"
find "$SAVE_DIR" -type f -exec chmod 640 {} \;
find "$SAVE_DIR" -type d -exec chmod 750 {} \;
echo "  save permissions fixed"

# ── Merge config ──────────────────────────────────────────────────────────────
echo ""
echo "Merging config ..."
if [[ -f "$CONFIG_SRC/Game.ini" ]]; then
    merge_game_ini "$CONFIG_SRC/Game.ini" "$CONFIG_DIR/Game.ini"
    chown "$ARK_USER:$ARK_GROUP" "$CONFIG_DIR/Game.ini"
    chmod 640 "$CONFIG_DIR/Game.ini"
else
    echo "  Game.ini: not in staging, skipped"
fi

if [[ -f "$CONFIG_SRC/GameUserSettings.ini" ]]; then
    merge_gus "$CONFIG_SRC/GameUserSettings.ini" "$CONFIG_DIR/GameUserSettings.ini"
    chown "$ARK_USER:$ARK_GROUP" "$CONFIG_DIR/GameUserSettings.ini"
    chmod 640 "$CONFIG_DIR/GameUserSettings.ini"
else
    echo "  GameUserSettings.ini: not in staging, skipped"
fi

# ──────────────────────────────────────────────────────────────────────────────
echo ""
echo "=== Import complete ==="
echo "Backup of previous state: $BACKUP_DIR"
echo ""
echo "Next steps:"
echo "  1. Start the home server:"
echo "     sudo systemctl start ark-server@home.service"
echo "  2. Transfer your character in-game via ARK upload/download"
echo "  3. Verify your structures and dinos are present"

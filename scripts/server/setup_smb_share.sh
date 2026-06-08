#!/usr/bin/env bash
# Setup SMB share for ARK cluster folder.
# Only accessible over Tailscale (bind to Tailscale IP only).
# Run on Ubuntu as root: sudo bash scripts/server/setup_smb_share.sh
set -euo pipefail

CLUSTER_DIR="/srv/ark/clusters/main"
SHARE_NAME="ark-cluster-main"
SMB_USER="arksmb"
TAILSCALE_IFACE="tailscale0"

# ── Helper ────────────────────────────────────────────────────────────────────

say()  { echo "==> $*"; }
ok()   { echo "    [OK] $*"; }
warn() { echo "    [WARN] $*" >&2; }
die()  { echo "    [FAIL] $*" >&2; exit 1; }

require_root() {
    [[ $EUID -eq 0 ]] || die "Run as root: sudo bash $0"
}

# ── Step 1: Check cluster directory ──────────────────────────────────────────

require_root
say "Checking cluster directory"
if [[ ! -d "$CLUSTER_DIR" ]]; then
    say "Creating $CLUSTER_DIR"
    mkdir -p "$CLUSTER_DIR"
    chown ark:ark "$CLUSTER_DIR" 2>/dev/null || true
    chmod 775 "$CLUSTER_DIR"
fi
ok "Cluster dir: $CLUSTER_DIR"

# ── Step 2: Get Tailscale IP ──────────────────────────────────────────────────

say "Getting Tailscale IP"
if ! command -v tailscale &>/dev/null; then
    die "Tailscale not installed. Install from https://tailscale.com/download"
fi
TAILSCALE_IP=$(tailscale ip -4 2>/dev/null | head -1)
if [[ -z "$TAILSCALE_IP" ]]; then
    die "Could not determine Tailscale IP. Is tailscale running? Run: tailscale up"
fi
ok "Tailscale IP: $TAILSCALE_IP"

# ── Step 3: Install Samba ─────────────────────────────────────────────────────

say "Installing Samba"
if ! command -v smbd &>/dev/null; then
    apt-get update -qq
    apt-get install -y samba samba-common-bin
fi
ok "Samba installed"

# ── Step 4: Create SMB user ───────────────────────────────────────────────────

say "Creating SMB user: $SMB_USER"
if ! id "$SMB_USER" &>/dev/null; then
    useradd -r -s /usr/sbin/nologin "$SMB_USER"
    ok "System user $SMB_USER created"
else
    ok "System user $SMB_USER already exists"
fi

# Add SMB user to ark group so it can write to ark-owned cluster dir
if getent group ark &>/dev/null; then
    usermod -aG ark "$SMB_USER"
    ok "$SMB_USER added to ark group"
else
    warn "ark group not found — SMB writes may fail if cluster dir is owned by ark:ark"
fi

# Set SMB password (prompted)
echo ""
echo "    Set Samba password for user '$SMB_USER'."
echo "    This password is needed when running setup.ps1 on Windows nodes."
echo "    Choose something secure and share it with each node operator."
echo ""
smbpasswd -a "$SMB_USER" || die "Failed to set Samba password"
ok "Samba password set for $SMB_USER"

# Grant read/write to cluster dir
chown -R ark:ark "$CLUSTER_DIR" 2>/dev/null || true
chmod 2775 "$CLUSTER_DIR"

# ── Step 5: Configure smb.conf ────────────────────────────────────────────────

say "Writing Samba config"
SMB_CONF="/etc/samba/smb.conf"
SMB_BACKUP="${SMB_CONF}.pre-ark-$(date +%Y%m%d%H%M%S)"

# Backup existing config
cp "$SMB_CONF" "$SMB_BACKUP"
ok "Backup: $SMB_BACKUP"

# Remove any existing [ark-cluster-main] section
python3 - "$SMB_CONF" "$SHARE_NAME" <<'PYEOF'
import sys, re
path, section = sys.argv[1], sys.argv[2]
with open(path) as f:
    content = f.read()
# Remove section if it exists
pattern = rf'\n?\[{re.escape(section)}\][^\[]*'
content = re.sub(pattern, '', content, flags=re.DOTALL)
with open(path, 'w') as f:
    f.write(content.rstrip() + '\n')
PYEOF

# Patch [global] to bind to Tailscale IP only (idempotent)
python3 - "$SMB_CONF" "$TAILSCALE_IP" <<'PYEOF'
import sys, re
path, ip = sys.argv[1], sys.argv[2]
with open(path) as f:
    content = f.read()
# Set interfaces in [global]
if 'interfaces =' not in content:
    content = re.sub(r'\[global\]', f'[global]\n   interfaces = lo {ip}\n   bind interfaces only = yes', content)
else:
    content = re.sub(r'interfaces\s*=.*', f'interfaces = lo {ip}', content)
if 'bind interfaces only' not in content:
    content = re.sub(r'interfaces\s*=.*', lambda m: m.group(0) + '\n   bind interfaces only = yes', content)
with open(path, 'w') as f:
    f.write(content)
PYEOF

# Append share section
cat >> "$SMB_CONF" <<EOF

[$SHARE_NAME]
   comment = ARK Cluster Transfer Folder
   path = $CLUSTER_DIR
   valid users = $SMB_USER
   read only = no
   browseable = no
   create mask = 0664
   directory mask = 0775
   force user = $SMB_USER
EOF

ok "smb.conf updated"

# ── Step 6: Validate and restart Samba ───────────────────────────────────────

say "Validating Samba config"
testparm -s "$SMB_CONF" 2>&1 | tail -5 || die "smb.conf validation failed. Check $SMB_CONF"

say "Restarting Samba"
systemctl restart smbd nmbd
systemctl enable smbd nmbd
ok "Samba running"

# ── Step 7: Configure UFW (if present) ────────────────────────────────────────

if command -v ufw &>/dev/null; then
    say "Adding UFW rules for SMB on Tailscale interface"
    ufw allow in on "$TAILSCALE_IFACE" to any port 445 proto tcp comment "SMB-ARK-Tailscale" 2>/dev/null || warn "UFW rule failed (check manually)"
    ufw allow in on "$TAILSCALE_IFACE" to any port 137 proto udp comment "SMB-ARK-Tailscale" 2>/dev/null || true
    ufw allow in on "$TAILSCALE_IFACE" to any port 138 proto udp comment "SMB-ARK-Tailscale" 2>/dev/null || true
    ufw reload 2>/dev/null || true
    ok "UFW rules added"
else
    warn "UFW not found. Ensure your firewall allows TCP/445 on the Tailscale interface only."
fi

# ── Done ──────────────────────────────────────────────────────────────────────

echo ""
echo "==========================================================="
echo " SMB Share Setup Complete"
echo "==========================================================="
echo ""
echo " Share name:    $SHARE_NAME"
echo " Local path:    $CLUSTER_DIR"
echo " Tailscale IP:  $TAILSCALE_IP"
echo " SMB user:      $SMB_USER"
echo ""
echo " Windows mount command:"
echo "   net use Z: \\\\${TAILSCALE_IP}\\${SHARE_NAME} /user:${SMB_USER} <password> /persistent:yes"
echo ""
echo " setup.ps1 will prompt for the SMB password during node setup."
echo ""
echo " Verify from Windows:"
echo "   Test-NetConnection ${TAILSCALE_IP} -Port 445"
echo "   net use Z: \\\\${TAILSCALE_IP}\\${SHARE_NAME} /user:${SMB_USER} <password>"
echo ""

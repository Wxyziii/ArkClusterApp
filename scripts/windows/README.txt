ARK Cluster Node Agent — Setup Instructions
===========================================

REQUIREMENTS
------------
- Windows 10/11, 16GB+ RAM recommended
- Tailscale installed and joined to the cluster tailnet
  Download: https://tailscale.com/download
- ~100GB free disk (for ARK Dedicated Server + mods)
- The ARK server admin must give you a pairing code (/node invite in Discord)

QUICK START
-----------
1. Install Tailscale: https://tailscale.com/download
   - Create a Tailscale account if needed
   - Ask the cluster admin for a tailnet join link or key
   - Join the tailnet: tailscale up

2. Get a pairing code from the cluster admin:
   - In Discord: /node invite <your-name>
   - They will see: "Pairing code: ABCD-1234 (expires in 15 min)"

3. Extract this zip to a folder (e.g. C:\ark-node-setup\)

4. Right-click PowerShell -> "Run as Administrator"

5. Navigate to the folder:
   cd C:\ark-node-setup

6. Allow PowerShell scripts (one-time):
   Set-ExecutionPolicy RemoteSigned -Scope Process

7. Run setup:
   .\setup.ps1

8. When prompted:
   - Manager URL: http://100.68.7.42:8788  (ask the admin)
   - Pairing code: ABCD-1234               (from Discord /node invite)
   - Node name: My Travel PC               (any name you like)
   - SMB credentials: ask the admin

9. After setup, IMPORTANT:
   Edit C:\ProgramData\ArkClusterNode\config.json
   Set "serverAdminPassword" to something secret (ask admin or pick your own)
   Then: Restart-Service ArkClusterNodeAgent

10. Verify in Discord:
    /node status  -> your node should show as "online" or "not_ready"
    /node details <your-node-id>  -> shows exact readiness checks

WHAT SETUP DOES AUTOMATICALLY
-------------------------------
- Checks Tailscale is running
- Verifies manager is reachable
- Downloads SteamCMD (if not installed)
- Installs ARK Dedicated Server (~30GB)
- Creates C:\ProgramData\ArkClusterNode\  (config, logs, backups)
- Mounts the cluster share (Z:\ark-cluster-main)
- Registers this PC with the cluster manager
- Gets a unique node token
- Opens Windows Firewall rules (private network only)
- Installs ark-node-agent.exe as a Windows service

PLAYING
-------
- Your friend (cluster admin) starts the cluster on Ubuntu
- You join the Home server normally (The Island)
- When you want to travel: type /travel valguero in Discord
- Wait for "ready" confirmation, then use ARK terminal transfer
- When done, return to Home, then: /travelnode close in Discord

TROUBLESHOOTING
---------------
- Node not appearing online?
  Check: Get-EventLog -LogName Application -Source ArkClusterNodeAgent -Newest 20

- Cluster share not mounting?
  Ensure the Ubuntu server has SMB set up (ask admin to run setup_smb_share.sh)
  Check Tailscale is connected: tailscale ping 100.68.7.42

- Mods not valid?
  Wait for ARK Dedicated Server to fully install mods.
  SteamCMD downloads mods automatically on first /travel request.
  Or restart the service and wait for next heartbeat.

- Ports in use?
  Default ports: UDP 7789/7790, UDP 27018, TCP 27023
  If another application uses these, edit config.json and change them.
  Tell the admin the new ports.

FILES
-----
ark-node-agent.exe          - The node agent binary
setup.ps1                   - This setup script
node-agent-config.example.json - Config template for reference
README.txt                  - This file

Config location after setup: C:\ProgramData\ArkClusterNode\config.json
Log location: C:\ProgramData\ArkClusterNode\logs\

SECURITY
--------
- The node token in config.json is YOUR node's credential. Keep it private.
- The token only allows heartbeat/task APIs, NOT admin access to the cluster.
- Only Tailscale IPs can reach the manager (no public internet exposure).
- The cluster share is only mounted from within the Tailscale network.

Questions? Ask the cluster admin or check Discord.

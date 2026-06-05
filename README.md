# ARK Smart Cluster Manager

Dark, survival-tech admin dashboard for a private ARK: Survival Evolved smart cluster.

**Two parts, clearly separated:**
- **Frontend** — SvelteKit + TypeScript UI (this repo root, `src/`).
- **Backend** — Rust manager service (`services/manager/`), guarded ARK runtime control-plane foundation.

> Private/Tailscale use only. The backend exposes guarded lifecycle, backup,
> runtime, travel, config, mod, and maintenance API contracts. Dangerous
> operations stay capability-gated in `manager.toml`.

## Repo layout
```
.                      # SvelteKit frontend
├─ src/lib/api.ts      # backend API client (token + graceful fallback)
├─ src/lib/data/mock.ts# local fallback mock data
└─ services/manager/   # Rust backend (Axum + Tokio + SQLite)
```

## Stack
- SvelteKit + Svelte 5 (runes) + TypeScript
- Tailwind CSS v4 (custom ARK palette as design tokens in `src/app.css`)
- No UI dependency — bespoke component kit in `src/lib/components`

## Run the frontend
```bash
npm install
cp .env.example .env   # point VITE_ARK_API_BASE / VITE_ARK_API_TOKEN at the backend
npm run dev            # http://localhost:5173
npm run check          # svelte-check (0 errors)
npm run build          # production build
```
The UI works **without** the backend: each page renders local mock data and shows
a "backend unavailable" banner if the API call fails.

## Run the backend (`services/manager`)
Requires a Rust toolchain (`cargo`, edition 2021).
```bash
cd services/manager
cp manager.example.toml manager.toml         # edit cluster/maps/policy as needed

# Provide the API token via env (preferred over committing it):
export ARK_MANAGER_API_TOKEN="choose-a-strong-token"   # PowerShell: $env:ARK_MANAGER_API_TOKEN="..."

cargo run                 # binds 127.0.0.1:8787 by default
cargo fmt                 # format
cargo clippy --all-targets -- -D warnings
cargo test                # config validation + db migration tests
```
Optional env overrides: `ARK_MANAGER_CONFIG` (config path, default `manager.toml`),
`ARK_MANAGER_DB` (SQLite path, default `data/manager.db`), `RUST_LOG` (e.g. `debug`).

## Ubuntu Deployment
T1.1.5 deployed the backend on the private Tailscale host `100.68.7.42`.
Port `8787` was already in use there, so the manager is bound to
`100.68.7.42:8788`.

Deployment layout:
```text
/opt/ark-cluster-app                  # git checkout + release binary
/etc/ark-cluster-manager/manager.toml # production config, root:marcel 640
/var/lib/ark-cluster-manager          # SQLite DB/state
/var/log/ark-cluster-manager          # reserved for logs if needed
```

Systemd service:
```bash
sudo systemctl status ark-cluster-manager.service --no-pager
sudo journalctl -u ark-cluster-manager.service -n 100 --no-pager
sudo systemctl restart ark-cluster-manager.service
```

The service runs as `marcel`, uses the release binary at
`/opt/ark-cluster-app/services/manager/target/release/ark-manager`, and sets:
```text
ARK_MANAGER_CONFIG=/etc/ark-cluster-manager/manager.toml
ARK_MANAGER_DB=/var/lib/ark-cluster-manager/manager.db
```

The API token lives only in `/etc/ark-cluster-manager/manager.toml`; do not
commit it. To point the frontend at the deployed backend, copy `.env.example`
to `.env` and set:
```text
VITE_ARK_API_BASE=http://100.68.7.42:8788
VITE_ARK_API_TOKEN=<token from the server config>
```

Smoke tests:
```bash
curl -i http://100.68.7.42:8788/health
curl -i http://100.68.7.42:8788/api/status       # should return 401
curl -i -H "Authorization: Bearer <TOKEN>" http://100.68.7.42:8788/api/status
curl -s -H "Authorization: Bearer <TOKEN>" http://100.68.7.42:8788/api/resources
```

Expected results: `/health` is public, `/api/*` requires Bearer auth,
`/api/resources` reports `source: "host"` on Ubuntu, and `/api/servers`
reports read-only systemd state for configured units. ARK units may show
inactive/not-found until real ARK services exist.

## T1.2 Guarded Operations
T1.2 adds the foundation for real operations while keeping the default deployed
posture conservative.

Capability flags live in `manager.toml`:
```toml
[operations]
systemd_control_enabled = false
backup_enabled = true
rcon_enabled = false
allow_home_manual_stop = false
allow_travel_manual_stop = true
require_confirmation_token = true
travel_scheduler_enabled = false
travel_idle_shutdown_secs = 10800
config_writes_enabled = false
mod_management_enabled = false
maintenance_enabled = false
```

`GET /api/capabilities` tells the UI what is enabled and why something is
disabled. The example config keeps systemd control and RCON disabled. Enabling
systemd control should only happen on the private Ubuntu host after reviewing
the configured slot units.

Manual lifecycle actions are available only through configured server/slot IDs:
```text
POST /api/servers/:id/actions/start
POST /api/servers/:id/actions/stop
POST /api/servers/:id/actions/restart
```
The client never sends raw unit names. The backend resolves `:id` to configured
Home / Travel A / Travel B units, validates the unit name, uses `systemctl`
directly with fixed arguments, and writes audit rows for allowed and blocked
attempts. Stop/restart require confirmation. Home stop is blocked unless
`allow_home_manual_stop = true`, a strong confirmation is provided, and the
reason is `manual_admin_override` or `resource_standby_preparation`.

Manual backups use configured paths only:
```text
POST /api/servers/:id/actions/backup
```
Backup sources must be configured under the allowed ARK root, and destinations
must remain under the backup root. The first implementation creates a
timestamped directory copy for configured save/config paths. Missing source
paths produce a clear failed backup record instead of crashing. Restore/delete
remain disabled.

RCON setup is read-only groundwork:
```toml
[rcon]
enabled = false
poll_interval_seconds = 5

[slots.home.rcon]
host = "127.0.0.1"
port = 27020
password_env = "ARK_HOME_RCON_PASSWORD"
```
Passwords must come from environment variables and are never returned or logged.
T1.2/T1.3 exposes disabled/unconfigured status and chat command parsing helpers.
RCON generic command execution is intentionally not exposed.

## Server-Side Runtime Phase

This repo now includes idempotent Ubuntu setup artifacts:

```text
scripts/server/install_steamcmd.sh
scripts/server/install_or_update_ark_server.sh
scripts/server/prepare_ark_runtime.sh
deploy/systemd/ark-server@.service
deploy/systemd/ark-slot-start
deploy/systemd/slot.env.example
```

ARK: Survival Evolved uses SteamCMD app `376030`, shared install
`/srv/ark/server`, shared config
`/srv/ark/server/ShooterGame/Saved/Config/LinuxServer`, and shared cluster dir
`/srv/ark/clusters/main`. Home / Travel A / Travel B differ only through slot
env files and launch args: map, session, game/query/RCON ports,
`AltSaveDirectoryName`, cluster id, and safe flags.

New backend API:

```text
GET  /api/runtime
GET  /api/travel
POST /api/travel/request
GET  /api/travel/history
GET  /api/config
POST /api/config/set
GET  /api/mods
POST /api/mods/add|enable|disable|remove
GET  /api/maintenance/status
POST /api/maintenance/update/ark
```

Config writes, mod changes, maintenance jobs, travel scheduler, systemd control,
and RCON are independently disabled by default until the live Ubuntu config is
reviewed.

### API endpoints (T1.1, read-only)
`GET /health` is public. **Everything under `/api/*` requires
`Authorization: Bearer <token>`** and returns `401` otherwise.

| Endpoint | Returns |
|----------|---------|
| `GET /health` | liveness (no auth) |
| `GET /api/status` | cluster/manager/Tailscale/Discord placeholders + real/fallback resource pressure + systemd availability summary |
| `GET /api/capabilities` | guarded operation capability flags and disabled reasons |
| `GET /api/servers` | configured maps/slots with real/fallback read-only systemd status; players/RCON/restart flags remain mock |
| `GET /api/servers/:id` | one map + players + detailed read-only systemd status (404 if unknown) |
| `POST /api/servers/:id/actions/start` | guarded configured-unit systemd start |
| `POST /api/servers/:id/actions/stop` | guarded configured-unit systemd stop |
| `POST /api/servers/:id/actions/restart` | guarded configured-unit systemd restart |
| `POST /api/servers/:id/actions/backup` | safe configured-path backup |
| `GET /api/travel` | travel slots, active request, stepper, block reason (mock) |
| `GET /api/resources` | real Linux or fallback RAM/CPU/load/swap/disk/uptime + governor decision preview + thresholds |
| `GET /api/backups` | SQLite backup history first, mock fallback if empty |
| `GET /api/activity` | SQLite audit/activity log first, mock fallback if empty |
| `GET /api/rcon/status` | read-only RCON configuration/status surface |
| `GET /api/players` | player list, RCON-backed when available, mock otherwise |
| `GET /api/chat/recent` | recent chat/detected command placeholder |
| `GET /api/config` | editable config fields + raw `Game.ini`/`GameUserSettings.ini` (mock, read-only) |
| `GET /api/mods` | installed/active/disabled mods + load order (mock, read-only) |
| `GET /api/discord/status` | bot status, command list, recent events (mock) |
| `GET /api/settings` | cluster/private-access/travel/resource/backup/config-mod/security settings |

No destructive `POST`/`PATCH`/`DELETE` routes exist. systemd start/stop/restart,
RCON commands, config writes, and mod mutations are intentionally **not** wired.

### Auth token
- Set via `ARK_MANAGER_API_TOKEN` (preferred) or `[server].api_token` in `manager.toml`.
- The frontend sends it as `VITE_ARK_API_TOKEN`; the two must match.
- The token is **never logged** and never returned by the API.
- Smoke test:
  ```bash
  curl localhost:8787/health                                    # 200, no token
  curl -o /dev/null -w '%{http_code}' localhost:8787/api/status # 401
  curl -H "Authorization: Bearer $ARK_MANAGER_API_TOKEN" localhost:8787/api/status
  ```

### Private / Tailscale access (read this)
This dashboard is for **private access over Tailscale or LAN only** — never
public port-forwarding. The backend defaults to binding `127.0.0.1`; for tailnet
access set `[server].bind_address` to your `100.x.x.x` Tailscale IP. The backend
logs a warning if it binds to a non-private address. `VITE_ARK_API_*` values are
bundled into the client, so only point them at a private backend.

### Slot and systemd config
T1.1 supports explicit read-only server slots in `manager.toml`:

```toml
[slots.home]
id = "home"
label = "Home"
map_key = "home-island"
systemd_unit = "ark-server@home.service"
game_port = 7777
query_port = 27015
rcon_port = 27020
protected = true
home_resource_standby_enabled = true

[slots.travel_a]
id = "travel-a"
label = "Travel A"
map_key = "travel-rag"
systemd_unit = "ark-server@travel-a.service"
game_port = 7781
query_port = 27016
rcon_port = 27021

[slots.travel_b]
id = "travel-b"
label = "Travel B"
map_key = "travel-ab"
systemd_unit = "ark-server@travel-b.service"
game_port = 7785
query_port = 27017
rcon_port = 27022
```

On Linux, status is read with `systemctl show <unit>` using fixed arguments and
no shell. Unit names come only from validated config and must match a safe
`.service` pattern.

### Config & safety
`manager.toml` is validated on startup and the process aborts on: empty token,
non-IP bind address, mis-ordered RAM thresholds, unsafe paths (relative or
containing `..`), slot paths outside `cluster.directory`, unsafe systemd unit
names, duplicate slot ids, duplicate map ids, duplicate systemd units, duplicate
query/rcon/game ports, no Home-capable map, more Home assignments than one, or
more travel slots than `max_travel_servers`. Secrets must come from the
environment — do not commit a real token (`manager.toml` and `*.db` are
git-ignored).

### Backend layout
```
services/manager/
├─ manager.example.toml      # documented config template
├─ migrations/0001_init.sql  # servers, travel_requests, activity_log, backups,
│                            #   config_versions, mods, settings
└─ src/
   ├─ config.rs   # load + validate (+ tests)
   ├─ db.rs       # SQLite open/create + migrations (+ test)
   ├─ auth.rs     # Bearer-token middleware (constant-time compare)
   ├─ api.rs      # /api routes -> host/systemd readers + mock placeholders
   ├─ mock.rs     # fallback/mock data mirroring src/lib/data/mock.ts
   ├─ state.rs    # shared app state
   └─ models/     # domain + audit + governor + systemd + rcon models
```

## What is real now (T1.2)
- Linux host resource readings from `/proc/meminfo`, `/proc/stat`,
  `/proc/loadavg`, `/proc/uptime`, and read-only filesystem stats.
- Resource API source markers: `host`, `mock`, or `fallback`.
- Read-only systemd status for configured units, including load/active/sub
  state, description, timestamps, main PID, memory, tasks, and clear error
  messages when systemd is unavailable.
- Config-driven Home, Travel A, and Travel B slot/unit/port mapping.
- Bearer-token auth for `/api/*`; `/health` remains public.
- T1.2 operation capability flags exposed to the UI.
- Guarded systemd action endpoints for configured units only. They are disabled
  unless `operations.systemd_control_enabled = true`.
- Manual backup endpoint for configured save/config paths, with SQLite backup
  records and missing-path failure records.
- SQLite-backed activity feed when audit rows exist.
- RCON read-only status/config groundwork with no password exposure.

## Still mock or not implemented
- ARK start/stop/restart remains disabled in the example/deployed config unless
  deliberately enabled by the operator.
- Automatic `!travel` scheduling and Home Resource Standby execution are not implemented.
- RCON live sockets/player polling/chat polling remain disabled/placeholders
  unless configured later.
- Discord bot execution remains disabled/placeholder.
- Config file writes and raw config editing remain preview-only.
- Mod downloads, deletes, updates, and load-order mutation remain mock.
- Backup restore/delete are not implemented.

## Next phase suggestion
Add guarded systemd control for predefined, validated units only: audited
start/stop/restart endpoints, explicit Home protection checks, backup/save
preconditions, and no user-supplied unit names.

## Pages (`src/routes`)
| Route | Page |
|-------|------|
| `/` | Dashboard — cluster health, slots, Home protection, governor, activity, backups |
| `/maps` | Maps — card/table, action rules, confirm dialogs |
| `/maps/[id]` | Map detail — state timeline, players, resources, config, logs |
| `/travel` | Travel — destination picker, resource checks, TravelStepper, block/queue/standby results |
| `/resources` | Resources — host/fallback resource sample, per-process memory, governor policy |
| `/config` | Config editor — safe form + raw editor, diff, validation, backup-before-save |
| `/mods` | Mods — load order, downloads, enable/disable vs remove |
| `/backups` | Backups — table, restore/delete confirms, policy |
| `/logs` | Activity / Logs — filterable audit timeline |
| `/discord` | Discord bot — commands, permission model, alerts |
| `/settings` | Settings — cluster/Tailscale/policy/security, masked secrets |

## Key domain concepts modeled in the UI
- **Home protection** — Home preferred online, protected from travel rotation, may enter **Resource Standby** only when empty + under RAM pressure, auto-restarts on recovery.
- **Resource governor** — never stops maps with players; explains each decision in plain language.
- **Travel slots** — max 2; blocked/queued when both have players; can offer Home Standby to free RAM.
- **Safety** — confirmation dialogs (typed phrase for Home stop / mod delete / restore), disabled states with tooltips, restart-required + backup warnings, masked tokens, private-Tailscale-only warnings.

## Mock data
Single source: `src/lib/data/mock.ts` (maps, players, resources, governor, backups, mods, config, logs, Discord). Types in `src/lib/types.ts`. Tweak the scenario there — current state shows high RAM pressure with Home in Resource Standby and a blocked travel request.

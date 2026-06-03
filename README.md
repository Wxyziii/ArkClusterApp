# ARK Smart Cluster Manager

Dark, survival-tech admin dashboard for a private ARK: Survival Evolved smart cluster.

**Two parts, clearly separated:**
- **Frontend** — SvelteKit + TypeScript UI (this repo root, `src/`).
- **Backend** — Rust manager service (`services/manager/`), T1.1 read-only host awareness.

> ⚠️ **T1.1 scope.** The backend can read Linux host resources and read-only
> systemd unit status for configured ARK slots. It does **NOT** start, stop,
> restart, enable, disable, or reload ARK units. It does not send RCON commands,
> run Discord bot actions, write ARK config files, or download/remove mods. The
> UI consumes real backend data where available and falls back to local mock data
> when the backend or host capability is unavailable.

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

### API endpoints (T1.1, read-only)
`GET /health` is public. **Everything under `/api/*` requires
`Authorization: Bearer <token>`** and returns `401` otherwise.

| Endpoint | Returns |
|----------|---------|
| `GET /health` | liveness (no auth) |
| `GET /api/status` | cluster/manager/Tailscale/Discord placeholders + real/fallback resource pressure + systemd availability summary |
| `GET /api/servers` | configured maps/slots with real/fallback read-only systemd status; players/RCON/restart flags remain mock |
| `GET /api/servers/:id` | one map + players + detailed read-only systemd status (404 if unknown) |
| `GET /api/travel` | travel slots, active request, stepper, block reason (mock) |
| `GET /api/resources` | real Linux or fallback RAM/CPU/load/swap/disk/uptime + governor decision preview + thresholds |
| `GET /api/backups` | backup history + policy (mock) |
| `GET /api/activity` | audit/activity log (mock) |
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

## What is real now (T1.1)
- Linux host resource readings from `/proc/meminfo`, `/proc/stat`,
  `/proc/loadavg`, `/proc/uptime`, and read-only filesystem stats.
- Resource API source markers: `host`, `mock`, or `fallback`.
- Read-only systemd status for configured units, including load/active/sub
  state, description, timestamps, main PID, memory, tasks, and clear error
  messages when systemd is unavailable.
- Config-driven Home, Travel A, and Travel B slot/unit/port mapping.
- Bearer-token auth for `/api/*`; `/health` remains public.

## Still mock or not implemented
- ARK start/stop/restart control remains disabled and not routed.
- RCON sockets, player counts, and in-game commands remain mock/placeholders.
- Discord bot execution remains disabled/placeholder.
- Config file writes and raw config editing remain preview-only.
- Mod downloads, deletes, updates, and load-order mutation remain mock.
- Backups are displayed from mock data; no backup command is executed.
- SQLite schema exists and startup audit events are persisted, but most handlers
  still serve in-memory mock data plus the new host/systemd readings.

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

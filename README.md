# ARK Smart Cluster Manager

Dark, survival-tech admin dashboard for a private ARK: Survival Evolved smart cluster.

**Two parts, clearly separated:**
- **Frontend** — SvelteKit + TypeScript UI (this repo root, `src/`).
- **Backend** — Rust manager service (`services/manager/`), Phase 1 skeleton.

> ⚠️ **Phase 1 scope.** The backend is a *safe skeleton*: config + API contract +
> token auth + SQLite + **mock data**. It does **NOT** control ARK servers, RCON,
> systemd, Discord, or Steam Workshop mods, and writes no ARK config files. All
> control-plane surfaces are inert data models. The UI consumes the backend where
> practical and falls back to local mock data when the backend is unavailable.

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
cargo clippy --all-targets
cargo test                # config validation + db migration tests
```
Optional env overrides: `ARK_MANAGER_CONFIG` (config path, default `manager.toml`),
`ARK_MANAGER_DB` (SQLite path, default `data/manager.db`), `RUST_LOG` (e.g. `debug`).

### API endpoints (Phase 1, read-only)
`GET /health` is public. **Everything under `/api/*` requires
`Authorization: Bearer <token>`** and returns `401` otherwise.

| Endpoint | Returns |
|----------|---------|
| `GET /health` | liveness (no auth) |
| `GET /api/status` | cluster/manager/Tailscale/Discord/systemd status + resource pressure (mock) |
| `GET /api/servers` | all maps (state, players, RCON/systemd/memory/uptime/restart — mock) |
| `GET /api/servers/:id` | one map + its players (404 if unknown) |
| `GET /api/travel` | travel slots, active request, stepper, block reason (mock) |
| `GET /api/resources` | RAM/CPU/swap/disk + governor decision preview + thresholds |
| `GET /api/backups` | backup history + policy (mock) |
| `GET /api/activity` | audit/activity log (mock) |
| `GET /api/config` | editable config fields + raw `Game.ini`/`GameUserSettings.ini` (mock, read-only) |
| `GET /api/mods` | installed/active/disabled mods + load order (mock, read-only) |
| `GET /api/discord/status` | bot status, command list, recent events (mock) |
| `GET /api/settings` | cluster/private-access/travel/resource/backup/config-mod/security settings |

No destructive `POST`/`PATCH`/`DELETE` routes exist. systemd start/stop/restart,
config writes, and mod mutations are intentionally **not** wired.

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

### Config & safety
`manager.toml` is validated on startup and the process aborts on: empty token,
non-IP bind address, mis-ordered RAM thresholds, unsafe paths (relative or
containing `..`), duplicate map ids, duplicate query/rcon/game ports, no
Home-capable map, more Home assignments than one, or more travel slots than
`max_travel_servers`. Secrets must come from the environment — do not commit a
real token (`manager.toml` and `*.db` are git-ignored).

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
   ├─ api.rs      # /api routes -> mock data
   ├─ mock.rs     # mock data mirroring src/lib/data/mock.ts
   ├─ state.rs    # shared app state
   └─ models/     # domain + audit + governor + systemd + rcon models
```

## Current limitations (Phase 1)
- All server/RCON/systemd/Discord/mod/backup data is **mock**; nothing is actuated.
- No ARK config files are read or written. Config/mod editors are preview-only.
- Resource governor, systemd controller, and RCON listener are **data models** —
  `start/stop/restart` return "not implemented" and are not exposed.
- SQLite schema is created and migrated, but handlers serve in-memory mock data;
  only startup audit events are persisted.

## Next phase suggestions
1. Implement the systemd controller behind `SystemdController` (still no public
   start/stop until confirmed safe) and read real unit status via `systemctl`.
2. Add a real resource sampler (e.g. `/proc`, `sysinfo`) feeding the governor.
3. Wire RCON connections + the all-map chat listener for `!travel <map>`.
4. Persist + serve real data from SQLite (servers/backups/activity).
5. Add guarded, audited write endpoints (config edit, mod reorder) with
   backup-before-change, then the Discord bot.

## Pages (`src/routes`)
| Route | Page |
|-------|------|
| `/` | Dashboard — cluster health, slots, Home protection, governor, activity, backups |
| `/maps` | Maps — card/table, action rules, confirm dialogs |
| `/maps/[id]` | Map detail — state timeline, players, resources, config, logs |
| `/travel` | Travel — destination picker, resource checks, TravelStepper, block/queue/standby results |
| `/resources` | Resources — charts (mock), per-process memory, governor policy |
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

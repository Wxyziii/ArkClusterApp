// API client for the Rust manager backend.
//
// The UI is designed to keep working with no backend: every call has a mock
// fallback (see `loadWithFallback`). Configure the backend via env vars:
//   VITE_ARK_API_BASE   default http://100.68.7.42:8788
//   VITE_ARK_API_TOKEN  Bearer token for /api/* (matches manager.toml)
// See `.env.example`.

import type { ArkMap, Backup, ConfigField, LogEvent, Mod, Player, ResourceSample } from '$lib/types';

const BASE: string =
  (import.meta.env.VITE_ARK_API_BASE as string | undefined)?.replace(/\/$/, '') ??
  'http://100.68.7.42:8788';
const TOKEN: string = (import.meta.env.VITE_ARK_API_TOKEN as string | undefined) ?? '';
const DEFAULT_TIMEOUT_MS = 8000;

export class ApiError extends Error {
  constructor(
    public status: number,
    message: string,
    public code = 'API_ERROR',
    public payload: unknown = null
  ) {
    super(message);
    this.name = 'ApiError';
  }
}

/** Authenticated GET against the manager API. Throws ApiError on failure. */
export async function apiGet<T>(path: string, signal?: AbortSignal): Promise<T> {
  const timeout = timeoutSignal(signal);
  let res: Response;
  try {
    res = await fetch(`${BASE}/api${path}`, {
      headers: TOKEN ? { Authorization: `Bearer ${TOKEN}` } : {},
      signal: timeout.signal
    });
  } catch (e) {
    // Network error / backend down.
    throw new ApiError(0, e instanceof Error ? e.message : 'network error', 'NETWORK_ERROR');
  } finally {
    timeout.cancel();
  }
  if (!res.ok) {
    throw await apiErrorFromResponse(res);
  }
  return (await res.json()) as T;
}

export async function apiPost<T>(
  path: string,
  body: unknown,
  signal?: AbortSignal
): Promise<T> {
  const timeout = timeoutSignal(signal);
  let res: Response;
  try {
    res = await fetch(`${BASE}/api${path}`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...(TOKEN ? { Authorization: `Bearer ${TOKEN}` } : {})
      },
      body: JSON.stringify(body),
      signal: timeout.signal
    });
  } catch (e) {
    throw new ApiError(0, e instanceof Error ? e.message : 'network error', 'NETWORK_ERROR');
  } finally {
    timeout.cancel();
  }
  if (!res.ok) {
    throw await apiErrorFromResponse(res);
  }
  return (await res.json()) as T;
}

/** Unauthenticated health check. Returns true if the backend is reachable. */
export async function health(signal?: AbortSignal): Promise<boolean> {
  const timeout = timeoutSignal(signal);
  try {
    const res = await fetch(`${BASE}/health`, { signal: timeout.signal });
    return res.ok;
  } catch {
    return false;
  } finally {
    timeout.cancel();
  }
}

function timeoutSignal(signal?: AbortSignal) {
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), DEFAULT_TIMEOUT_MS);
  if (signal) signal.addEventListener('abort', () => controller.abort(), { once: true });
  return { signal: controller.signal, cancel: () => clearTimeout(timeout) };
}

async function apiErrorFromResponse(res: Response): Promise<ApiError> {
  let message = `${res.status} ${res.statusText}`;
  let code = 'HTTP_ERROR';
  let payload: unknown = null;
  try {
    const data = await res.json();
    payload = data;
    message = data?.error?.message ?? data?.reason ?? data?.message ?? message;
    code = data?.error?.code ?? data?.status ?? code;
  } catch {
    // keep status text
  }
  return new ApiError(res.status, message, code, payload);
}

export interface Loaded<T> {
  data: T;
  /** True when the value came from the local mock fallback, not the backend. */
  fromFallback: boolean;
  /** Populated when the backend call failed (and fallback was used). */
  error: string | null;
}

/**
 * Run an API fetch but never throw to the UI: on any failure, return the
 * provided mock fallback and flag it. This keeps the dashboard usable when the
 * backend is unavailable.
 */
export async function loadWithFallback<T>(
  fetcher: () => Promise<T>,
  fallback: T
): Promise<Loaded<T>> {
  try {
    return { data: await fetcher(), fromFallback: false, error: null };
  } catch (e) {
    const error = e instanceof Error ? e.message : 'unknown error';
    return { data: fallback, fromFallback: true, error };
  }
}

// ---- Typed endpoint helpers (shapes match src/lib/types.ts) ----

export interface ClusterStatus {
  cluster: {
    name: string;
    id: string;
    directory: string;
    managerVersion: string;
    maxTravelServers: number;
    emptyShutdownMins: number;
  };
  manager: { status: string; tone: string };
  tailscale: { status: string; tone: string; bindPrivate: boolean; bindAddress: string };
  discord: { status: string; tone: string };
  systemd: {
    status: string;
    tone: string;
    available?: boolean;
    source?: string;
    activeUnits?: number;
    failedUnits?: number;
    checkedUnits?: number;
  };
  resourcePressure: {
    ramPct: number;
    label: string;
    tone: string;
    source?: string;
    load1?: number;
    load5?: number;
    load15?: number;
  };
  players: number;
  runningMaps: number;
}

export interface ResourcesResponse {
  sample: ResourceSample;
  derived: {
    ramPct: number;
    cpuPct: number;
    swapPct: number;
    diskPct: number;
    pressure: { label: string; tone: string };
  };
  thresholds: Record<string, number>;
  governor: {
    decision: string;
    why: string;
    examples: string[];
    policy: {
      neverStopWithPlayers: boolean;
      homeStandbyEnabled: boolean;
      homeStopsOnlyWhenEmpty: boolean;
      preferActivePlayerMaps: boolean;
      autoRestartHome: boolean;
      maxTravelServers?: number;
      emptyShutdownMins?: number;
    };
  };
  source: string;
  uptime: { managerSecs: number; systemSecs?: number | null };
  loadAverage: { one: number; five: number; fifteen: number };
  perProcess: { map: string; ramMb: number; cpuPct: number }[];
}

export interface CapabilityItem {
  enabled: boolean;
  available: boolean;
  reason: string;
}

export interface Capabilities {
  systemdControl: CapabilityItem;
  backup: CapabilityItem;
  rcon: CapabilityItem;
  discord: CapabilityItem;
  configWrites: CapabilityItem;
  modManagement: CapabilityItem;
  restore: CapabilityItem;
  travelScheduler?: CapabilityItem;
  maintenance?: CapabilityItem;
  mode: string;
  backendSource: string;
}

export interface ActionRequest {
  confirm: boolean;
  strongConfirm?: boolean;
  adminOverride?: boolean;
  reason?: string;
}

export interface ActionResponse {
  accepted: boolean;
  actionId: string;
  serverId: string;
  operation: string;
  result: string;
  message: string;
  auditEventId?: number | null;
  updatedStatus?: unknown;
  backup?: Backup;
}

export interface RuntimeStatus {
  ready: boolean;
  steamcmd: { ok: boolean; path: string; message: string };
  arkServer: { ok: boolean; path: string; message: string };
  sharedConfig: { ok: boolean; path: string; message: string };
  clusterDir: { ok: boolean; path: string; message: string };
  backupRoot: { ok: boolean; path: string; message: string };
  arkRoot: string;
  serverRoot: string;
  executable: string;
}

export interface TravelState {
  enabled: boolean;
  idleShutdownSecs: number;
  idleShutdownProduction: boolean;
  maxTravelServers: number;
  homeResourceStandby: boolean;
  slots: { home?: ArkMap; travelA?: ArkMap; travelB?: ArkMap };
  recent: unknown[];
  queue: unknown[];
  blockReason?: string;
}

export interface TravelDecision {
  id: string;
  accepted: boolean;
  requestedMap: string;
  resolvedMap?: string;
  chosenSlot?: string;
  status: string;
  reason: string;
}

export interface ConfigResponse {
  fields: ConfigField[];
  gameIni: string;
  gameUserSettingsIni: string;
  restartRequired: boolean;
  writable: boolean;
  shared?: {
    writable: boolean;
    sharedConfigDir: string;
    gameIniPath: string;
    gameUserSettingsIniPath: string;
    gameIni: string;
    gameUserSettingsIni: string;
    masked: boolean;
  };
}

export interface ModsResponse {
  mods: Array<Mod | ModRecord>;
  mutable: boolean;
  steamcmdRequired?: boolean;
  restartRequired?: boolean;
  activeModsConfig?: string;
  testModId?: string;
}

export interface ModRecord {
  workshopId: string;
  name: string;
  enabled: number | boolean;
  installed: number | boolean;
  loadOrder: number;
  lastUpdated?: string | null;
  status: string;
  error?: string | null;
}

export interface ModLookupResponse {
  workshopId: string;
  name: string;
  url: string;
  game: string;
  installAvailable: boolean;
  mutable: boolean;
  disabledReason?: string;
}

export interface MaintenanceStatus {
  enabled: boolean;
  steamAppId: string;
  installPath: string;
  safeCommand: string;
  jobs: unknown[];
}

export interface DiscordStatusResponse {
  status: {
    online: boolean;
    guild: string;
    statusChannel: string;
    lastHeartbeat: string;
    permissionsOk: boolean;
    implemented: boolean;
    service?: {
      active: boolean;
      enabled: boolean;
      activeState: string;
      subState: string;
    };
    dashboard?: {
      category: string;
      channels: string[];
      stateFile: string;
    };
  };
  commands: { cmd?: string; name?: string; desc?: string; access?: string }[];
  events: { id?: string; ts?: string; kind?: string; text?: string }[];
  alertSettings: { key: string; label: string; enabled: boolean }[];
}

export const api = {
  status: (s?: AbortSignal) => apiGet<ClusterStatus>('/status', s),
  runtime: (s?: AbortSignal) => apiGet<RuntimeStatus>('/runtime', s),
  capabilities: (s?: AbortSignal) => apiGet<Capabilities>('/capabilities', s),
  servers: (s?: AbortSignal) => apiGet<ArkMap[]>('/servers', s),
  server: (id: string, s?: AbortSignal) =>
    apiGet<{ server: ArkMap; players: Player[] }>(`/servers/${encodeURIComponent(id)}`, s),
  resources: (s?: AbortSignal) => apiGet<ResourcesResponse>('/resources', s),
  travel: (s?: AbortSignal) => apiGet<TravelState>('/travel', s),
  travelHistory: (s?: AbortSignal) => apiGet<{ history: unknown[] }>('/travel/history', s),
  travelRequest: (body: { map: string; source: string; actor: string }, s?: AbortSignal) =>
    apiPost<TravelDecision>('/travel/request', body, s),
  serverAction: (id: string, action: 'start' | 'stop' | 'restart' | 'backup', body: ActionRequest, s?: AbortSignal) =>
    apiPost<ActionResponse>(`/servers/${encodeURIComponent(id)}/actions/${action}`, body, s),
  backups: (s?: AbortSignal) =>
    apiGet<{ backups: Backup[]; policy: Record<string, unknown> }>('/backups', s),
  activity: (s?: AbortSignal) =>
    apiGet<{ activity: LogEvent[]; recent: LogEvent[] }>('/activity', s),
  rconStatus: (s?: AbortSignal) => apiGet<Record<string, unknown>>('/rcon/status', s),
  players: (s?: AbortSignal) => apiGet<{ players: Player[]; source: string; rconEnabled: boolean }>('/players', s),
  chatRecent: (s?: AbortSignal) => apiGet<Record<string, unknown>>('/chat/recent', s),
  config: (s?: AbortSignal) => apiGet<ConfigResponse>('/config', s),
  configRaw: (s?: AbortSignal) => apiGet<ConfigResponse>('/config/raw', s),
  configPreview: (body: { file: string; key: string; value: string; confirm?: boolean; reason?: string }, s?: AbortSignal) =>
    apiPost<Record<string, unknown>>('/config/preview', body, s),
  configApply: (body: { file: string; key: string; value: string; confirm: boolean; reason?: string }, s?: AbortSignal) =>
    apiPost<ConfigResponse>('/config/apply', body, s),
  configVersions: (s?: AbortSignal) => apiGet<{ versions: Record<string, unknown>[] }>('/config/versions', s),
  mods: (s?: AbortSignal) => apiGet<ModsResponse>('/mods', s),
  modLookup: (body: { workshopId?: string; url?: string }, s?: AbortSignal) =>
    apiPost<ModLookupResponse>('/mods/lookup', body, s),
  modAction: (action: 'add' | 'update' | 'enable' | 'disable' | 'remove', body: { workshopId: string; confirm: boolean }, s?: AbortSignal) =>
    apiPost<ModsResponse>(`/mods/${action}`, body, s),
  maintenance: (s?: AbortSignal) => apiGet<MaintenanceStatus>('/maintenance/status', s),
  maintenanceDryRun: (s?: AbortSignal) =>
    apiPost<Record<string, unknown>>('/maintenance/ark/update', { dryRun: true, confirm: false, reason: 'web_ui_dry_run' }, s),
  discordStatus: (s?: AbortSignal) => apiGet<DiscordStatusResponse>('/discord/status', s),
  settings: (s?: AbortSignal) => apiGet<Record<string, unknown>>('/settings', s)
};

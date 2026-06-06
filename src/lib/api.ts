import type { ArkMap, Backup, ConfigField, LogEvent, Mod, Player, ResourceSample } from '$lib/types';

const BASE = ((import.meta.env.VITE_ARK_API_BASE as string | undefined) ?? '').replace(/\/$/, '');
const TOKEN = (import.meta.env.VITE_ARK_API_TOKEN as string | undefined) ?? '';
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

export async function apiGet<T>(path: string, signal?: AbortSignal): Promise<T> {
  return request<T>('GET', path, undefined, signal);
}

export async function apiPost<T>(path: string, body: unknown, signal?: AbortSignal): Promise<T> {
  return request<T>('POST', path, body, signal);
}

async function request<T>(
  method: 'GET' | 'POST',
  path: string,
  body?: unknown,
  signal?: AbortSignal
): Promise<T> {
  if (!TOKEN) {
    throw new ApiError(0, 'Missing VITE_ARK_API_TOKEN in frontend environment', 'AUTH_MISSING');
  }
  const timeout = timeoutSignal(signal);
  let res: Response;
  try {
    res = await fetch(`${BASE}/api${path}`, {
      method,
      headers: {
        Authorization: `Bearer ${TOKEN}`,
        ...(body === undefined ? {} : { 'Content-Type': 'application/json' })
      },
      body: body === undefined ? undefined : JSON.stringify(body),
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
    // keep HTTP status text
  }
  return new ApiError(res.status, message, code, payload);
}

export interface ClusterStatus {
  dataMode?: 'live' | 'demo';
  cluster: {
    name: string;
    id: string;
    directory: string;
    managerVersion: string;
    maxTravelServers: number;
    emptyShutdownMins: number;
  };
  manager: { status: string; tone: string };
  tailscale: {
    status: string;
    tone: string;
    bindPrivate: boolean;
    bindAddress: string;
    source?: string;
    connected?: boolean | null;
  };
  discord: { status: string; tone: string; source?: string };
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
  players: number | null;
  playerCountSource?: string;
  runningMaps: number;
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
    policy: Record<string, unknown>;
  };
  source: string;
  uptime: { managerSecs: number; systemSecs?: number | null };
  loadAverage: { one: number; five: number; fifteen: number };
  perProcess: { map: string; ramMb: number; cpuPct: number }[];
}

export interface TravelSlotState {
  slotId: string;
  role: string;
  mapKey: string;
  map?: ArkMap | null;
  unit: string;
  systemd: string;
  active: boolean;
  playerCount: number | null;
  playerCountSource: string;
  idleShutdownSecs: number;
  policy: string;
}

export interface TravelState {
  enabled: boolean;
  idleShutdownSecs: number;
  idleShutdownProduction: boolean;
  maxTravelServers: number;
  homeResourceStandby: boolean;
  slots: TravelSlotState[];
  destinations: ArkMap[];
  recent: unknown[];
  queue: unknown[];
  blockReason?: string | null;
}

export interface TravelDecision {
  id: string;
  accepted: boolean;
  requestedMap: string;
  resolvedMap?: string;
  resolvedMapName?: string;
  chosenSlot?: string;
  status: string;
  reason: string;
  userMessage: string;
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
  activeModIds?: string[];
}

export interface ModRecord {
  workshopId: string;
  name?: string | null;
  enabled: number | boolean;
  installed: number | boolean;
  loadOrder: number;
  lastUpdated?: string | null;
  status: string;
  error?: string | null;
}

export interface ModLookupResponse {
  workshopId: string;
  name?: string | null;
  url: string;
  game: string;
  installAvailable: boolean;
  mutable: boolean;
  metadataSource?: string;
  metadataAvailable?: boolean;
  reason?: string;
  disabledReason?: string;
}

export interface DiscordStatusResponse {
  status: {
    online: boolean;
    guild: string;
    statusChannel: string;
    lastHeartbeat: string;
    permissionsOk: boolean | null;
    implemented: boolean;
    source?: string;
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
  commandsSource?: string;
  eventsSource?: string;
}

export const api = {
  status: (s?: AbortSignal) => apiGet<ClusterStatus>('/status', s),
  capabilities: (s?: AbortSignal) => apiGet<Capabilities>('/capabilities', s),
  servers: (s?: AbortSignal) => apiGet<ArkMap[]>('/servers', s),
  server: (id: string, s?: AbortSignal) =>
    apiGet<{ server: ArkMap; players: Player[]; playerCountSource: string; available: boolean; reason: string }>(
      `/servers/${encodeURIComponent(id)}`,
      s
    ),
  travel: (s?: AbortSignal) => apiGet<TravelState>('/travel', s),
  travelRequest: (body: { map: string; source: string; actor: string }, s?: AbortSignal) =>
    apiPost<TravelDecision>('/travel/request', body, s),
  serverAction: (
    id: string,
    action: 'start' | 'stop' | 'restart' | 'backup',
    body: ActionRequest,
    s?: AbortSignal
  ) => apiPost<ActionResponse>(`/servers/${encodeURIComponent(id)}/actions/${action}`, body, s),
  resources: (s?: AbortSignal) => apiGet<ResourcesResponse>('/resources', s),
  backups: (s?: AbortSignal) =>
    apiGet<{ backups: Backup[]; policy: Record<string, unknown> }>('/backups', s),
  activity: (s?: AbortSignal) =>
    apiGet<{ activity: LogEvent[]; recent: LogEvent[]; source: string; empty?: boolean }>('/activity', s),
  rconStatus: (s?: AbortSignal) => apiGet<Record<string, unknown>>('/rcon/status', s),
  players: (s?: AbortSignal) =>
    apiGet<{ players: Player[]; source: string; rconEnabled: boolean; available: boolean; reason: string }>(
      '/players',
      s
    ),
  chatRecent: (s?: AbortSignal) => apiGet<Record<string, unknown>>('/chat/recent', s),
  config: (s?: AbortSignal) => apiGet<ConfigResponse>('/config', s),
  configRaw: (s?: AbortSignal) => apiGet<ConfigResponse>('/config/raw', s),
  configApply: (
    body: { file: string; key: string; value: string; confirm: boolean; reason?: string },
    s?: AbortSignal
  ) => apiPost<ConfigResponse>('/config/apply', body, s),
  configVersions: (s?: AbortSignal) => apiGet<{ versions: Record<string, unknown>[] }>('/config/versions', s),
  mods: (s?: AbortSignal) => apiGet<ModsResponse>('/mods', s),
  modLookup: (body: { workshopId?: string; url?: string }, s?: AbortSignal) =>
    apiPost<ModLookupResponse>('/mods/lookup', body, s),
  modAction: (
    action: 'add' | 'update' | 'enable' | 'disable' | 'remove',
    body: { workshopId: string; confirm: boolean },
    s?: AbortSignal
  ) => apiPost<ModsResponse>(`/mods/${action}`, body, s),
  discordStatus: (s?: AbortSignal) => apiGet<DiscordStatusResponse>('/discord/status', s)
};

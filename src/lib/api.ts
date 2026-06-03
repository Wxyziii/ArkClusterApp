// API client for the Rust manager backend.
//
// The UI is designed to keep working with no backend: every call has a mock
// fallback (see `loadWithFallback`). Configure the backend via env vars:
//   VITE_ARK_API_BASE   default http://127.0.0.1:8787
//   VITE_ARK_API_TOKEN  Bearer token for /api/* (matches manager.toml)
// See `.env.example`.

import type { ArkMap, Backup, ConfigField, LogEvent, Mod, Player, ResourceSample } from '$lib/types';

const BASE: string =
  (import.meta.env.VITE_ARK_API_BASE as string | undefined)?.replace(/\/$/, '') ??
  'http://127.0.0.1:8787';
const TOKEN: string = (import.meta.env.VITE_ARK_API_TOKEN as string | undefined) ?? '';

export class ApiError extends Error {
  constructor(
    public status: number,
    message: string
  ) {
    super(message);
    this.name = 'ApiError';
  }
}

/** Authenticated GET against the manager API. Throws ApiError on failure. */
export async function apiGet<T>(path: string, signal?: AbortSignal): Promise<T> {
  let res: Response;
  try {
    res = await fetch(`${BASE}/api${path}`, {
      headers: TOKEN ? { Authorization: `Bearer ${TOKEN}` } : {},
      signal
    });
  } catch (e) {
    // Network error / backend down.
    throw new ApiError(0, e instanceof Error ? e.message : 'network error');
  }
  if (!res.ok) {
    throw new ApiError(res.status, `${res.status} ${res.statusText}`);
  }
  return (await res.json()) as T;
}

/** Unauthenticated health check. Returns true if the backend is reachable. */
export async function health(signal?: AbortSignal): Promise<boolean> {
  try {
    const res = await fetch(`${BASE}/health`, { signal });
    return res.ok;
  } catch {
    return false;
  }
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

export const api = {
  status: (s?: AbortSignal) => apiGet<ClusterStatus>('/status', s),
  servers: (s?: AbortSignal) => apiGet<ArkMap[]>('/servers', s),
  server: (id: string, s?: AbortSignal) =>
    apiGet<{ server: ArkMap; players: Player[] }>(`/servers/${encodeURIComponent(id)}`, s),
  resources: (s?: AbortSignal) => apiGet<ResourcesResponse>('/resources', s),
  backups: (s?: AbortSignal) =>
    apiGet<{ backups: Backup[]; policy: Record<string, unknown> }>('/backups', s),
  activity: (s?: AbortSignal) =>
    apiGet<{ activity: LogEvent[]; recent: LogEvent[] }>('/activity', s),
  config: (s?: AbortSignal) =>
    apiGet<{
      fields: ConfigField[];
      gameIni: string;
      gameUserSettingsIni: string;
      restartRequired: boolean;
      writable: boolean;
    }>('/config', s),
  mods: (s?: AbortSignal) =>
    apiGet<{ mods: Mod[]; restartRequired: boolean; mutable: boolean }>('/mods', s),
  settings: (s?: AbortSignal) => apiGet<Record<string, unknown>>('/settings', s)
};

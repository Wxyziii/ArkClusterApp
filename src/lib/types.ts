// ARK Smart Cluster Manager — shared domain types (mock prototype)

export type Tone = 'green' | 'amber' | 'red' | 'gray' | 'cyan' | 'accent';

export type MapState =
  | 'Offline'
  | 'Starting'
  | 'Online'
  | 'Ready'
  | 'Draining'
  | 'Saving'
  | 'Backing Up'
  | 'Stopping'
  | 'Resource Standby'
  | 'Error'
  | 'Unknown';

export type RoleCapability = 'Home-capable' | 'Travel-capable' | 'Disabled';
export type SlotAssignment = 'Home' | 'Travel A' | 'Travel B' | 'Unassigned';
export type RconStatus = 'Connected' | 'Connecting' | 'Disconnected';
export type SystemdStatus =
  | 'active (running)'
  | 'activating'
  | 'inactive (dead)'
  | 'failed'
  | 'unknown'
  | 'systemd unavailable';

export interface SystemdDetail {
  unit: string;
  source: string;
  exists: boolean;
  loaded: boolean;
  state: SystemdStatus;
  active: boolean;
  activeState: string;
  subState: string;
  description?: string;
  since?: string;
  mainPid?: number;
  memoryCurrentBytes?: number;
  tasksCurrent?: number;
  error?: string;
}

export interface Player {
  name: string;
  level: number;
  tribe: string;
  connectedMins: number;
  map: string;
}

export interface MapConfigSummary {
  systemdUnit: string;
  arkMapName: string;
  queryPort: number;
  rconPort: number;
  gamePort: number;
  slotPriority: number;
  autoShutdownEnabled: boolean;
  canBeHome: boolean;
  canAutoStopWhenEmpty: boolean;
  canEnterStandby: boolean;
  modList: string[];
}

export interface ArkMap {
  id: string;
  name: string;
  alias: string;
  role: RoleCapability;
  assignment: SlotAssignment;
  state: MapState;
  players: number;
  maxPlayers: number;
  ramMb: number;
  ramEstimateMb: number;
  uptimeMins: number;
  idleMins: number;
  lastBackup: string;
  rcon: RconStatus;
  systemd: SystemdStatus;
  restartRequired: boolean;
  cpuPct: number;
  saveSizeMb: number;
  isHome: boolean;
  protected: boolean;
  nextAction: string;
  config: MapConfigSummary;
  systemdDetail?: SystemdDetail;
}

export interface TravelRequest {
  id: string;
  map: string;
  requestedBy: string;
  source: 'In-game chat' | 'Discord command' | 'Web UI';
  sourceRaw: string;
  sourceMap: string;
  step: number;
  result: 'Ready' | 'Starting' | 'Blocked' | 'Queued' | 'Failed' | 'Already online';
  reason: string;
  at: string;
}

export interface ResourceSample {
  source: 'host' | 'mock' | 'fallback' | string;
  ramUsedGb: number;
  ramTotalGb: number;
  ramAvailableGb: number;
  cpuPct: number;
  swapUsedGb: number;
  swapTotalGb: number;
  diskUsedGb: number;
  diskTotalGb: number;
  diskFreeGb: number;
  arkProcMemGb: number;
  load1: number;
  load5: number;
  load15: number;
  managerUptimeSecs: number;
  systemUptimeSecs?: number | null;
}

export interface LogEvent {
  id: string;
  ts: string;
  severity: 'info' | 'warn' | 'error' | 'success';
  source: 'Map' | 'Travel' | 'Governor' | 'RCON' | 'Discord' | 'Config' | 'Mod' | 'Backup' | 'User';
  actor: string;
  targetMap: string;
  message: string;
  detail: string;
}

export interface Backup {
  id: string;
  map: string;
  type: 'save' | 'config' | 'mod' | 'cluster data' | 'combined';
  sizeMb: number;
  sizeBytes?: number;
  created: string;
  createdAt?: string;
  completedAt?: string | null;
  reason: 'manual' | 'manual_admin_action' | 'auto-shutdown' | 'resource standby' | 'scheduled' | 'pre-update' | 'pre-config-change' | 'pre-mod-change';
  status: 'success' | 'running' | 'failed' | 'verifying';
  path?: string;
  source?: string;
  progress?: number;
  error?: string;
}

export interface Mod {
  id: string;
  name: string;
  workshopId: string;
  enabled: boolean;
  installed: boolean;
  loadOrder: number;
  lastUpdated: string;
  sizeMb: number;
  usedBy: string[];
  state: 'active' | 'disabled' | 'downloading' | 'failed' | 'missing';
  progress?: number;
}

export interface ConfigField {
  key: string;
  label: string;
  value: number | boolean | string;
  type: 'number' | 'bool' | 'enum';
  group: string;
  min?: number;
  max?: number;
  step?: number;
  options?: string[];
  hint: string;
  restartRequired: boolean;
}

export interface DiscordEvent {
  id: string;
  ts: string;
  kind: string;
  text: string;
}

export interface AlertSetting {
  key: string;
  label: string;
  enabled: boolean;
}

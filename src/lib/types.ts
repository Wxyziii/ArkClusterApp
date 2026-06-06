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
  | 'Not running'
  | 'Error'
  | 'Unknown'
  | 'Unavailable';

export type RoleCapability = 'Home-capable' | 'Travel-capable' | 'Disabled' | 'Not configured';
export type SlotAssignment = 'Home' | 'On-demand' | 'Unassigned' | 'Not configured' | string;
export type RconStatus = 'Connected' | 'Connecting' | 'Disconnected' | 'Disabled' | 'Unavailable' | string;
export type SystemdStatus =
  | 'active (running)'
  | 'activating'
  | 'inactive (dead)'
  | 'failed'
  | 'unknown'
  | 'systemd unavailable'
  | 'not configured'
  | string;

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
  playerCountSource: string;
  maxPlayers: number | null;
  maxPlayersSource: string;
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
  configured: boolean;
  launchReady: boolean;
  unavailableReason?: string;
  slotId?: string;
  slotRole: string;
  nextAction: string;
  config: MapConfigSummary;
  systemdDetail?: SystemdDetail;
}

export interface ResourceSample {
  source: 'host' | 'unavailable' | string;
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
  severity: 'info' | 'warn' | 'error' | 'success' | string;
  source: string;
  actor: string;
  targetMap: string;
  message: string;
  detail: string;
}

export interface Backup {
  id: string;
  map: string;
  type: string;
  sizeMb: number;
  sizeBytes?: number;
  created: string;
  createdAt?: string;
  completedAt?: string | null;
  reason: string;
  status: string;
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
  type: 'number' | 'bool' | 'enum' | 'string';
  group: string;
  min?: number;
  max?: number;
  step?: number;
  options?: string[];
  hint: string;
  restartRequired: boolean;
}

import type {
  ArkMap,
  Backup,
  ConfigField,
  DiscordEvent,
  AlertSetting,
  LogEvent,
  Mod,
  Player,
  ResourceSample,
  TravelRequest
} from '$lib/types';

export const cluster = {
  name: 'Marcel ARK Cluster',
  id: 'ark-prime-7f3a',
  directory: '/srv/ark/cluster',
  managerVersion: 'rust-manager v0.4.2',
  maxTravelServers: 2,
  emptyShutdownMins: 10
};

// ---- Resource snapshot (high pressure scenario for demo) ----
export const resources: ResourceSample = {
  source: 'mock',
  ramUsedGb: 26.8,
  ramTotalGb: 32,
  ramAvailableGb: 5.2,
  cpuPct: 71,
  swapUsedGb: 1.9,
  swapTotalGb: 8,
  diskUsedGb: 412,
  diskTotalGb: 930,
  diskFreeGb: 518,
  arkProcMemGb: 23.1,
  load1: 3.4,
  load5: 2.8,
  load15: 2.2,
  managerUptimeSecs: 0,
  systemUptimeSecs: null
};

export const thresholds = {
  ramWarnPct: 70,
  ramPressurePct: 82,
  ramEmergencyPct: 92,
  maxTravel: 2,
  emptyShutdownMins: 10
};

// ---- Players split across maps ----
export const players: Player[] = [
  { name: 'Marcel', level: 104, tribe: 'Apex', connectedMins: 142, map: 'Ragnarok' },
  { name: 'Nyx', level: 98, tribe: 'Apex', connectedMins: 96, map: 'Ragnarok' },
  { name: 'Voidwalker', level: 87, tribe: 'Drakes', connectedMins: 51, map: 'Ragnarok' },
  { name: 'Sable', level: 76, tribe: 'Drakes', connectedMins: 33, map: 'Aberration' },
  { name: 'Korin', level: 112, tribe: 'Apex', connectedMins: 210, map: 'Aberration' }
];

// ---- Maps ----
export const maps: ArkMap[] = [
  {
    id: 'home-island',
    name: 'The Island',
    alias: 'island',
    role: 'Home-capable',
    assignment: 'Home',
    state: 'Resource Standby',
    players: 0,
    maxPlayers: 20,
    ramMb: 0,
    ramEstimateMb: 7200,
    uptimeMins: 0,
    idleMins: 18,
    lastBackup: '2026-06-03 18:42',
    rcon: 'Disconnected',
    systemd: 'inactive (dead)',
    restartRequired: false,
    cpuPct: 0,
    saveSizeMb: 184,
    isHome: true,
    protected: true,
    nextAction: 'Auto-restart when resources recover or Home requested',
    config: {
      systemdUnit: 'ark-server@home-island.service',
      arkMapName: 'TheIsland',
      queryPort: 27015,
      rconPort: 27020,
      gamePort: 7777,
      slotPriority: 0,
      autoShutdownEnabled: false,
      canBeHome: true,
      canAutoStopWhenEmpty: false,
      canEnterStandby: true,
      modList: ['Structures Plus (S+)', 'Awesome SpyGlass!', 'Dino Storage v2', 'Super Structures']
    }
  },
  {
    id: 'travel-rag',
    name: 'Ragnarok',
    alias: 'ragnarok',
    role: 'Travel-capable',
    assignment: 'Travel A',
    state: 'Online',
    players: 3,
    maxPlayers: 20,
    ramMb: 8100,
    ramEstimateMb: 8000,
    uptimeMins: 148,
    idleMins: 0,
    lastBackup: '2026-06-03 20:30',
    rcon: 'Connected',
    systemd: 'active (running)',
    restartRequired: false,
    cpuPct: 38,
    saveSizeMb: 221,
    isHome: false,
    protected: false,
    nextAction: 'Monitoring players — auto-shutdown only when empty',
    config: {
      systemdUnit: 'ark-server@travel-a.service',
      arkMapName: 'Ragnarok',
      queryPort: 27017,
      rconPort: 27022,
      gamePort: 7779,
      slotPriority: 1,
      autoShutdownEnabled: true,
      canBeHome: false,
      canAutoStopWhenEmpty: true,
      canEnterStandby: false,
      modList: ['Structures Plus (S+)', 'Awesome SpyGlass!', 'Dino Storage v2']
    }
  },
  {
    id: 'travel-ab',
    name: 'Aberration',
    alias: 'aberration',
    role: 'Travel-capable',
    assignment: 'Travel B',
    state: 'Online',
    players: 2,
    maxPlayers: 20,
    ramMb: 7600,
    ramEstimateMb: 7800,
    uptimeMins: 64,
    idleMins: 0,
    lastBackup: '2026-06-03 19:58',
    rcon: 'Connected',
    systemd: 'active (running)',
    restartRequired: true,
    cpuPct: 33,
    saveSizeMb: 167,
    isHome: false,
    protected: false,
    nextAction: 'Restart required — GameUserSettings.ini changed',
    config: {
      systemdUnit: 'ark-server@travel-b.service',
      arkMapName: 'Aberration',
      queryPort: 27019,
      rconPort: 27024,
      gamePort: 7781,
      slotPriority: 2,
      autoShutdownEnabled: true,
      canBeHome: false,
      canAutoStopWhenEmpty: true,
      canEnterStandby: false,
      modList: ['Awesome SpyGlass!', 'Dino Storage v2', 'Aberration Ascended Loot']
    }
  },
  {
    id: 'map-extinction',
    name: 'Extinction',
    alias: 'extinction',
    role: 'Travel-capable',
    assignment: 'Unassigned',
    state: 'Offline',
    players: 0,
    maxPlayers: 20,
    ramMb: 0,
    ramEstimateMb: 8200,
    uptimeMins: 0,
    idleMins: 0,
    lastBackup: '2026-06-02 23:10',
    rcon: 'Disconnected',
    systemd: 'inactive (dead)',
    restartRequired: false,
    cpuPct: 0,
    saveSizeMb: 203,
    isHome: false,
    protected: false,
    nextAction: 'Idle — start via !travel extinction',
    config: {
      systemdUnit: 'ark-server@travel-c.service',
      arkMapName: 'Extinction',
      queryPort: 27021,
      rconPort: 27026,
      gamePort: 7783,
      slotPriority: 3,
      autoShutdownEnabled: true,
      canBeHome: false,
      canAutoStopWhenEmpty: true,
      canEnterStandby: false,
      modList: ['Awesome SpyGlass!', 'Dino Storage v2']
    }
  },
  {
    id: 'map-scorched',
    name: 'Scorched Earth',
    alias: 'scorched',
    role: 'Travel-capable',
    assignment: 'Unassigned',
    state: 'Offline',
    players: 0,
    maxPlayers: 20,
    ramMb: 0,
    ramEstimateMb: 6800,
    uptimeMins: 0,
    idleMins: 0,
    lastBackup: '2026-06-01 14:05',
    rcon: 'Disconnected',
    systemd: 'inactive (dead)',
    restartRequired: false,
    cpuPct: 0,
    saveSizeMb: 142,
    isHome: false,
    protected: false,
    nextAction: 'Idle — start via !travel scorched',
    config: {
      systemdUnit: 'ark-server@travel-c.service',
      arkMapName: 'ScorchedEarth_P',
      queryPort: 27021,
      rconPort: 27026,
      gamePort: 7783,
      slotPriority: 4,
      autoShutdownEnabled: true,
      canBeHome: false,
      canAutoStopWhenEmpty: true,
      canEnterStandby: false,
      modList: ['Awesome SpyGlass!']
    }
  },
  {
    id: 'map-fjordur',
    name: 'Fjordur',
    alias: 'fjordur',
    role: 'Home-capable',
    assignment: 'Unassigned',
    state: 'Offline',
    players: 0,
    maxPlayers: 20,
    ramMb: 0,
    ramEstimateMb: 8600,
    uptimeMins: 0,
    idleMins: 0,
    lastBackup: '2026-05-30 09:21',
    rcon: 'Disconnected',
    systemd: 'inactive (dead)',
    restartRequired: false,
    cpuPct: 0,
    saveSizeMb: 256,
    isHome: false,
    protected: false,
    nextAction: 'Idle — Home-capable alternate',
    config: {
      systemdUnit: 'ark-server@travel-c.service',
      arkMapName: 'Fjordur',
      queryPort: 27021,
      rconPort: 27026,
      gamePort: 7783,
      slotPriority: 5,
      autoShutdownEnabled: true,
      canBeHome: true,
      canAutoStopWhenEmpty: true,
      canEnterStandby: true,
      modList: ['Structures Plus (S+)', 'Awesome SpyGlass!', 'Dino Storage v2']
    }
  }
];

export const homeMap = maps.find((m) => m.isHome)!;
export const travelA = maps.find((m) => m.assignment === 'Travel A') ?? null;
export const travelB = maps.find((m) => m.assignment === 'Travel B') ?? null;

// ---- Active travel request ----
export const activeTravel: TravelRequest = {
  id: 'tr-2201',
  map: 'Extinction',
  requestedBy: 'Sable',
  source: 'In-game chat',
  sourceRaw: '!travel extinction',
  sourceMap: 'Aberration',
  step: 2,
  result: 'Blocked',
  reason: 'Both travel slots have active players. Request queued until a slot frees or Home leaves Resource Standby.',
  at: '2026-06-03 20:48'
};

// ---- Resource governor decision ----
export const governor = {
  decision: 'Home eligible for Resource Standby',
  why: 'Home has 0 players and RAM pressure is high (84%). Travel A (Ragnarok) and Travel B (Aberration) have active players. Home can be saved, backed up, and stopped so travel players keep playing.',
  examples: [
    'Travel A and Travel B have players. New travel requests are blocked.',
    'Home has 0 players and RAM pressure is high. Home is eligible for Resource Standby.',
    'Travel A is empty for 10 minutes. It is eligible for save, backup, and shutdown.',
    'No action needed. Resources are healthy.'
  ],
  policy: {
    neverStopWithPlayers: true,
    homeStandbyEnabled: true,
    homeStopsOnlyWhenEmpty: true,
    preferActivePlayerMaps: true,
    autoRestartHome: true
  }
};

// ---- Recent activity (dashboard short list) ----
export const recentActivity: LogEvent[] = [
  {
    id: 'a1', ts: '20:48', severity: 'warn', source: 'Travel', actor: 'Sable', targetMap: 'Extinction',
    message: 'Travel request blocked because both travel slots had players', detail: 'Source: Aberration RCON chat `!travel extinction`. Queued.'
  },
  {
    id: 'a2', ts: '20:41', severity: 'warn', source: 'Governor', actor: 'resource-governor', targetMap: 'The Island',
    message: 'Home entered Resource Standby — empty and travel maps had active players', detail: 'RAM pressure 84% > 82% threshold. Home saved + backed up before stop.'
  },
  {
    id: 'a3', ts: '20:40', severity: 'success', source: 'Backup', actor: 'rust-manager', targetMap: 'The Island',
    message: 'Backup completed before shutdown', detail: 'save backup, 184 MB, reason: resource standby'
  },
  {
    id: 'a4', ts: '18:12', severity: 'success', source: 'Map', actor: 'Voidwalker', targetMap: 'Ragnarok',
    message: 'Ragnarok started from in-game `!travel ragnarok`', detail: 'Travel Slot A assigned. RCON ready in 41s.'
  },
  {
    id: 'a5', ts: '18:13', severity: 'info', source: 'Discord', actor: 'discord-bot', targetMap: 'Ragnarok',
    message: 'Discord bot announced server readiness', detail: '#ark-status: Ragnarok is ready for transfer.'
  }
];

// ---- Backups ----
export const backups: Backup[] = [
  { id: 'b1', map: 'Aberration', type: 'save', sizeMb: 167, created: '2026-06-03 20:55', reason: 'scheduled', status: 'running', progress: 62 },
  { id: 'b2', map: 'The Island', type: 'save', sizeMb: 184, created: '2026-06-03 20:40', reason: 'resource standby', status: 'success' },
  { id: 'b3', map: 'Ragnarok', type: 'save', sizeMb: 221, created: '2026-06-03 20:30', reason: 'scheduled', status: 'success' },
  { id: 'b4', map: 'Aberration', type: 'config', sizeMb: 1, created: '2026-06-03 19:58', reason: 'pre-config-change', status: 'success' },
  { id: 'b5', map: 'Cluster', type: 'cluster data', sizeMb: 64, created: '2026-06-03 12:00', reason: 'scheduled', status: 'verifying', progress: 88 },
  { id: 'b6', map: 'Extinction', type: 'save', sizeMb: 0, created: '2026-06-02 23:10', reason: 'auto-shutdown', status: 'failed', error: 'rsync exited 23: disk write timeout on /srv/ark/backups' },
  { id: 'b7', map: 'The Island', type: 'mod', sizeMb: 12, created: '2026-06-02 09:30', reason: 'pre-mod-change', status: 'success' }
];

export const backupPolicy = {
  beforeShutdown: true,
  beforeConfigSave: true,
  beforeModChange: true,
  retention: 'Keep 14 daily + 6 weekly, prune older than 60 days'
};

// ---- Mods ----
export const mods: Mod[] = [
  { id: 'm1', name: 'Structures Plus (S+)', workshopId: '731604991', enabled: true, installed: true, loadOrder: 1, lastUpdated: '2026-05-28', sizeMb: 142, usedBy: ['The Island', 'Ragnarok', 'Fjordur'], state: 'active' },
  { id: 'm2', name: 'Awesome SpyGlass!', workshopId: '1404697612', enabled: true, installed: true, loadOrder: 2, lastUpdated: '2026-05-20', sizeMb: 38, usedBy: ['The Island', 'Ragnarok', 'Aberration', 'Extinction'], state: 'active' },
  { id: 'm3', name: 'Dino Storage v2', workshopId: '1609138312', enabled: true, installed: true, loadOrder: 3, lastUpdated: '2026-05-31', sizeMb: 56, usedBy: ['The Island', 'Ragnarok', 'Aberration'], state: 'active' },
  { id: 'm4', name: 'Super Structures', workshopId: '1999447172', enabled: false, installed: true, loadOrder: 0, lastUpdated: '2026-04-12', sizeMb: 121, usedBy: ['The Island'], state: 'disabled' },
  { id: 'm5', name: 'Ultra Stacks', workshopId: '1654255131', enabled: false, installed: true, loadOrder: 0, lastUpdated: '2026-03-02', sizeMb: 8, usedBy: [], state: 'disabled' },
  { id: 'm6', name: 'Aberration Ascended Loot', workshopId: '2901112233', enabled: true, installed: false, loadOrder: 4, lastUpdated: '—', sizeMb: 0, usedBy: ['Aberration'], state: 'downloading', progress: 47 },
  { id: 'm7', name: 'Custom Ragnarok Expansion', workshopId: '3088112299', enabled: true, installed: false, loadOrder: 0, lastUpdated: '2026-05-15', sizeMb: 0, usedBy: ['Ragnarok'], state: 'failed' },
  { id: 'm8', name: 'Legacy Bridge Mod', workshopId: '1230000099', enabled: false, installed: false, loadOrder: 0, lastUpdated: '—', sizeMb: 0, usedBy: [], state: 'missing' }
];

// ---- Config fields (safe form editor) ----
export const configFields: ConfigField[] = [
  { key: 'XPMultiplier', label: 'XP Multiplier', value: 3, type: 'number', group: 'Rates', min: 0.1, max: 100, step: 0.1, hint: 'Global experience gain rate', restartRequired: false },
  { key: 'HarvestAmountMultiplier', label: 'Harvest Multiplier', value: 4, type: 'number', group: 'Rates', min: 0.1, max: 100, step: 0.1, hint: 'Resource gathering yield', restartRequired: false },
  { key: 'TamingSpeedMultiplier', label: 'Taming Speed', value: 8, type: 'number', group: 'Rates', min: 0.1, max: 100, step: 0.1, hint: 'How fast dinos tame', restartRequired: false },
  { key: 'BabyMatureSpeedMultiplier', label: 'Baby Mature Speed', value: 12, type: 'number', group: 'Breeding', min: 0.1, max: 100, step: 0.1, hint: 'Baby growth rate', restartRequired: false },
  { key: 'MatingIntervalMultiplier', label: 'Mating Interval', value: 0.4, type: 'number', group: 'Breeding', min: 0.01, max: 10, step: 0.01, hint: 'Lower = breed more often', restartRequired: false },
  { key: 'EggHatchSpeedMultiplier', label: 'Egg Hatch Speed', value: 10, type: 'number', group: 'Breeding', min: 0.1, max: 100, step: 0.1, hint: 'Incubation speed', restartRequired: false },
  { key: 'PlayerWeightMultiplier', label: 'Player Weight', value: 2, type: 'number', group: 'Stats', min: 0.1, max: 50, step: 0.1, hint: 'Carry capacity for players', restartRequired: true },
  { key: 'DinoWeightMultiplier', label: 'Dino Weight', value: 3, type: 'number', group: 'Stats', min: 0.1, max: 50, step: 0.1, hint: 'Carry capacity for dinos', restartRequired: true },
  { key: 'DifficultyOffset', label: 'Difficulty Offset', value: 1, type: 'number', group: 'World', min: 0, max: 1, step: 0.01, hint: '1.0 = max wild levels', restartRequired: true },
  { key: 'OverrideOfficialDifficulty', label: 'Max Difficulty', value: 5, type: 'number', group: 'World', min: 1, max: 20, step: 0.5, hint: 'Difficulty 5 = level 150 wilds', restartRequired: true },
  { key: 'ServerPVE', label: 'PvE Mode', value: true, type: 'bool', group: 'World', hint: 'On = PvE, Off = PvP', restartRequired: true },
  { key: 'bAllowFlyerCarryPvE', label: 'Allow Flyer Carry', value: true, type: 'bool', group: 'World', hint: 'Flyers can pick up dinos in PvE', restartRequired: true },
  { key: 'StructurePickupTimeAfterPlacement', label: 'Structure Pickup Timer (s)', value: 1800, type: 'number', group: 'Structures', min: 0, max: 99999, step: 30, hint: 'Quick-pickup window', restartRequired: false },
  { key: 'ItemStackSizeMultiplier', label: 'Stack Size Multiplier', value: 5, type: 'number', group: 'Items', min: 1, max: 100, step: 1, hint: 'Inventory stack scaling', restartRequired: true },
  { key: 'SPlusCraftingSpeed', label: 'S+ Crafting Speed', value: 4, type: 'number', group: 'S+', min: 0.1, max: 50, step: 0.1, hint: 'Structures Plus station crafting speed', restartRequired: false },
  { key: 'SPlusForgeSpeed', label: 'S+ Forge Speed', value: 3, type: 'number', group: 'S+', min: 0.1, max: 50, step: 0.1, hint: 'S+ forge smelt rate', restartRequired: false },
  { key: 'SPlusStorageBoost', label: 'S+ Storage Boost', value: 2, type: 'number', group: 'S+', min: 1, max: 20, step: 1, hint: 'S+ storage capacity multiplier', restartRequired: false }
];

export const rawGameIni = `[/script/shootergame.shootergamemode]
HarvestAmountMultiplier=4.0
TamingSpeedMultiplier=8.0
MatingIntervalMultiplier=0.4
BabyMatureSpeedMultiplier=12.0
EggHatchSpeedMultiplier=10.0
PlayerWeightMultiplier=2.0
DinoCharacterFoodDrainMultiplier=0.8
bAllowFlyerCarryPvE=True
ConfigOverrideItemMaxQuantity=(ItemClassString="PrimalItemResource_Wood_C",Quantity=(MaxItemQuantity=500,bIgnoreMultiplier=true))`;

export const rawGusIni = `[ServerSettings]
ServerPVE=True
XPMultiplier=3.0
DifficultyOffset=1.0
OverrideOfficialDifficulty=5.0
AllowThirdPersonPlayer=True
ShowMapPlayerLocation=True
ServerCrosshair=True
RCONEnabled=True
RCONPort=27020
ItemStackSizeMultiplier=5.0

[/script/engine.gamesession]
MaxPlayers=20`;

// ---- Discord ----
export const discord = {
  online: true,
  guild: 'Apex Survivors',
  statusChannel: '#ark-status',
  lastHeartbeat: '2026-06-03 20:55:12',
  permissionsOk: true
};

export const discordCommands = [
  { cmd: '/status', desc: 'Cluster + map overview', access: 'Everyone' },
  { cmd: '/maps', desc: 'List configured maps and state', access: 'Everyone' },
  { cmd: '/players', desc: 'Who is online and where', access: 'Everyone' },
  { cmd: '/travel <map>', desc: 'Request a travel server', access: 'Everyone' },
  { cmd: '/mods', desc: 'Show active mod load order', access: 'Everyone' },
  { cmd: '/config', desc: 'View config summary', access: 'Admin' },
  { cmd: '/restart <map>', desc: 'Restart a map', access: 'Admin' },
  { cmd: '/broadcast <msg>', desc: 'Send in-game broadcast', access: 'Admin' }
];

export const discordEvents: DiscordEvent[] = [
  { id: 'd1', ts: '20:48', kind: 'travel', text: 'Sable requested /travel extinction — blocked (slots full)' },
  { id: 'd2', ts: '20:41', kind: 'alert', text: 'Resource warning sent: RAM pressure 84%' },
  { id: 'd3', ts: '20:40', kind: 'alert', text: 'Backup completed alert: The Island (resource standby)' },
  { id: 'd4', ts: '18:13', kind: 'status', text: 'Ragnarok readiness announced in #ark-status' },
  { id: 'd5', ts: '17:02', kind: 'status', text: 'Korin ran /status' }
];

export const alertSettings: AlertSetting[] = [
  { key: 'started', label: 'Server started', enabled: true },
  { key: 'stopped', label: 'Server stopped', enabled: true },
  { key: 'pressure', label: 'Resource pressure', enabled: true },
  { key: 'backupFailed', label: 'Backup failed', enabled: true },
  { key: 'modUpdate', label: 'Mod update needed', enabled: false },
  { key: 'rcon', label: 'RCON disconnected', enabled: true }
];

// ---- Full activity log ----
export const activityLog: LogEvent[] = [
  { id: 'l1', ts: '2026-06-03 20:48:11', severity: 'warn', source: 'Travel', actor: 'Sable', targetMap: 'Extinction', message: 'Travel request denied because both travel slots had active players', detail: 'Slots: Travel A=Ragnarok (3 players), Travel B=Aberration (2 players). Request queued.' },
  { id: 'l2', ts: '2026-06-03 20:48:03', severity: 'info', source: 'RCON', actor: 'rust-manager', targetMap: 'Aberration', message: 'Travel request received on Aberration RCON', detail: 'Chat line: Sable: !travel extinction' },
  { id: 'l3', ts: '2026-06-03 20:41:55', severity: 'warn', source: 'Governor', actor: 'resource-governor', targetMap: 'The Island', message: 'Home entered Resource Standby because it was empty and RAM pressure was high', detail: 'RAM 84% > pressure threshold 82%. Home players=0. Travel maps active.' },
  { id: 'l4', ts: '2026-06-03 20:40:20', severity: 'success', source: 'Backup', actor: 'rust-manager', targetMap: 'The Island', message: 'Backup completed for Home before Resource Standby', detail: 'save backup 184 MB → /srv/ark/backups/home-island/2026-06-03T2040.tar.zst' },
  { id: 'l5', ts: '2026-06-03 19:58:40', severity: 'warn', source: 'Config', actor: 'Marcel', targetMap: 'Aberration', message: 'GameUserSettings.ini changed; restart required', detail: 'OverrideOfficialDifficulty 4.0 → 5.0. Pre-config backup taken.' },
  { id: 'l6', ts: '2026-06-03 19:30:02', severity: 'warn', source: 'Mod', actor: 'Marcel', targetMap: 'The Island', message: 'Mod disabled; restart required', detail: 'Super Structures (1999447172) removed from active load order, files kept.' },
  { id: 'l7', ts: '2026-06-03 18:14:09', severity: 'success', source: 'RCON', actor: 'rust-manager', targetMap: 'Travel B', message: 'RCON reconnect succeeded for Travel B', detail: 'Reconnected after 2 retries (4.1s).' },
  { id: 'l8', ts: '2026-06-03 18:13:12', severity: 'info', source: 'Discord', actor: 'discord-bot', targetMap: 'Ragnarok', message: 'Discord user requested Extinction', detail: '/travel extinction by Korin (queued earlier).' },
  { id: 'l9', ts: '2026-06-03 18:12:50', severity: 'success', source: 'Map', actor: 'Voidwalker', targetMap: 'Ragnarok', message: 'Ragnarok started by in-game command from Marcel', detail: 'systemd unit ark-server@travel-a.service started.' },
  { id: 'l10', ts: '2026-06-03 18:12:48', severity: 'info', source: 'Map', actor: 'systemd', targetMap: 'Ragnarok', message: 'systemd unit ark-server@travel-a.service started', detail: 'Active: activating → active (running) in 41s.' },
  { id: 'l11', ts: '2026-06-03 17:02:31', severity: 'info', source: 'User', actor: 'Marcel', targetMap: '—', message: 'Admin opened Config Editor', detail: 'Web UI session via Tailscale 100.84.x.x.' },
  { id: 'l12', ts: '2026-06-02 23:10:18', severity: 'error', source: 'Backup', actor: 'rust-manager', targetMap: 'Extinction', message: 'Backup failed during auto-shutdown', detail: 'rsync exited 23: disk write timeout on /srv/ark/backups.' }
];

// ---- Derived helpers ----
export const ramPct = Math.round((resources.ramUsedGb / resources.ramTotalGb) * 100);
export const cpuPct = resources.cpuPct;
export const swapPct = Math.round((resources.swapUsedGb / resources.swapTotalGb) * 100);
export const diskPct = Math.round((resources.diskUsedGb / resources.diskTotalGb) * 100);
export const runningMaps = maps.filter((m) => m.state === 'Online' || m.state === 'Ready' || m.state === 'Starting').length;
export const totalPlayers = players.length;

export function pressureLevel(): { label: string; tone: 'green' | 'amber' | 'red'; } {
  if (ramPct >= thresholds.ramEmergencyPct) return { label: 'Critical', tone: 'red' };
  if (ramPct >= thresholds.ramPressurePct) return { label: 'Resource Pressure', tone: 'amber' };
  if (ramPct >= thresholds.ramWarnPct) return { label: 'Warning', tone: 'amber' };
  return { label: 'Healthy', tone: 'green' };
}

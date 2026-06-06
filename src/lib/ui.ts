import type { MapState, SystemdStatus, Tone } from '$lib/types';

export const stateTone: Record<MapState, Tone> = {
  Offline: 'gray',
  Starting: 'amber',
  Online: 'green',
  Ready: 'green',
  Draining: 'amber',
  Saving: 'amber',
  'Backing Up': 'amber',
  Stopping: 'amber',
  'Resource Standby': 'gray',
  'Not running': 'gray',
  Error: 'red',
  Unknown: 'gray',
  Unavailable: 'gray'
};

export const rconTone: Record<string, Tone> = {
  Connected: 'cyan',
  Connecting: 'amber',
  Disconnected: 'gray',
  Disabled: 'gray',
  Unavailable: 'gray'
};

export const systemdTone = (s: SystemdStatus): Tone =>
  s === 'active (running)' ? 'green' : s === 'activating' ? 'amber' : s === 'failed' ? 'red' : 'gray';

export const severityTone: Record<string, Tone> = {
  info: 'cyan',
  warn: 'amber',
  error: 'red',
  success: 'green'
};

export function fmtDuration(mins: number): string {
  if (mins <= 0) return '—';
  const h = Math.floor(mins / 60);
  const m = mins % 60;
  if (h === 0) return `${m}m`;
  return `${h}h ${m}m`;
}

export function fmtMb(mb: number): string {
  if (mb <= 0) return '—';
  if (mb >= 1024) return `${(mb / 1024).toFixed(1)} GB`;
  return `${mb} MB`;
}

export function barTone(pct: number, warn = 70, danger = 88): Tone {
  if (pct >= danger) return 'red';
  if (pct >= warn) return 'amber';
  return 'green';
}

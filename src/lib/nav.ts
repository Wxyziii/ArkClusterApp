export interface NavItem {
  href: string;
  label: string;
  icon: string;
  group: string;
}

export const nav: NavItem[] = [
  { href: '/', label: 'Dashboard', icon: '🛰️', group: 'Overview' },
  { href: '/maps', label: 'Maps', icon: '🗺️', group: 'Cluster' },
  { href: '/travel', label: 'Travel', icon: '🧭', group: 'Cluster' },
  { href: '/resources', label: 'Resources', icon: '📊', group: 'Cluster' },
  { href: '/config', label: 'Config Editor', icon: '⚙️', group: 'Management' },
  { href: '/mods', label: 'Mods', icon: '🧩', group: 'Management' },
  { href: '/maintenance', label: 'Maintenance', icon: '🛠️', group: 'Management' },
  { href: '/backups', label: 'Backups', icon: '💾', group: 'Management' },
  { href: '/logs', label: 'Activity / Logs', icon: '📜', group: 'Management' },
  { href: '/discord', label: 'Discord Bot', icon: '💬', group: 'Integrations' },
  { href: '/settings', label: 'Settings', icon: '🔧', group: 'Integrations' }
];

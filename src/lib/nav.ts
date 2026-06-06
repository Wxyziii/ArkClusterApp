export interface NavItem {
  href: string;
  label: string;
  icon: string;
  group: string;
}

export const nav: NavItem[] = [
  { href: '/', label: 'Dashboard', icon: 'D', group: 'Main' },
  { href: '/servers', label: 'Server Manager', icon: 'S', group: 'Main' },
  { href: '/mods', label: 'Mods Manager', icon: 'M', group: 'Main' },
  { href: '/config', label: 'Config Editor', icon: 'C', group: 'Main' },
  { href: '/backups', label: 'Backups/Logs', icon: 'B', group: 'Main' }
];

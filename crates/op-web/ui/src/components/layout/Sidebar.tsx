import { NavLink } from 'react-router-dom';

const navItems = [
  { to: '/', label: 'Dashboard', icon: '📊' },
  { to: '/chat', label: 'Chat', icon: '💬' },
  { to: '/tools', label: 'Tools', icon: '🔧' },
  { to: '/agents', label: 'Agents', icon: '🤖' },
  { to: '/dbus', label: 'D-Bus', icon: '🚌' },
  { to: '/workflows', label: 'Workflows', icon: '⚡' },
  { to: '/state', label: 'State', icon: '📦' },
];

export function Sidebar({ open }: { open: boolean }) {
  if (!open) return null;
  return (
    <aside className="w-56 border-r border-zinc-800 bg-zinc-900 p-4">
      <div className="mb-6 text-xl font-bold text-rose-500">op-dbus</div>
      <nav className="space-y-1">
        {navItems.map((item) => (
          <NavLink
            key={item.to}
            to={item.to}
            className={({ isActive }) =>
              `flex items-center gap-2 rounded px-3 py-2 text-sm ${isActive ? 'bg-zinc-800 text-white' : 'text-zinc-400 hover:bg-zinc-800/50'}`
            }
          >
            <span>{item.icon}</span>
            {item.label}
          </NavLink>
        ))}
      </nav>
    </aside>
  );
}

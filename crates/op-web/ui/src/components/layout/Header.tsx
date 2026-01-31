import { useUiStore, useQuotaStore } from '@/stores';

export function Header() {
  const toggleSidebar = useUiStore((s) => s.toggleSidebar);
  const { used, limit } = useQuotaStore();
  const pct = Math.round((used / limit) * 100);

  return (
    <header className="flex items-center justify-between border-b border-zinc-800 bg-zinc-900 px-4 py-3">
      <button onClick={toggleSidebar} className="text-zinc-400 hover:text-white">☰</button>
      <div className="flex items-center gap-4">
        <div className="text-sm text-zinc-400">
          Quota: <span className={pct > 80 ? 'text-rose-400' : 'text-emerald-400'}>{used}/{limit}</span>
        </div>
        <input
          type="search"
          placeholder="Search..."
          className="rounded bg-zinc-800 px-3 py-1.5 text-sm text-zinc-100 placeholder-zinc-500 outline-none focus:ring-1 focus:ring-rose-500"
        />
      </div>
    </header>
  );
}

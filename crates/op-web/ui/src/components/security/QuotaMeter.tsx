import { useQuotaStore } from '@/stores';

export function QuotaMeter() {
  const { used, limit } = useQuotaStore();
  const pct = Math.min((used / limit) * 100, 100);
  const color = pct > 90 ? 'bg-rose-500' : pct > 70 ? 'bg-amber-500' : 'bg-emerald-500';

  return (
    <div className="w-32">
      <div className="mb-1 flex justify-between text-xs text-zinc-400">
        <span>Quota</span>
        <span>{used}/{limit}</span>
      </div>
      <div className="h-2 rounded-full bg-zinc-800">
        <div className={`h-full rounded-full ${color}`} style={{ width: `${pct}%` }} />
      </div>
    </div>
  );
}

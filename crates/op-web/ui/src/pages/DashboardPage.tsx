import { MetricChart, QuotaMeter } from '@/components';

const mockData = [
  { name: '00:00', value: 12 },
  { name: '04:00', value: 8 },
  { name: '08:00', value: 25 },
  { name: '12:00', value: 42 },
  { name: '16:00', value: 38 },
  { name: '20:00', value: 15 },
];

export function DashboardPage() {
  return (
    <div className="space-y-6">
      <h1 className="text-xl font-semibold">Dashboard</h1>
      <div className="grid grid-cols-3 gap-4">
        <Card title="Tools" value="47" />
        <Card title="Agents" value="12" />
        <Card title="Active Sessions" value="3" />
      </div>
      <div className="grid grid-cols-2 gap-4">
        <div className="rounded-lg bg-zinc-900 p-4">
          <h2 className="mb-4 text-sm font-medium text-zinc-400">Tool Executions (24h)</h2>
          <MetricChart data={mockData} type="area" />
        </div>
        <div className="rounded-lg bg-zinc-900 p-4">
          <h2 className="mb-4 text-sm font-medium text-zinc-400">Quota Usage</h2>
          <QuotaMeter />
        </div>
      </div>
    </div>
  );
}

function Card({ title, value }: { title: string; value: string }) {
  return (
    <div className="rounded-lg bg-zinc-900 p-4">
      <div className="text-sm text-zinc-400">{title}</div>
      <div className="text-2xl font-bold">{value}</div>
    </div>
  );
}

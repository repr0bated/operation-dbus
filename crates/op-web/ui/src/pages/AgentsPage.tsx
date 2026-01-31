import { useState } from 'react';
import { SearchInput, DataTable } from '@/components';

interface Agent {
  id: string;
  name: string;
  category: string;
  status: 'running' | 'stopped';
}

const mockAgents: Agent[] = [
  { id: '1', name: 'rust-pro', category: 'language', status: 'running' },
  { id: '2', name: 'python-pro', category: 'language', status: 'running' },
  { id: '3', name: 'code-reviewer', category: 'analysis', status: 'stopped' },
  { id: '4', name: 'memory', category: 'orchestration', status: 'running' },
];

export function AgentsPage() {
  const [search, setSearch] = useState('');
  const agents = mockAgents.filter((a) => a.name.includes(search) || a.category.includes(search));

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h1 className="text-xl font-semibold">Agents</h1>
        <div className="w-64">
          <SearchInput onSearch={setSearch} placeholder="Filter agents..." />
        </div>
      </div>
      <DataTable
        data={agents}
        columns={[
          { key: 'name', header: 'Name', render: (a: Agent) => <span className="font-mono text-rose-400">{a.name}</span> },
          { key: 'category', header: 'Category' },
          { key: 'status', header: 'Status', render: (a: Agent) => (
            <span className={a.status === 'running' ? 'text-emerald-400' : 'text-zinc-500'}>{a.status}</span>
          )},
        ]}
      />
    </div>
  );
}

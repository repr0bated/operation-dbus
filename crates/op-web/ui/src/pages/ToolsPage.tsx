import { useQuery } from '@tanstack/react-query';
import { api } from '@/api';
import { DataTable, LoadingSpinner, SearchInput } from '@/components';
import { useState } from 'react';

interface Tool {
  name: string;
  description: string;
}

export function ToolsPage() {
  const [search, setSearch] = useState('');
  const { data, isLoading } = useQuery({ queryKey: ['tools'], queryFn: api.tools.list });

  const tools = (data?.tools as Tool[] ?? []).filter((t) => 
    t.name.toLowerCase().includes(search.toLowerCase()) ||
    t.description.toLowerCase().includes(search.toLowerCase())
  );

  if (isLoading) return <div className="flex justify-center p-8"><LoadingSpinner /></div>;

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h1 className="text-xl font-semibold">Tools</h1>
        <div className="w-64">
          <SearchInput onSearch={setSearch} placeholder="Filter tools..." />
        </div>
      </div>
      <DataTable
        data={tools}
        columns={[
          { key: 'name', header: 'Name', render: (t: Tool) => <span className="font-mono text-rose-400">{t.name}</span> },
          { key: 'description', header: 'Description' },
        ]}
      />
    </div>
  );
}

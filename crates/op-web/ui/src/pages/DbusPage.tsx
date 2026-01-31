import { useState } from 'react';
import { SearchInput } from '@/components';

interface DbusService {
  name: string;
  bus: 'system' | 'session';
}

const mockServices: DbusService[] = [
  { name: 'org.freedesktop.systemd1', bus: 'system' },
  { name: 'org.freedesktop.NetworkManager', bus: 'system' },
  { name: 'org.freedesktop.DBus', bus: 'system' },
  { name: 'org.bluez', bus: 'system' },
];

export function DbusPage() {
  const [search, setSearch] = useState('');
  const [selected, setSelected] = useState<string>();
  const services = mockServices.filter((s) => s.name.toLowerCase().includes(search.toLowerCase()));

  return (
    <div className="flex h-full gap-4">
      <div className="w-72 space-y-2">
        <SearchInput onSearch={setSearch} placeholder="Search services..." />
        <div className="space-y-1">
          {services.map((s) => (
            <button
              key={s.name}
              onClick={() => setSelected(s.name)}
              className={`w-full rounded px-3 py-2 text-left text-sm ${selected === s.name ? 'bg-zinc-800 text-white' : 'text-zinc-400 hover:bg-zinc-800/50'}`}
            >
              <div className="font-mono text-xs">{s.name}</div>
              <div className="text-xs text-zinc-500">{s.bus}</div>
            </button>
          ))}
        </div>
      </div>
      <div className="flex-1 rounded-lg bg-zinc-900 p-4">
        {selected ? (
          <div>
            <h2 className="mb-4 font-mono text-lg text-rose-400">{selected}</h2>
            <div className="text-sm text-zinc-400">Select an object to inspect...</div>
          </div>
        ) : (
          <div className="text-zinc-500">Select a service to browse</div>
        )}
      </div>
    </div>
  );
}

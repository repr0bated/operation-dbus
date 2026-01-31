import { useState } from 'react';

interface Props {
  suggestions?: string[];
  onSearch: (query: string) => void;
  placeholder?: string;
}

export function SearchInput({ suggestions = [], onSearch, placeholder = 'Search...' }: Props) {
  const [value, setValue] = useState('');
  const [showSuggestions, setShowSuggestions] = useState(false);
  const filtered = suggestions.filter((s) => s.toLowerCase().includes(value.toLowerCase()));

  return (
    <div className="relative">
      <input
        type="search"
        value={value}
        onChange={(e) => { setValue(e.target.value); setShowSuggestions(true); }}
        onKeyDown={(e) => e.key === 'Enter' && onSearch(value)}
        onBlur={() => setTimeout(() => setShowSuggestions(false), 150)}
        placeholder={placeholder}
        className="w-full rounded bg-zinc-800 px-3 py-2 text-sm outline-none focus:ring-1 focus:ring-rose-500"
      />
      {showSuggestions && filtered.length > 0 && (
        <ul className="absolute z-10 mt-1 w-full rounded bg-zinc-800 py-1 shadow-lg">
          {filtered.slice(0, 5).map((s) => (
            <li
              key={s}
              onClick={() => { setValue(s); onSearch(s); setShowSuggestions(false); }}
              className="cursor-pointer px-3 py-1.5 text-sm hover:bg-zinc-700"
            >{s}</li>
          ))}
        </ul>
      )}
    </div>
  );
}

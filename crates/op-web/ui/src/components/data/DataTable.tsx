interface Column<T> {
  key: string;
  header: string;
  render?: (row: T) => React.ReactNode;
}

interface Props<T> {
  data: T[];
  columns: Column<T>[];
  onRowClick?: (row: T) => void;
}

export function DataTable<T>({ data, columns, onRowClick }: Props<T>) {
  return (
    <table className="w-full text-sm">
      <thead>
        <tr className="border-b border-zinc-800 text-left text-zinc-400">
          {columns.map((col) => (
            <th key={col.key} className="px-3 py-2 font-medium">{col.header}</th>
          ))}
        </tr>
      </thead>
      <tbody>
        {data.map((row, i) => (
          <tr
            key={i}
            onClick={() => onRowClick?.(row)}
            className={`border-b border-zinc-800/50 ${onRowClick ? 'cursor-pointer hover:bg-zinc-800/50' : ''}`}
          >
            {columns.map((col) => (
              <td key={col.key} className="px-3 py-2">
                {col.render ? col.render(row) : String((row as Record<string, unknown>)[col.key] ?? '')}
              </td>
            ))}
          </tr>
        ))}
      </tbody>
    </table>
  );
}

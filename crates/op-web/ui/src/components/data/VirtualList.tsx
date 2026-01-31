import { useVirtualizer } from '@tanstack/react-virtual';
import { useRef } from 'react';

interface Props<T> {
  items: T[];
  height: number;
  renderItem: (item: T, index: number) => React.ReactNode;
  estimateSize?: number;
}

export function VirtualList<T>({ items, height, renderItem, estimateSize = 40 }: Props<T>) {
  const parentRef = useRef<HTMLDivElement>(null);
  const virtualizer = useVirtualizer({
    count: items.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => estimateSize,
  });

  return (
    <div ref={parentRef} className="overflow-auto" style={{ height }}>
      <div style={{ height: virtualizer.getTotalSize(), position: 'relative' }}>
        {virtualizer.getVirtualItems().map((row) => (
          <div
            key={row.key}
            style={{ position: 'absolute', top: row.start, width: '100%' }}
          >
            {renderItem(items[row.index], row.index)}
          </div>
        ))}
      </div>
    </div>
  );
}

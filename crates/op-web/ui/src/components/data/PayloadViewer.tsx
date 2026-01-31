import { useState } from 'react';
import { decodePayload, validateJson } from '@/wasm/decoder';
import { ConfirmModal } from '@/components';

interface Props {
  data: string;
  masked?: boolean;
}

export function PayloadViewer({ data, masked = true }: Props) {
  const [showRaw, setShowRaw] = useState(!masked);
  const [showConfirm, setShowConfirm] = useState(false);
  const isValid = validateJson(data);

  const handleUnmask = () => {
    if (masked && !showRaw) {
      setShowConfirm(true);
    } else {
      setShowRaw(!showRaw);
    }
  };

  const formatted = showRaw && isValid ? decodePayload(data) : null;

  return (
    <div className="rounded bg-zinc-900 p-4">
      <div className="mb-2 flex items-center justify-between">
        <span className="text-xs text-zinc-500">{isValid ? 'Valid JSON' : 'Invalid JSON'}</span>
        <button onClick={handleUnmask} className="text-xs text-rose-400 hover:underline">
          {showRaw ? 'Mask' : 'Unmask'}
        </button>
      </div>
      <pre className="max-h-64 overflow-auto text-xs">
        {showRaw ? formatted ?? data : '••••••••••••••••'}
      </pre>
      {showConfirm && (
        <ConfirmModal
          title="Unmask Payload"
          message="This action will be logged to the blockchain audit trail."
          onConfirm={() => { setShowRaw(true); setShowConfirm(false); }}
          onCancel={() => setShowConfirm(false)}
        />
      )}
    </div>
  );
}

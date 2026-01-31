import { useState } from 'react';

interface Props {
  onSend: (message: string) => void;
  disabled?: boolean;
}

export function ChatInput({ onSend, disabled }: Props) {
  const [value, setValue] = useState('');

  const submit = () => {
    if (!value.trim() || disabled) return;
    onSend(value.trim());
    setValue('');
  };

  return (
    <div className="border-t border-zinc-800 p-4">
      <div className="flex gap-2">
        <textarea
          value={value}
          onChange={(e) => setValue(e.target.value)}
          onKeyDown={(e) => e.key === 'Enter' && !e.shiftKey && (e.preventDefault(), submit())}
          placeholder="Type a message..."
          disabled={disabled}
          rows={2}
          className="flex-1 resize-none rounded bg-zinc-800 px-3 py-2 text-sm outline-none focus:ring-1 focus:ring-rose-500 disabled:opacity-50"
        />
        <button
          onClick={submit}
          disabled={disabled || !value.trim()}
          className="rounded bg-rose-600 px-4 py-2 text-sm font-medium hover:bg-rose-500 disabled:opacity-50"
        >Send</button>
      </div>
    </div>
  );
}

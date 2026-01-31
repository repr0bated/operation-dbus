interface Props {
  title: string;
  message: string;
  onConfirm: () => void;
  onCancel: () => void;
  confirmLabel?: string;
}

export function ConfirmModal({ title, message, onConfirm, onCancel, confirmLabel = 'Confirm' }: Props) {
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60">
      <div className="w-full max-w-md rounded-lg bg-zinc-900 p-6">
        <h2 className="mb-2 text-lg font-semibold">{title}</h2>
        <p className="mb-6 text-sm text-zinc-400">{message}</p>
        <div className="flex justify-end gap-3">
          <button onClick={onCancel} className="rounded bg-zinc-700 px-4 py-2 text-sm hover:bg-zinc-600">Cancel</button>
          <button onClick={onConfirm} className="rounded bg-rose-600 px-4 py-2 text-sm hover:bg-rose-500">{confirmLabel}</button>
        </div>
      </div>
    </div>
  );
}

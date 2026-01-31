export function StatePage() {
  return (
    <div className="space-y-4">
      <h1 className="text-xl font-semibold">State Management</h1>
      <div className="grid grid-cols-2 gap-4">
        <div className="rounded-lg bg-zinc-900 p-4">
          <h2 className="mb-2 text-sm font-medium text-zinc-400">Current State</h2>
          <pre className="text-xs text-zinc-500">Loading...</pre>
        </div>
        <div className="rounded-lg bg-zinc-900 p-4">
          <h2 className="mb-2 text-sm font-medium text-zinc-400">Desired State</h2>
          <pre className="text-xs text-zinc-500">Loading...</pre>
        </div>
      </div>
    </div>
  );
}

import { useState } from 'react';
import { Terminal, Play, Pause, Trash2, Filter } from 'lucide-react';
import { ToolLog, ToolCall } from '@/components/dashboard/ToolLog';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { apiGet } from '@/lib/backend';

interface ToolsResponse {
  tools: Array<{ name: string }>;
}

interface JsonRpcToolCallResult {
  content?: Array<{ type: string; text: string }>;
  isError?: boolean;
}

async function callTool(toolName: string, argumentsObj: Record<string, unknown>) {
  const req = {
    jsonrpc: '2.0',
    id: `${Date.now()}`,
    method: 'tools/call',
    params: {
      name: toolName,
      arguments: argumentsObj,
    },
  };

  const res = await fetch('/jsonrpc', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(req),
  });

  if (!res.ok) {
    throw new Error(`HTTP ${res.status}`);
  }

  const data = await res.json();
  if (data.error) {
    throw new Error(data.error.message || `JSON-RPC ${data.error.code}`);
  }

  return data.result as JsonRpcToolCallResult;
}

export default function McpExecution() {
  const [logs, setLogs] = useState<ToolCall[]>([]);
  const [isStreaming, setIsStreaming] = useState(false);
  const [filter, setFilter] = useState('');

  const runSample = async () => {
    setIsStreaming(true);
    try {
      const list = await apiGet<ToolsResponse>('/api/tools');
      const names = (list.tools || []).map((t) => t.name).slice(0, 5);

      for (const name of names) {
        const startedAt = new Date();
        const pendingId = `${startedAt.getTime()}-${name}`;

        setLogs((prev) => [
          {
            id: pendingId,
            toolName: name,
            args: {},
            timestamp: startedAt.toLocaleTimeString(),
            status: 'pending',
          },
          ...prev,
        ]);

        try {
          const result = await callTool(name, {});
          setLogs((prev) =>
            prev.map((l) => l.id === pendingId ? {
              ...l,
              status: result?.isError ? 'failure' : 'success',
              result,
            } : l),
          );
        } catch (err) {
          setLogs((prev) =>
            prev.map((l) => l.id === pendingId ? {
              ...l,
              status: 'failure',
              result: { error: err instanceof Error ? err.message : 'Execution failed' },
            } : l),
          );
        }
      }
    } finally {
      setIsStreaming(false);
    }
  };

  const filteredLogs = filter
    ? logs.filter((log) => log.toolName.toLowerCase().includes(filter.toLowerCase()))
    : logs;

  const clearLogs = () => setLogs([]);
  const successCount = logs.filter((l) => l.status === 'success').length;
  const failureCount = logs.filter((l) => l.status === 'failure').length;

  return (
    <div className="flex flex-col h-full">
      <div className="p-4 border-b border-border bg-card/50">
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center gap-3">
            <Terminal className="h-5 w-5 text-primary" />
            <h1 className="text-lg font-semibold text-foreground">MCP Execution Stream</h1>
            <span className="text-xs font-mono text-muted-foreground bg-muted px-2 py-0.5 rounded">JSON-RPC 2.0</span>
          </div>
          <div className="flex items-center gap-2">
            <Button variant={isStreaming ? 'default' : 'outline'} size="sm" onClick={runSample} disabled={isStreaming}>
              {isStreaming ? (
                <><Pause className="h-4 w-4 mr-1" />Running</>
              ) : (
                <><Play className="h-4 w-4 mr-1" />Run Sample</>
              )}
            </Button>
            <Button variant="outline" size="sm" onClick={clearLogs}>
              <Trash2 className="h-4 w-4 mr-1" />Clear
            </Button>
          </div>
        </div>

        <div className="flex items-center gap-4">
          <div className="flex items-center gap-4 text-sm">
            <div className="flex items-center gap-2"><div className="h-2 w-2 rounded-full bg-success" /><span className="text-muted-foreground">Success: <span className="text-success font-mono">{successCount}</span></span></div>
            <div className="flex items-center gap-2"><div className="h-2 w-2 rounded-full bg-destructive" /><span className="text-muted-foreground">Failed: <span className="text-destructive font-mono">{failureCount}</span></span></div>
            <div className="text-muted-foreground">Total: <span className="font-mono">{logs.length}</span></div>
          </div>

          <div className="flex-1 max-w-xs ml-auto">
            <div className="relative">
              <Filter className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
              <Input placeholder="Filter by tool name..." value={filter} onChange={(e) => setFilter(e.target.value)} className="pl-9 h-8 text-sm" />
            </div>
          </div>
        </div>
      </div>

      <div className="flex-1 overflow-hidden">
        <ToolLog logs={filteredLogs} />
      </div>
    </div>
  );
}

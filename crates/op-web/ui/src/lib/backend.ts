export interface JsonRpcRequest {
  jsonrpc: '2.0';
  id: string | number;
  method: string;
  params?: unknown;
}

export interface JsonRpcResponse<T = unknown> {
  jsonrpc: '2.0';
  id: string | number | null;
  result?: T;
  error?: {
    code: number;
    message: string;
    data?: unknown;
  };
}

export async function jsonRpcCall<T = unknown>(
  method: string,
  params?: unknown,
): Promise<T> {
  const req: JsonRpcRequest = {
    jsonrpc: '2.0',
    id: `${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
    method,
    params,
  };

  const res = await fetch('/jsonrpc', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(req),
  });

  if (!res.ok) {
    throw new Error(`JSON-RPC HTTP ${res.status}`);
  }

  const data = (await res.json()) as JsonRpcResponse<T>;
  if (data.error) {
    throw new Error(data.error.message || `JSON-RPC error ${data.error.code}`);
  }

  return data.result as T;
}

export async function apiGet<T>(path: string): Promise<T> {
  const res = await fetch(path, { headers: { Accept: 'application/json' } });
  if (!res.ok) {
    throw new Error(`GET ${path} failed (${res.status})`);
  }
  return (await res.json()) as T;
}

export async function apiPost<T>(path: string, body: unknown): Promise<T> {
  const res = await fetch(path, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Accept: 'application/json',
    },
    body: JSON.stringify(body),
  });
  if (!res.ok) {
    throw new Error(`POST ${path} failed (${res.status})`);
  }
  return (await res.json()) as T;
}

export function formatUptime(totalSecs: number): string {
  const days = Math.floor(totalSecs / 86400);
  const hours = Math.floor((totalSecs % 86400) / 3600);
  const mins = Math.floor((totalSecs % 3600) / 60);
  return `${days}d ${hours}h ${mins}m`;
}

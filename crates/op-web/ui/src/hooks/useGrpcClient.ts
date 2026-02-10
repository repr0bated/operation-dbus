import { useState, useCallback } from 'react';
import { jsonRpcCall } from '@/lib/backend';

interface GrpcCallOptions {
  service?: string;
  method: string;
  payload?: unknown;
}

interface GrpcResponse<T = unknown> {
  data: T | null;
  error: string | null;
}

export function useGrpcClient() {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const call = useCallback(async <T = unknown>(
    options: GrpcCallOptions,
  ): Promise<GrpcResponse<T>> => {
    setLoading(true);
    setError(null);

    try {
      // op-web exposes MCP/JSON-RPC methods on /jsonrpc
      const result = await jsonRpcCall<T>(options.method, options.payload);
      return { data: result, error: null };
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Unknown error';
      setError(message);
      return { data: null, error: message };
    } finally {
      setLoading(false);
    }
  }, []);

  return { call, loading, error };
}

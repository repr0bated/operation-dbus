// REST client for /api/* endpoints
const BASE = '';

async function request<T>(path: string, options?: RequestInit): Promise<T> {
  const res = await fetch(`${BASE}${path}`, {
    ...options,
    headers: { 'Content-Type': 'application/json', ...options?.headers },
  });
  if (!res.ok) throw new Error(`${res.status}: ${await res.text()}`);
  return res.json();
}

export const api = {
  get: <T>(path: string) => request<T>(path),
  post: <T>(path: string, body: unknown) => 
    request<T>(path, { method: 'POST', body: JSON.stringify(body) }),
  
  // Typed endpoints
  health: () => api.get<{ status: string }>('/api/health'),
  chat: (message: string, sessionId?: string) => 
    api.post<{ message: string; session_id: string }>('/api/chat', { message, session_id: sessionId }),
  tools: {
    list: () => api.get<{ tools: Tool[] }>('/api/tools'),
    get: (name: string) => api.get<Tool>(`/api/tools/${name}`),
    execute: (name: string, input: unknown) => api.post<unknown>(`/api/tools/${name}/execute`, input),
  },
  llm: {
    status: () => api.get<{ provider: string; model: string }>('/api/llm/status'),
    models: () => api.get<{ models: string[] }>('/api/llm/models'),
  },
};

export interface Tool {
  name: string;
  description: string;
  input_schema: unknown;
}

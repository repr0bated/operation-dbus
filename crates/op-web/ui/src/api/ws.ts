// WebSocket client for /ws
type MessageHandler = (data: unknown) => void;

export class WsClient {
  private ws: WebSocket | null = null;
  private handlers = new Map<string, Set<MessageHandler>>();
  private reconnectTimer: number | null = null;

  connect(url = `${location.protocol === 'https:' ? 'wss:' : 'ws:'}//${location.host}/ws`) {
    this.ws = new WebSocket(url);
    this.ws.onmessage = (e) => {
      const msg = JSON.parse(e.data);
      this.handlers.get(msg.type)?.forEach(h => h(msg.data));
      this.handlers.get('*')?.forEach(h => h(msg));
    };
    this.ws.onclose = () => {
      this.reconnectTimer = window.setTimeout(() => this.connect(url), 3000);
    };
  }

  disconnect() {
    if (this.reconnectTimer) clearTimeout(this.reconnectTimer);
    this.ws?.close();
  }

  on(type: string, handler: MessageHandler) {
    if (!this.handlers.has(type)) this.handlers.set(type, new Set());
    this.handlers.get(type)!.add(handler);
    return () => this.handlers.get(type)?.delete(handler);
  }

  send(type: string, data: unknown) {
    this.ws?.send(JSON.stringify({ type, data }));
  }
}

export const ws = new WsClient();

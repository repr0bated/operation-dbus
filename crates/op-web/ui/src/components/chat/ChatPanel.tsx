import { useState, useRef, useEffect } from 'react';
import { ChatMessage, type Message } from './ChatMessage';
import { ChatInput } from './ChatInput';
import { api } from '@/api';

export function ChatPanel() {
  const [messages, setMessages] = useState<Message[]>([]);
  const [loading, setLoading] = useState(false);
  const [sessionId, setSessionId] = useState<string>();
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => { bottomRef.current?.scrollIntoView({ behavior: 'smooth' }); }, [messages]);

  const send = async (content: string) => {
    setMessages((m) => [...m, { role: 'user', content }]);
    setLoading(true);
    try {
      const res = await api.chat(content, sessionId);
      setSessionId(res.session_id);
      setMessages((m) => [...m, { role: 'assistant', content: res.message }]);
    } catch (e) {
      setMessages((m) => [...m, { role: 'assistant', content: `Error: ${e}` }]);
    }
    setLoading(false);
  };

  return (
    <div className="flex h-full flex-col">
      <div className="flex-1 overflow-auto p-4 space-y-4">
        {messages.map((m, i) => <ChatMessage key={i} message={m} />)}
        {loading && <div className="text-zinc-500">Thinking...</div>}
        <div ref={bottomRef} />
      </div>
      <ChatInput onSend={send} disabled={loading} />
    </div>
  );
}

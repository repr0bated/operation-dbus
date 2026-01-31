export interface Message {
  role: 'user' | 'assistant';
  content: string;
}

export function ChatMessage({ message }: { message: Message }) {
  const isUser = message.role === 'user';
  return (
    <div className={`flex ${isUser ? 'justify-end' : 'justify-start'}`}>
      <div className={`max-w-[80%] rounded-lg px-4 py-2 ${isUser ? 'bg-rose-600' : 'bg-zinc-800'}`}>
        <pre className="whitespace-pre-wrap font-sans text-sm">{message.content}</pre>
      </div>
    </div>
  );
}

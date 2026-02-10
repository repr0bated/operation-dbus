import { useState } from "react";
import { Send } from "lucide-react";
import { cn } from "@/lib/utils";

export interface ChatMessage {
  id: string;
  role: "user" | "assistant";
  content: string;
}

interface ChatPanelProps {
  messages: ChatMessage[];
  isProcessing: boolean;
  onSend: (message: string) => void;
}

export function ChatPanel({ messages, isProcessing, onSend }: ChatPanelProps) {
  const [input, setInput] = useState("");

  const handleSend = () => {
    if (input.trim() && !isProcessing) {
      onSend(input.trim());
      setInput("");
    }
  };

  const handleKeyPress = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  return (
    <div className="flex flex-col h-full">
      {/* Chat History */}
      <div className="flex-1 overflow-y-auto p-4 space-y-4">
        {messages.map((msg) => {
          const isUser = msg.role === "user";
          return (
            <div
              key={msg.id}
              className={cn("flex", isUser ? "justify-end" : "justify-start")}
            >
              <div
                className={cn(
                  "max-w-[85%] rounded-lg p-3 text-sm leading-relaxed",
                  isUser
                    ? "bg-primary text-primary-foreground rounded-br-none"
                    : "bg-card text-card-foreground rounded-bl-none border border-border"
                )}
              >
                <div className="flex items-center gap-2 mb-1 opacity-50 text-xs font-mono uppercase">
                  <span
                    className={cn(
                      isUser ? "text-primary-foreground/70" : "text-muted-foreground"
                    )}
                  >
                    {isUser ? "Operator" : "Agent"}
                  </span>
                </div>
                <div className="whitespace-pre-wrap">{msg.content}</div>
              </div>
            </div>
          );
        })}

        {isProcessing && (
          <div className="flex justify-start">
            <div className="bg-card border border-border rounded-lg p-3 rounded-bl-none flex items-center gap-2">
              <div className="w-2 h-2 bg-success rounded-full animate-bounce" />
              <div
                className="w-2 h-2 bg-success rounded-full animate-bounce"
                style={{ animationDelay: "75ms" }}
              />
              <div
                className="w-2 h-2 bg-success rounded-full animate-bounce"
                style={{ animationDelay: "150ms" }}
              />
              <span className="text-xs text-muted-foreground ml-2 font-mono">
                Processing...
              </span>
            </div>
          </div>
        )}
      </div>

      {/* Input Area */}
      <div className="p-4 border-t border-border bg-card/50">
        <div className="relative">
          <input
            type="text"
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyPress={handleKeyPress}
            disabled={isProcessing}
            placeholder="Type a command (e.g., 'list tools')..."
            className="w-full bg-background border border-border text-foreground rounded-md pl-4 pr-12 py-3 text-sm focus:outline-none focus:border-primary focus:ring-1 focus:ring-primary disabled:opacity-50 font-mono transition-all"
          />
          <button
            onClick={handleSend}
            disabled={!input.trim() || isProcessing}
            className="absolute right-2 top-2 p-1.5 text-muted-foreground hover:text-primary disabled:opacity-30 transition-colors"
          >
            <Send className="h-5 w-5" />
          </button>
        </div>
        <div className="mt-2 text-[10px] text-muted-foreground font-mono text-center">
          MCP Enabled • React/TypeScript UI
        </div>
      </div>
    </div>
  );
}

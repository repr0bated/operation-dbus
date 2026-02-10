import { Terminal } from "lucide-react";
import { cn } from "@/lib/utils";

export interface ToolCall {
  id: string;
  toolName: string;
  args: Record<string, unknown>;
  timestamp: string;
  status: "success" | "pending" | "failure";
  result?: unknown;
}

interface ToolLogProps {
  logs: ToolCall[];
}

export function ToolLog({ logs }: ToolLogProps) {
  return (
    <div className="flex flex-col h-full bg-background/20">
      <div className="px-4 py-2 border-b border-border flex justify-between items-center bg-card/50">
        <h3 className="text-xs font-bold text-muted-foreground uppercase tracking-wider flex items-center gap-2">
          <Terminal className="h-3.5 w-3.5 text-success" />
          MCP Execution Stream
        </h3>
        <span className="text-[10px] font-mono text-muted-foreground">
          JSON-RPC 2.0
        </span>
      </div>

      <div className="flex-1 overflow-y-auto p-3 font-mono text-xs space-y-1">
        {logs.length === 0 ? (
          <div className="h-full flex items-center justify-center text-muted-foreground italic">
            Waiting for tool calls...
          </div>
        ) : (
          logs.map((log) => (
            <div
              key={log.id}
              className={cn(
                "border-l-2 pl-3 py-2 hover:bg-card/50",
                log.status === "success" && "border-success",
                log.status === "pending" && "border-warning",
                log.status === "failure" && "border-destructive"
              )}
            >
              <div className="flex items-center gap-3 mb-1">
                <span className="text-muted-foreground">
                  [{log.timestamp}]
                </span>
                <span className="font-bold text-success">{log.toolName}</span>
                <span
                  className={cn(
                    "ml-auto text-[10px] uppercase px-1.5 rounded border",
                    log.status === "success" &&
                      "bg-success/10 text-success border-success/30",
                    log.status === "pending" &&
                      "bg-warning/10 text-warning border-warning/30 animate-pulse",
                    log.status === "failure" &&
                      "bg-destructive/10 text-destructive border-destructive/30"
                  )}
                >
                  {log.status}
                </span>
              </div>
              <div className="text-muted-foreground pl-1">
                <span className="text-muted-foreground/50 select-none">$ </span>
                <span className="text-primary">params: </span>
                {JSON.stringify(log.args)}
              </div>
              {log.result && (
                <div className="text-muted-foreground pl-1 mt-1">
                  <span className="text-muted-foreground/50 select-none">
                    {">"}{" "}
                  </span>
                  <span className="text-success">return: </span>
                  {JSON.stringify(log.result)}
                </div>
              )}
            </div>
          ))
        )}
      </div>
    </div>
  );
}

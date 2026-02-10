import { cn } from "@/lib/utils";

export interface SystemService {
  id: string;
  name: string;
  status: "active" | "error" | "inactive";
  subState?: string;
}

export interface DbusService {
  name: string;
  category: string;
}

export interface SystemDiagnostics {
  hostname: string;
  uptime: { formatted: string };
  load: { oneMin: number; fiveMin: number; fifteenMin: number };
  memory: { formatted: string; percentUsed: number };
  cpuCores: number;
}

export interface ToolDefinition {
  name: string;
  description: string;
  plugin?: string;
}

interface SystemMonitorProps {
  services: SystemService[];
  dbusServices: DbusService[];
  diagnostics: SystemDiagnostics | null;
  tools: ToolDefinition[];
  connectionStatus: "connected" | "connecting" | "disconnected";
}

export function SystemMonitor({
  services,
  dbusServices,
  diagnostics,
  tools,
  connectionStatus,
}: SystemMonitorProps) {
  const activeCount = services.filter((s) => s.status === "active").length;
  const errorCount = services.filter((s) => s.status === "error").length;

  const opDbusServices = dbusServices.filter((s) => s.category === "op-dbus");
  const otherDbusServices = dbusServices
    .filter((s) => s.category !== "op-dbus")
    .slice(0, 10);

  return (
    <div className="space-y-4">
      {/* Diagnostics Header */}
      {diagnostics && (
        <div className="bg-gradient-to-r from-card to-background border border-border rounded-lg p-4">
          <div className="flex items-center justify-between mb-3">
            <div className="flex items-center gap-3">
              <div
                className={cn(
                  "w-3 h-3 rounded-full",
                  connectionStatus === "connected"
                    ? "bg-success shadow-[0_0_10px_hsl(var(--success)/0.5)]"
                    : "bg-destructive animate-pulse"
                )}
              />
              <h2 className="text-lg font-bold text-foreground">
                {diagnostics.hostname}
              </h2>
            </div>
            <div className="text-xs font-mono text-muted-foreground">
              Uptime: {diagnostics.uptime.formatted}
            </div>
          </div>

          <div className="grid grid-cols-4 gap-3">
            <div className="bg-background/50 rounded p-2 border border-border/50">
              <div className="text-[10px] text-muted-foreground uppercase">
                Load
              </div>
              <div className="font-mono text-xs text-foreground">
                {diagnostics.load.oneMin.toFixed(2)} /{" "}
                {diagnostics.load.fiveMin.toFixed(2)} /{" "}
                {diagnostics.load.fifteenMin.toFixed(2)}
              </div>
            </div>
            <div className="bg-background/50 rounded p-2 border border-border/50">
              <div className="text-[10px] text-muted-foreground uppercase">
                Memory
              </div>
              <div className="font-mono text-xs text-foreground">
                {diagnostics.memory.formatted}
              </div>
              <div className="mt-1 h-1 bg-muted rounded overflow-hidden">
                <div
                  className="h-full bg-success"
                  style={{ width: `${diagnostics.memory.percentUsed}%` }}
                />
              </div>
            </div>
            <div className="bg-background/50 rounded p-2 border border-border/50">
              <div className="text-[10px] text-muted-foreground uppercase">
                CPU Cores
              </div>
              <div className="font-mono text-xl text-foreground">
                {diagnostics.cpuCores}
              </div>
            </div>
            <div className="bg-background/50 rounded p-2 border border-border/50">
              <div className="text-[10px] text-muted-foreground uppercase">
                Services
              </div>
              <div className="flex items-center gap-1">
                <span className="font-mono text-success">{activeCount}</span>
                <span className="text-muted-foreground">/</span>
                <span className="font-mono text-muted-foreground">
                  {services.length}
                </span>
                {errorCount > 0 && (
                  <span className="ml-2 text-[10px] bg-destructive/20 text-destructive px-1 rounded">
                    {errorCount} failed
                  </span>
                )}
              </div>
            </div>
          </div>
        </div>
      )}

      <div className="grid grid-cols-2 gap-4">
        {/* Services Panel */}
        <div className="bg-card border border-border rounded-lg overflow-hidden">
          <div className="px-3 py-2 border-b border-border bg-background/50 flex justify-between items-center">
            <h3 className="text-xs font-semibold text-foreground">
              Systemd Services
            </h3>
            <span className="text-[10px] font-mono text-muted-foreground">
              {services.length} units
            </span>
          </div>
          <div className="p-2 space-y-1 max-h-40 overflow-y-auto">
            {services.slice(0, 15).map((svc) => (
              <div
                key={svc.id}
                className="flex items-center justify-between p-1.5 rounded bg-background/50 border border-border/50 hover:border-border"
              >
                <div className="flex items-center gap-2">
                  <div
                    className={cn(
                      "w-1.5 h-1.5 rounded-full",
                      svc.status === "active" && "bg-success",
                      svc.status === "error" && "bg-destructive",
                      svc.status === "inactive" && "bg-muted-foreground"
                    )}
                  />
                  <span className="text-[10px] text-foreground truncate max-w-[120px]">
                    {svc.name}
                  </span>
                </div>
                <span className="text-[9px] text-muted-foreground">
                  {svc.subState}
                </span>
              </div>
            ))}
          </div>
        </div>

        {/* D-Bus Panel */}
        <div className="bg-card border border-border rounded-lg overflow-hidden">
          <div className="px-3 py-2 border-b border-border bg-background/50 flex justify-between items-center">
            <h3 className="text-xs font-semibold text-foreground">
              D-Bus Services
            </h3>
            <span className="text-[10px] font-mono text-muted-foreground">
              {dbusServices.length} names
            </span>
          </div>
          <div className="p-2 space-y-1 max-h-40 overflow-y-auto">
            {opDbusServices.map((svc) => (
              <div
                key={svc.name}
                className="flex items-center justify-between p-1.5 rounded bg-success/10 border border-success/20"
              >
                <div className="flex items-center gap-2">
                  <div className="w-1.5 h-1.5 rounded-full bg-success" />
                  <span className="text-[10px] font-mono text-success truncate max-w-[150px]">
                    {svc.name}
                  </span>
                </div>
                <span className="text-[9px] text-success/70">OP-DBUS</span>
              </div>
            ))}
            {otherDbusServices.map((svc) => (
              <div
                key={svc.name}
                className="flex items-center justify-between p-1.5 rounded bg-background/50 border border-border/50"
              >
                <span className="text-[10px] font-mono text-muted-foreground truncate max-w-[150px]">
                  {svc.name}
                </span>
                <span className="text-[9px] text-muted-foreground">
                  {svc.category}
                </span>
              </div>
            ))}
          </div>
        </div>
      </div>

      {/* Tools */}
      <div className="bg-card border border-border rounded-lg overflow-hidden">
        <div className="px-3 py-2 border-b border-border bg-background/50 flex justify-between items-center">
          <h3 className="text-xs font-semibold text-foreground">
            Available Tools
          </h3>
          <span className="text-[10px] font-mono text-muted-foreground">
            {tools.length} registered
          </span>
        </div>
        <div className="p-2 flex flex-wrap gap-1 max-h-24 overflow-y-auto">
          {tools.map((tool) => (
            <span
              key={tool.name}
              className="text-[10px] font-mono text-accent-foreground bg-accent/30 px-1.5 py-0.5 rounded border border-accent/50"
            >
              {tool.name}
            </span>
          ))}
        </div>
      </div>
    </div>
  );
}

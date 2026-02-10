import { 
  Wrench, 
  Bot, 
  Users, 
  Server, 
  Cpu, 
  HardDrive, 
  Activity,
  Clock,
  Network,
  Shield,
  Workflow,
  Database
} from "lucide-react";

export interface SystemMetrics {
  // System
  hostname: string;
  kernel: string;
  uptime: string;
  loadAverage: [number, number, number];
  memoryUsedPercent: number;
  memoryFormatted: string;
  cpuCores: number;
  cpuUsage: number;
  
  // Tools (D-Bus objects = tools)
  totalTools: number;
  toolsByCategory: Record<string, number>;
  
  // Agents
  agentTypesAvailable: number;
  agentInstancesRunning: number;
  
  // LLM
  llmProvider: string;
  llmModel: string;
  llmAvailable: boolean;
  
  // Network
  networkInterfaces: number;
  interfacesUp: number;
  
  // Services
  servicesActive: number;
  servicesTotal: number;
  
  // Connections
  connectedUsers: number;
  activeSessions: number;
}

interface MetricsOverviewProps {
  metrics: SystemMetrics;
}

export function MetricsOverview({ metrics }: MetricsOverviewProps) {
  // Calculate tool distribution
  const dbusTools = metrics.toolsByCategory?.dbus || 0;
  const systemdTools = metrics.toolsByCategory?.systemd || 0;
  const ovsTools = metrics.toolsByCategory?.ovs || 0;
  const fileTools = metrics.toolsByCategory?.file || 0;
  const otherTools = metrics.totalTools - dbusTools - systemdTools - ovsTools - fileTools;

  return (
    <div className="space-y-4">
      {/* Primary Metrics Row */}
      <div className="grid grid-cols-4 gap-3">
        <MetricCard
          icon={<Wrench className="h-5 w-5" />}
          label="Total Tools"
          value={metrics.totalTools.toLocaleString()}
          subtext={`${Object.keys(metrics.toolsByCategory).length} categories`}
          color="cyan"
          glow
        />
        <MetricCard
          icon={<Bot className="h-5 w-5" />}
          label="Agents"
          value={metrics.agentInstancesRunning.toString()}
          subtext={`${metrics.agentTypesAvailable} types available`}
          color="purple"
          glow
        />
        <MetricCard
          icon={<Users className="h-5 w-5" />}
          label="Connected Users"
          value={metrics.connectedUsers.toString()}
          subtext={`${metrics.activeSessions} active sessions`}
          color="emerald"
        />
        <MetricCard
          icon={<Server className="h-5 w-5" />}
          label="Services"
          value={`${metrics.servicesActive}/${metrics.servicesTotal}`}
          subtext="running"
          color="amber"
          status={metrics.servicesActive === metrics.servicesTotal ? "healthy" : "warning"}
        />
      </div>

      {/* System Health Row */}
      <div className="grid grid-cols-5 gap-3">
        <div className="col-span-2 bg-gradient-to-br from-card to-card/80 border border-border rounded-lg p-3">
          <div className="flex items-center gap-2 mb-2">
            <Activity className="h-4 w-4 text-primary" />
            <span className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
              System Load
            </span>
          </div>
          <div className="flex items-baseline gap-4">
            <div className="text-2xl font-bold text-foreground">
              {metrics.loadAverage[0].toFixed(2)}
            </div>
            <div className="text-sm text-muted-foreground">
              {metrics.loadAverage[1].toFixed(2)} / {metrics.loadAverage[2].toFixed(2)}
            </div>
          </div>
          <div className="mt-2 text-xs text-muted-foreground">
            {metrics.hostname} • {metrics.cpuCores} cores
          </div>
        </div>

        <ResourceMeter
          icon={<Cpu className="h-4 w-4" />}
          label="CPU"
          value={metrics.cpuUsage}
          format={(v) => `${v.toFixed(0)}%`}
        />

        <ResourceMeter
          icon={<HardDrive className="h-4 w-4" />}
          label="Memory"
          value={metrics.memoryUsedPercent}
          format={() => metrics.memoryFormatted}
        />

        <div className="bg-gradient-to-br from-card to-card/80 border border-border rounded-lg p-3">
          <div className="flex items-center gap-2 mb-2">
            <Clock className="h-4 w-4 text-muted-foreground" />
            <span className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
              Uptime
            </span>
          </div>
          <div className="text-lg font-bold text-foreground">{metrics.uptime}</div>
          <div className="mt-1 text-xs text-muted-foreground truncate" title={metrics.kernel}>
            {metrics.kernel}
          </div>
        </div>
      </div>

      {/* Tool Distribution */}
      <div className="bg-gradient-to-br from-card to-card/80 border border-border rounded-lg p-3">
        <div className="flex items-center justify-between mb-3">
          <div className="flex items-center gap-2">
            <Database className="h-4 w-4 text-primary" />
            <span className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
              Tool Distribution (D-Bus Objects)
            </span>
          </div>
          <div className="text-xs text-muted-foreground">
            {metrics.totalTools.toLocaleString()} total registered
          </div>
        </div>
        <div className="flex gap-2">
          <ToolCategory label="D-Bus" count={dbusTools} color="bg-cyan-500" />
          <ToolCategory label="Systemd" count={systemdTools} color="bg-purple-500" />
          <ToolCategory label="OVS" count={ovsTools} color="bg-amber-500" />
          <ToolCategory label="File" count={fileTools} color="bg-emerald-500" />
          <ToolCategory label="Other" count={otherTools} color="bg-zinc-500" />
        </div>
      </div>

      {/* LLM + Network Status */}
      <div className="grid grid-cols-2 gap-3">
        <div className="bg-gradient-to-br from-card to-card/80 border border-border rounded-lg p-3">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <Workflow className="h-4 w-4 text-primary" />
              <span className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
                LLM Engine
              </span>
            </div>
            <div className={`h-2 w-2 rounded-full ${metrics.llmAvailable ? 'bg-success animate-pulse' : 'bg-destructive'}`} />
          </div>
          <div className="mt-2 font-mono text-sm text-foreground">{metrics.llmProvider}</div>
          <div className="text-xs text-muted-foreground truncate">{metrics.llmModel}</div>
        </div>

        <div className="bg-gradient-to-br from-card to-card/80 border border-border rounded-lg p-3">
          <div className="flex items-center gap-2 mb-2">
            <Network className="h-4 w-4 text-primary" />
            <span className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
              Network
            </span>
          </div>
          <div className="flex items-baseline gap-2">
            <span className="text-lg font-bold text-success">{metrics.interfacesUp}</span>
            <span className="text-muted-foreground">/</span>
            <span className="text-lg font-bold text-foreground">{metrics.networkInterfaces}</span>
            <span className="text-xs text-muted-foreground">interfaces up</span>
          </div>
        </div>
      </div>
    </div>
  );
}

// Sub-components

interface MetricCardProps {
  icon: React.ReactNode;
  label: string;
  value: string;
  subtext: string;
  color: "cyan" | "purple" | "emerald" | "amber";
  glow?: boolean;
  status?: "healthy" | "warning" | "error";
}

function MetricCard({ icon, label, value, subtext, color, glow, status }: MetricCardProps) {
  const colorMap = {
    cyan: "text-cyan-400",
    purple: "text-purple-400",
    emerald: "text-emerald-400",
    amber: "text-amber-400",
  };
  
  const glowMap = {
    cyan: "shadow-[0_0_20px_rgba(6,182,212,0.15)]",
    purple: "shadow-[0_0_20px_rgba(168,85,247,0.15)]",
    emerald: "shadow-[0_0_20px_rgba(16,185,129,0.15)]",
    amber: "shadow-[0_0_20px_rgba(245,158,11,0.15)]",
  };

  return (
    <div className={`
      bg-gradient-to-br from-card to-card/80 border border-border rounded-lg p-3
      transition-all duration-200 hover:border-primary/50
      ${glow ? glowMap[color] : ""}
    `}>
      <div className="flex items-center gap-2 mb-2">
        <div className={colorMap[color]}>{icon}</div>
        <span className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
          {label}
        </span>
        {status && (
          <div className={`ml-auto h-2 w-2 rounded-full ${
            status === "healthy" ? "bg-success" : 
            status === "warning" ? "bg-warning" : "bg-destructive"
          }`} />
        )}
      </div>
      <div className={`text-2xl font-bold ${colorMap[color]}`}>{value}</div>
      <div className="text-xs text-muted-foreground mt-1">{subtext}</div>
    </div>
  );
}

interface ResourceMeterProps {
  icon: React.ReactNode;
  label: string;
  value: number;
  format: (value: number) => string;
}

function ResourceMeter({ icon, label, value, format }: ResourceMeterProps) {
  const getColor = (v: number) => {
    if (v < 50) return "bg-success";
    if (v < 80) return "bg-warning";
    return "bg-destructive";
  };

  return (
    <div className="bg-gradient-to-br from-card to-card/80 border border-border rounded-lg p-3">
      <div className="flex items-center gap-2 mb-2">
        <div className="text-muted-foreground">{icon}</div>
        <span className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
          {label}
        </span>
      </div>
      <div className="text-lg font-bold text-foreground">{format(value)}</div>
      <div className="mt-2 h-1.5 bg-muted rounded-full overflow-hidden">
        <div
          className={`h-full ${getColor(value)} transition-all duration-500`}
          style={{ width: `${Math.min(100, value)}%` }}
        />
      </div>
    </div>
  );
}

interface ToolCategoryProps {
  label: string;
  count: number;
  color: string;
}

function ToolCategory({ label, count, color }: ToolCategoryProps) {
  return (
    <div className="flex-1 bg-muted/30 rounded p-2 text-center">
      <div className="flex items-center justify-center gap-1.5 mb-1">
        <div className={`h-2 w-2 rounded-full ${color}`} />
        <span className="text-xs text-muted-foreground">{label}</span>
      </div>
      <div className="text-sm font-bold text-foreground">{count.toLocaleString()}</div>
    </div>
  );
}

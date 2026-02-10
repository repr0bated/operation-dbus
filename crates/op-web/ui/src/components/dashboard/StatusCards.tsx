import { Activity, Server, Cpu, HardDrive, Clock, Users } from 'lucide-react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { SystemStatus } from '@/types/opdbus';

interface StatusCardsProps {
  status: SystemStatus;
}

export function StatusCards({ status }: StatusCardsProps) {
  const formatUptime = (seconds: number) => {
    const days = Math.floor(seconds / 86400);
    const hours = Math.floor((seconds % 86400) / 3600);
    const mins = Math.floor((seconds % 3600) / 60);
    return `${days}d ${hours}h ${mins}m`;
  };

  const cardClass = "bg-gradient-to-br from-card to-[hsl(217,33%,14%)] border-border rounded-[14px] transition-all duration-200 hover:border-primary hover:-translate-y-[3px] hover:shadow-[0_12px_24px_rgba(99,102,241,0.12)]";

  return (
    <div className="grid gap-6 md:grid-cols-2 lg:grid-cols-4">
      <Card className={cardClass}>
        <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-3">
          <CardTitle className="text-sm font-medium text-muted-foreground uppercase tracking-wide">Total Services</CardTitle>
          <Server className="h-4 w-4 text-muted-foreground" />
        </CardHeader>
        <CardContent>
          <div className="text-[2.8rem] font-bold text-primary leading-none">{status.totalServices}</div>
          <p className="text-sm text-muted-foreground mt-2">
            {status.activeServices} active, {status.errorServices} errors
          </p>
        </CardContent>
      </Card>

      <Card className={cardClass}>
        <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-3">
          <CardTitle className="text-sm font-medium text-muted-foreground uppercase tracking-wide">Active Services</CardTitle>
          <Activity className="h-4 w-4 text-success" />
        </CardHeader>
        <CardContent>
          <div className="text-[2.8rem] font-bold text-success leading-none">{status.activeServices}</div>
          <p className="text-sm text-muted-foreground mt-2">
            {((status.activeServices / status.totalServices) * 100).toFixed(0)}% healthy
          </p>
        </CardContent>
      </Card>

      <Card className={cardClass}>
        <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-3">
          <CardTitle className="text-sm font-medium text-muted-foreground uppercase tracking-wide">Resource Usage</CardTitle>
          <Cpu className="h-4 w-4 text-muted-foreground" />
        </CardHeader>
        <CardContent>
          <div className="text-[2.8rem] font-bold text-primary leading-none">{status.totalCpuPercent.toFixed(1)}%</div>
          <p className="text-sm text-muted-foreground mt-2 flex items-center gap-1">
            <HardDrive className="h-3 w-3" />
            {status.totalMemoryMb.toFixed(1)} MB RAM
          </p>
        </CardContent>
      </Card>

      <Card className={cardClass}>
        <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-3">
          <CardTitle className="text-sm font-medium text-muted-foreground uppercase tracking-wide">System Status</CardTitle>
          <Clock className="h-4 w-4 text-muted-foreground" />
        </CardHeader>
        <CardContent>
          <div className="text-[2.8rem] font-bold text-primary leading-none">{formatUptime(status.uptime)}</div>
          <p className="text-sm text-muted-foreground mt-2 flex items-center gap-1">
            <Users className="h-3 w-3" />
            {status.wireguardPeers} WireGuard peers
          </p>
        </CardContent>
      </Card>
    </div>
  );
}

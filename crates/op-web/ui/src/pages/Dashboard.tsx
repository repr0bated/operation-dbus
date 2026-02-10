import { useServices } from '@/hooks/useServices';
import { StatusCards } from '@/components/dashboard/StatusCards';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';

export default function Dashboard() {
  const { services, getSystemStatus } = useServices();
  const status = getSystemStatus();

  return (
    <div className="p-8 max-w-[1600px]">
      <div className="mb-8">
        <h1 className="text-[2.2rem] font-bold mb-2">System Dashboard</h1>
        <p className="text-muted-foreground text-lg">Real-time health and performance metrics</p>
      </div>

      <StatusCards status={status} />
      
      <div className="grid gap-6 lg:grid-cols-2 mt-6">
        <Card className="bg-gradient-to-br from-card to-[hsl(217,33%,14%)] border-border rounded-[14px] transition-all duration-200 hover:border-primary hover:-translate-y-[3px] hover:shadow-[0_12px_24px_rgba(99,102,241,0.12)]">
          <CardHeader>
            <CardTitle className="text-primary text-xl font-semibold">Recent Activity</CardTitle>
            <CardDescription className="text-muted-foreground">Latest service events and changes</CardDescription>
          </CardHeader>
          <CardContent>
            <div className="space-y-3">
              {services.slice(0, 4).map((service, idx) => (
                <div key={idx} className="flex items-center justify-between p-3 rounded-lg bg-secondary/50">
                  <div className="flex items-center gap-3">
                    <div className={`w-2 h-2 rounded-full ${
                      service.status === 'active' ? 'bg-success' :
                      service.status === 'error' ? 'bg-destructive' : 'bg-muted-foreground'
                    }`} />
                    <span className="font-mono text-sm">{service.busName}</span>
                  </div>
                  <span className="text-xs text-muted-foreground">
                    {service.lastSeen.toLocaleTimeString()}
                  </span>
                </div>
              ))}
            </div>
          </CardContent>
        </Card>

        <Card className="bg-gradient-to-br from-card to-[hsl(217,33%,14%)] border-border rounded-[14px] transition-all duration-200 hover:border-primary hover:-translate-y-[3px] hover:shadow-[0_12px_24px_rgba(99,102,241,0.12)]">
          <CardHeader>
            <CardTitle className="text-primary text-xl font-semibold">Quick Stats</CardTitle>
            <CardDescription className="text-muted-foreground">System resource overview</CardDescription>
          </CardHeader>
          <CardContent>
            <div className="space-y-4">
              <div>
                <div className="flex justify-between text-sm mb-1">
                  <span>CPU Usage</span>
                  <span>{status.totalCpuPercent.toFixed(1)}%</span>
                </div>
                <div className="h-2 bg-secondary rounded-full overflow-hidden">
                  <div 
                    className="h-full bg-primary transition-all"
                    style={{ width: `${Math.min(status.totalCpuPercent * 10, 100)}%` }}
                  />
                </div>
              </div>
              <div>
                <div className="flex justify-between text-sm mb-1">
                  <span>Memory Usage</span>
                  <span>{status.totalMemoryMb.toFixed(0)} MB</span>
                </div>
                <div className="h-2 bg-secondary rounded-full overflow-hidden">
                  <div 
                    className="h-full bg-primary transition-all"
                    style={{ width: `${Math.min((status.totalMemoryMb / 500) * 100, 100)}%` }}
                  />
                </div>
              </div>
              <div>
                <div className="flex justify-between text-sm mb-1">
                  <span>Service Health</span>
                  <span>{((status.activeServices / status.totalServices) * 100).toFixed(0)}%</span>
                </div>
                <div className="h-2 bg-secondary rounded-full overflow-hidden">
                  <div 
                    className="h-full bg-success transition-all"
                    style={{ width: `${(status.activeServices / status.totalServices) * 100}%` }}
                  />
                </div>
              </div>
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}

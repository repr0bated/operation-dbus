import { FileText, Search, Download, Link } from 'lucide-react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';

const mockAuditLogs = [
  { id: '1', timestamp: new Date(), operation: 'service.start', service: 'org.freedesktop.NetworkManager', status: 'success' },
  { id: '2', timestamp: new Date(Date.now() - 60000), operation: 'policy.update', service: 'admin-policy', status: 'success' },
  { id: '3', timestamp: new Date(Date.now() - 120000), operation: 'user.login', service: 'auth-service', status: 'success' },
  { id: '4', timestamp: new Date(Date.now() - 180000), operation: 'service.stop', service: 'org.bluez', status: 'failure' },
  { id: '5', timestamp: new Date(Date.now() - 240000), operation: 'config.change', service: 'network-config', status: 'success' },
];

export default function Audit() {
  return (
    <div className="p-8 max-w-[1600px]">
      <div className="mb-8 flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold mb-2">Audit & Blockchain</h1>
          <p className="text-muted-foreground">Immutable change trail and verification</p>
        </div>
        <div className="flex items-center gap-2">
          <Button variant="outline">
            <Download className="h-4 w-4 mr-2" />
            Export Logs
          </Button>
          <Button variant="outline">
            <Link className="h-4 w-4 mr-2" />
            Verify Chain
          </Button>
        </div>
      </div>

      <Card className="mb-6">
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle>Audit Trail</CardTitle>
              <CardDescription>Complete history of system changes</CardDescription>
            </div>
            <div className="relative">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
              <Input placeholder="Search logs..." className="pl-9 w-64" />
            </div>
          </div>
        </CardHeader>
        <CardContent>
          <div className="space-y-2">
            {mockAuditLogs.map((log) => (
              <div key={log.id} className="flex items-center justify-between p-4 rounded-lg bg-muted/50 hover:bg-muted/70 transition-colors">
                <div className="flex items-center gap-4">
                  <FileText className="h-4 w-4 text-muted-foreground" />
                  <div>
                    <span className="font-mono text-sm">{log.operation}</span>
                    <p className="text-xs text-muted-foreground">{log.service}</p>
                  </div>
                </div>
                <div className="flex items-center gap-4">
                  <span className={`text-xs px-2 py-1 rounded-full ${
                    log.status === 'success' 
                      ? 'bg-success/10 text-success' 
                      : 'bg-destructive/10 text-destructive'
                  }`}>
                    {log.status}
                  </span>
                  <span className="text-xs text-muted-foreground">
                    {log.timestamp.toLocaleTimeString()}
                  </span>
                </div>
              </div>
            ))}
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <div className="flex items-center gap-3">
            <Link className="h-5 w-5 text-primary" />
            <CardTitle>Blockchain Verification</CardTitle>
          </div>
          <CardDescription>Cryptographic proof of audit integrity</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="text-center py-8 text-muted-foreground">
            <Link className="h-12 w-12 mx-auto mb-4 opacity-50" />
            <p>Blockchain explorer coming soon</p>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

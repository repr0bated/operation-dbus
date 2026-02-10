import { Network as NetworkIcon, Router, Globe, Lock } from 'lucide-react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';

export default function Network() {
  return (
    <div className="p-8 max-w-[1600px]">
      <div className="mb-8">
        <h1 className="text-3xl font-bold mb-2">Network Management</h1>
        <p className="text-muted-foreground">Interfaces, OVSDB, and network namespaces</p>
      </div>

      <div className="grid gap-6 md:grid-cols-2 lg:grid-cols-4">
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm text-muted-foreground">Interfaces</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-3xl font-bold">8</div>
            <p className="text-xs text-success">6 active</p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm text-muted-foreground">Bridges</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-3xl font-bold">3</div>
            <p className="text-xs text-muted-foreground">OVS managed</p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm text-muted-foreground">Namespaces</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-3xl font-bold">5</div>
            <p className="text-xs text-muted-foreground">Isolated</p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm text-muted-foreground">WireGuard Peers</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-3xl font-bold">12</div>
            <p className="text-xs text-success">All connected</p>
          </CardContent>
        </Card>
      </div>

      <div className="grid gap-6 lg:grid-cols-2 mt-6">
        <Card>
          <CardHeader>
            <div className="flex items-center gap-3">
              <Router className="h-5 w-5 text-primary" />
              <CardTitle>Network Interfaces</CardTitle>
            </div>
            <CardDescription>Physical and virtual interface management</CardDescription>
          </CardHeader>
          <CardContent>
            <div className="space-y-3">
              {['eth0', 'wg0', 'br0', 'veth-ns1'].map((iface) => (
                <div key={iface} className="flex items-center justify-between p-3 rounded-lg bg-muted/50">
                  <div className="flex items-center gap-3">
                    <div className="w-2 h-2 rounded-full bg-success" />
                    <span className="font-mono text-sm">{iface}</span>
                  </div>
                  <span className="text-xs text-muted-foreground">UP</span>
                </div>
              ))}
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <div className="flex items-center gap-3">
              <Lock className="h-5 w-5 text-primary" />
              <CardTitle>WireGuard Sessions</CardTitle>
            </div>
            <CardDescription>VPN tunnel and peer management</CardDescription>
          </CardHeader>
          <CardContent>
            <div className="text-center py-8 text-muted-foreground">
              <Lock className="h-12 w-12 mx-auto mb-4 opacity-50" />
              <p>WireGuard configuration coming soon</p>
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}

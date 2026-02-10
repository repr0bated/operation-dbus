import { Puzzle, Upload, Settings, Check, X } from 'lucide-react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';

const mockPlugins = [
  { name: 'network-manager', version: '1.2.0', status: 'active', description: 'Network configuration plugin' },
  { name: 'auth-ldap', version: '2.1.3', status: 'active', description: 'LDAP authentication backend' },
  { name: 'metrics-collector', version: '0.9.1', status: 'inactive', description: 'System metrics gathering' },
  { name: 'log-aggregator', version: '1.0.0', status: 'active', description: 'Centralized log collection' },
];

export default function Plugins() {
  return (
    <div className="p-8 max-w-[1600px]">
      <div className="mb-8 flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold mb-2">Plugin & Schema Manager</h1>
          <p className="text-muted-foreground">Plugin definitions and schema registry</p>
        </div>
        <Button>
          <Upload className="h-4 w-4 mr-2" />
          Install Plugin
        </Button>
      </div>

      <div className="grid gap-6 md:grid-cols-2 lg:grid-cols-4 mb-6">
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm text-muted-foreground">Installed</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-3xl font-bold">24</div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm text-muted-foreground">Active</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-3xl font-bold text-success">21</div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm text-muted-foreground">Schemas</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-3xl font-bold">156</div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm text-muted-foreground">Updates</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-3xl font-bold text-warning">3</div>
          </CardContent>
        </Card>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Installed Plugins</CardTitle>
          <CardDescription>Manage system plugins and extensions</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="space-y-3">
            {mockPlugins.map((plugin) => (
              <div key={plugin.name} className="flex items-center justify-between p-4 rounded-lg bg-muted/50 hover:bg-muted/70 transition-colors">
                <div className="flex items-center gap-4">
                  <Puzzle className="h-5 w-5 text-primary" />
                  <div>
                    <div className="flex items-center gap-2">
                      <span className="font-mono font-medium">{plugin.name}</span>
                      <span className="text-xs text-muted-foreground">v{plugin.version}</span>
                    </div>
                    <p className="text-sm text-muted-foreground">{plugin.description}</p>
                  </div>
                </div>
                <div className="flex items-center gap-3">
                  <span className={`flex items-center gap-1 text-xs px-2 py-1 rounded-full ${
                    plugin.status === 'active' 
                      ? 'bg-success/10 text-success' 
                      : 'bg-muted text-muted-foreground'
                  }`}>
                    {plugin.status === 'active' ? <Check className="h-3 w-3" /> : <X className="h-3 w-3" />}
                    {plugin.status}
                  </span>
                  <Button variant="ghost" size="icon">
                    <Settings className="h-4 w-4" />
                  </Button>
                </div>
              </div>
            ))}
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

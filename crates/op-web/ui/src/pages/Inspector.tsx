import { Search, Database, ArrowRight, RefreshCw } from 'lucide-react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';

export default function Inspector() {
  return (
    <div className="p-8 max-w-[1600px]">
      <div className="mb-8 flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold mb-2">Inspector Gadget</h1>
          <p className="text-muted-foreground">Schema discovery and migration tools</p>
        </div>
        <Button>
          <RefreshCw className="h-4 w-4 mr-2" />
          Scan Schemas
        </Button>
      </div>

      <Card className="mb-6">
        <CardHeader>
          <CardTitle>Schema Discovery</CardTitle>
          <CardDescription>Explore and analyze database schemas</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="flex items-center gap-4 mb-6">
            <div className="relative flex-1">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
              <Input placeholder="Search tables, columns, or types..." className="pl-9" />
            </div>
            <Button variant="outline">Inspect</Button>
          </div>

          <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
            {['users', 'services', 'policies', 'audit_logs', 'sessions', 'config'].map((table) => (
              <div key={table} className="flex items-center justify-between p-4 rounded-lg bg-muted/50 hover:bg-muted/70 transition-colors cursor-pointer">
                <div className="flex items-center gap-3">
                  <Database className="h-4 w-4 text-primary" />
                  <span className="font-mono text-sm">{table}</span>
                </div>
                <ArrowRight className="h-4 w-4 text-muted-foreground" />
              </div>
            ))}
          </div>
        </CardContent>
      </Card>

      <div className="grid gap-6 lg:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle>Migration Queue</CardTitle>
            <CardDescription>Pending schema migrations</CardDescription>
          </CardHeader>
          <CardContent>
            <div className="text-center py-8 text-muted-foreground">
              <RefreshCw className="h-12 w-12 mx-auto mb-4 opacity-50" />
              <p>No pending migrations</p>
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Recent Changes</CardTitle>
            <CardDescription>Schema modification history</CardDescription>
          </CardHeader>
          <CardContent>
            <div className="space-y-3">
              <div className="flex items-center justify-between p-3 rounded-lg bg-muted/50">
                <span className="text-sm">Added column: users.last_login</span>
                <span className="text-xs text-muted-foreground">2h ago</span>
              </div>
              <div className="flex items-center justify-between p-3 rounded-lg bg-muted/50">
                <span className="text-sm">Created table: audit_logs</span>
                <span className="text-xs text-muted-foreground">1d ago</span>
              </div>
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}

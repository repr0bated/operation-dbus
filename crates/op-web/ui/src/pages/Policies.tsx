import { Shield, Lock, Key, AlertTriangle } from 'lucide-react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';

export default function Policies() {
  return (
    <div className="p-8 max-w-[1600px]">
      <div className="mb-8 flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold mb-2">Policy Manager</h1>
          <p className="text-muted-foreground">RBAC and security profiles</p>
        </div>
        <Button>
          <Shield className="h-4 w-4 mr-2" />
          Create Policy
        </Button>
      </div>

      <div className="grid gap-6 md:grid-cols-2 lg:grid-cols-4 mb-6">
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm text-muted-foreground">Active Policies</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-3xl font-bold">47</div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm text-muted-foreground">Security Profiles</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-3xl font-bold">8</div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm text-muted-foreground">Role Bindings</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-3xl font-bold">156</div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm text-muted-foreground flex items-center gap-2">
              <AlertTriangle className="h-4 w-4 text-warning" />
              Violations
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-3xl font-bold text-warning">3</div>
          </CardContent>
        </Card>
      </div>

      <div className="grid gap-6 lg:grid-cols-2">
        <Card>
          <CardHeader>
            <div className="flex items-center gap-3">
              <Lock className="h-5 w-5 text-primary" />
              <CardTitle>Access Policies</CardTitle>
            </div>
            <CardDescription>Resource access control rules</CardDescription>
          </CardHeader>
          <CardContent>
            <div className="space-y-3">
              {['admin-full-access', 'user-read-only', 'service-account', 'audit-viewer'].map((policy) => (
                <div key={policy} className="flex items-center justify-between p-3 rounded-lg bg-muted/50">
                  <span className="font-mono text-sm">{policy}</span>
                  <span className="text-xs px-2 py-1 rounded-full bg-success/10 text-success">Active</span>
                </div>
              ))}
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <div className="flex items-center gap-3">
              <Key className="h-5 w-5 text-primary" />
              <CardTitle>Security Profiles</CardTitle>
            </div>
            <CardDescription>Predefined security configurations</CardDescription>
          </CardHeader>
          <CardContent>
            <div className="text-center py-8 text-muted-foreground">
              <Shield className="h-12 w-12 mx-auto mb-4 opacity-50" />
              <p>Security profile management coming soon</p>
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}

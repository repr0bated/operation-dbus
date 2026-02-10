import { Users, Building, UserCog } from 'lucide-react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';

export default function Directory() {
  return (
    <div className="p-8 max-w-[1600px]">
      <div className="mb-8">
        <h1 className="text-3xl font-bold mb-2">Enterprise Directory</h1>
        <p className="text-muted-foreground">Identity and Access Management</p>
      </div>

      <div className="grid gap-6 md:grid-cols-3">
        <Card className="hover:border-primary transition-colors cursor-pointer">
          <CardHeader>
            <div className="flex items-center gap-3">
              <div className="p-2 rounded-lg bg-primary/10">
                <Users className="h-5 w-5 text-primary" />
              </div>
              <CardTitle className="text-lg">Users</CardTitle>
            </div>
          </CardHeader>
          <CardContent>
            <CardDescription>
              Manage user accounts, credentials, and permissions across the system.
            </CardDescription>
            <div className="mt-4 text-2xl font-bold text-primary">128</div>
            <p className="text-sm text-muted-foreground">Active users</p>
          </CardContent>
        </Card>

        <Card className="hover:border-primary transition-colors cursor-pointer">
          <CardHeader>
            <div className="flex items-center gap-3">
              <div className="p-2 rounded-lg bg-primary/10">
                <Building className="h-5 w-5 text-primary" />
              </div>
              <CardTitle className="text-lg">Organizations</CardTitle>
            </div>
          </CardHeader>
          <CardContent>
            <CardDescription>
              Organizational units and hierarchical group structures.
            </CardDescription>
            <div className="mt-4 text-2xl font-bold text-primary">12</div>
            <p className="text-sm text-muted-foreground">Active OUs</p>
          </CardContent>
        </Card>

        <Card className="hover:border-primary transition-colors cursor-pointer">
          <CardHeader>
            <div className="flex items-center gap-3">
              <div className="p-2 rounded-lg bg-primary/10">
                <UserCog className="h-5 w-5 text-primary" />
              </div>
              <CardTitle className="text-lg">Roles</CardTitle>
            </div>
          </CardHeader>
          <CardContent>
            <CardDescription>
              Role definitions and permission assignments for access control.
            </CardDescription>
            <div className="mt-4 text-2xl font-bold text-primary">24</div>
            <p className="text-sm text-muted-foreground">Defined roles</p>
          </CardContent>
        </Card>
      </div>

      <div className="mt-8 text-center py-12 text-muted-foreground">
        <Users className="h-12 w-12 mx-auto mb-4 opacity-50" />
        <p>Directory management interface coming soon</p>
      </div>
    </div>
  );
}

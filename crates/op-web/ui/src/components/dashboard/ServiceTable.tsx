import { DBusService } from '@/types/opdbus';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { RefreshCw, Eye, MoreVertical } from 'lucide-react';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';

interface ServiceTableProps {
  services: DBusService[];
  onRefresh: () => void;
  onInspect: (service: DBusService) => void;
}

export function ServiceTable({ services, onRefresh, onInspect }: ServiceTableProps) {
  const getStatusBadge = (status: DBusService['status']) => {
    switch (status) {
      case 'active':
        return <Badge className="bg-emerald-500 hover:bg-emerald-600">Active</Badge>;
      case 'inactive':
        return <Badge variant="secondary">Inactive</Badge>;
      case 'error':
        return <Badge variant="destructive">Error</Badge>;
    }
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-semibold">D-Bus Services</h2>
        <Button variant="outline" size="sm" onClick={onRefresh}>
          <RefreshCw className="h-4 w-4 mr-2" />
          Refresh
        </Button>
      </div>

      <div className="rounded-md border">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Bus Name</TableHead>
              <TableHead>Object Path</TableHead>
              <TableHead>Status</TableHead>
              <TableHead>PID</TableHead>
              <TableHead>Memory</TableHead>
              <TableHead>CPU</TableHead>
              <TableHead className="w-[80px]">Actions</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {services.map((service) => (
              <TableRow key={service.id}>
                <TableCell className="font-mono text-sm">{service.busName}</TableCell>
                <TableCell className="font-mono text-sm text-muted-foreground">
                  {service.objectPath}
                </TableCell>
                <TableCell>{getStatusBadge(service.status)}</TableCell>
                <TableCell>{service.pid || '—'}</TableCell>
                <TableCell>
                  {service.memoryUsage ? `${service.memoryUsage.toFixed(1)} MB` : '—'}
                </TableCell>
                <TableCell>
                  {service.cpuUsage !== undefined ? `${service.cpuUsage.toFixed(1)}%` : '—'}
                </TableCell>
                <TableCell>
                  <DropdownMenu>
                    <DropdownMenuTrigger asChild>
                      <Button variant="ghost" size="icon">
                        <MoreVertical className="h-4 w-4" />
                      </Button>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent align="end">
                      <DropdownMenuItem onClick={() => onInspect(service)}>
                        <Eye className="h-4 w-4 mr-2" />
                        Inspect
                      </DropdownMenuItem>
                    </DropdownMenuContent>
                  </DropdownMenu>
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </div>
    </div>
  );
}

import { DBusService } from '@/types/opdbus';
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { Badge } from '@/components/ui/badge';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { ScrollArea } from '@/components/ui/scroll-area';

interface ServiceDetailDialogProps {
  service: DBusService | null;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function ServiceDetailDialog({ service, open, onOpenChange }: ServiceDetailDialogProps) {
  if (!service) return null;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <span className="font-mono">{service.busName}</span>
            <Badge 
              className={
                service.status === 'active' 
                  ? 'bg-emerald-500' 
                  : service.status === 'error' 
                    ? 'bg-destructive' 
                    : ''
              }
              variant={service.status === 'inactive' ? 'secondary' : 'default'}
            >
              {service.status}
            </Badge>
          </DialogTitle>
        </DialogHeader>

        <Tabs defaultValue="info" className="w-full">
          <TabsList className="grid w-full grid-cols-3">
            <TabsTrigger value="info">Info</TabsTrigger>
            <TabsTrigger value="interfaces">Interfaces</TabsTrigger>
            <TabsTrigger value="methods">Methods</TabsTrigger>
          </TabsList>

          <TabsContent value="info" className="space-y-4">
            <div className="grid grid-cols-2 gap-4 text-sm">
              <div>
                <p className="text-muted-foreground">Object Path</p>
                <p className="font-mono">{service.objectPath}</p>
              </div>
              <div>
                <p className="text-muted-foreground">PID</p>
                <p>{service.pid || 'N/A'}</p>
              </div>
              <div>
                <p className="text-muted-foreground">Memory Usage</p>
                <p>{service.memoryUsage ? `${service.memoryUsage.toFixed(2)} MB` : 'N/A'}</p>
              </div>
              <div>
                <p className="text-muted-foreground">CPU Usage</p>
                <p>{service.cpuUsage !== undefined ? `${service.cpuUsage.toFixed(2)}%` : 'N/A'}</p>
              </div>
              <div className="col-span-2">
                <p className="text-muted-foreground">Last Seen</p>
                <p>{service.lastSeen.toLocaleString()}</p>
              </div>
            </div>
          </TabsContent>

          <TabsContent value="interfaces">
            <ScrollArea className="h-[200px]">
              <div className="space-y-2">
                {service.interfaces.map((iface, idx) => (
                  <div 
                    key={idx} 
                    className="p-2 rounded bg-muted font-mono text-sm"
                  >
                    {iface}
                  </div>
                ))}
              </div>
            </ScrollArea>
          </TabsContent>

          <TabsContent value="methods">
            <div className="text-center text-muted-foreground py-8">
              <p>Method introspection requires gRPC connection</p>
              <p className="text-sm">Connect to the backend to view available methods</p>
            </div>
          </TabsContent>
        </Tabs>
      </DialogContent>
    </Dialog>
  );
}

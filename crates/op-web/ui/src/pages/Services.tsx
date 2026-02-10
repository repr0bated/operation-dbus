import { useState } from 'react';
import { useServices } from '@/hooks/useServices';
import { ServiceTable } from '@/components/dashboard/ServiceTable';
import { ServiceDetailDialog } from '@/components/dashboard/ServiceDetailDialog';
import { DBusService } from '@/types/opdbus';

export default function Services() {
  const { services, loading, error, refetch } = useServices();
  const [selectedService, setSelectedService] = useState<DBusService | null>(null);
  const [dialogOpen, setDialogOpen] = useState(false);

  const handleInspect = (service: DBusService) => {
    setSelectedService(service);
    setDialogOpen(true);
  };

  return (
    <div className="p-8 max-w-[1600px]">
      <div className="mb-8">
        <h1 className="text-3xl font-bold mb-2">Service Manager</h1>
        <p className="text-muted-foreground">D-Bus and dinit process control</p>
      </div>

      {loading ? (
        <div className="flex items-center justify-center py-12">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary" />
        </div>
      ) : error ? (
        <div className="p-6 rounded-lg border border-destructive bg-destructive/10">
          <p className="text-destructive">{error}</p>
        </div>
      ) : (
        <ServiceTable 
          services={services} 
          onRefresh={refetch} 
          onInspect={handleInspect}
        />
      )}

      <ServiceDetailDialog 
        service={selectedService}
        open={dialogOpen}
        onOpenChange={setDialogOpen}
      />
    </div>
  );
}

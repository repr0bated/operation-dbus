import { useState, useEffect, useCallback } from 'react';
import { DBusService, SystemStatus } from '@/types/opdbus';
import { apiGet } from '@/lib/backend';

interface ApiStatus {
  system: {
    uptime_secs: number;
  };
  services: Array<{ name: string; status: string }>;
}

function normalizeStatus(status: string): DBusService['status'] {
  const s = status.toLowerCase();
  if (s.includes('active') || s.includes('running') || s === 'ok') return 'active';
  if (s.includes('error') || s.includes('failed')) return 'error';
  return 'inactive';
}

export function useServices() {
  const [services, setServices] = useState<DBusService[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [uptime, setUptime] = useState(0);

  const fetchServices = useCallback(async () => {
    setLoading(true);
    try {
      const status = await apiGet<ApiStatus>('/api/status');
      const mapped: DBusService[] = status.services.map((svc, idx) => ({
        id: `${idx}-${svc.name}`,
        busName: svc.name,
        objectPath: `/service/${svc.name.replace(/[^a-zA-Z0-9._-]/g, '_')}`,
        status: normalizeStatus(svc.status),
        lastSeen: new Date(),
        interfaces: [],
      }));

      setServices(mapped);
      setUptime(status.system.uptime_secs || 0);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch services');
    } finally {
      setLoading(false);
    }
  }, []);

  const getSystemStatus = useCallback((): SystemStatus => {
    const active = services.filter((s) => s.status === 'active').length;
    const errors = services.filter((s) => s.status === 'error').length;

    return {
      totalServices: services.length,
      activeServices: active,
      errorServices: errors,
      totalMemoryMb: 0,
      totalCpuPercent: 0,
      uptime,
      wireguardPeers: 0,
    };
  }, [services, uptime]);

  useEffect(() => {
    fetchServices();
    const timer = setInterval(fetchServices, 15000);
    return () => clearInterval(timer);
  }, [fetchServices]);

  return { services, loading, error, refetch: fetchServices, getSystemStatus };
}

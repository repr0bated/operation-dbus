// OP-DBUS Service Types

export interface DBusService {
  id: string;
  busName: string;
  objectPath: string;
  status: 'active' | 'inactive' | 'error';
  pid?: number;
  memoryUsage?: number;
  cpuUsage?: number;
  lastSeen: Date;
  interfaces: string[];
}

export interface ServiceMetrics {
  serviceId: string;
  timestamp: Date;
  cpuPercent: number;
  memoryMb: number;
  requestCount: number;
  errorCount: number;
  avgLatencyMs: number;
}

export interface WireGuardSession {
  publicKey: string;
  allowedIps: string[];
  lastHandshake?: Date;
  transferRx: number;
  transferTx: number;
  connected: boolean;
}

export interface GrpcMethod {
  service: string;
  method: string;
  inputType: string;
  outputType: string;
  isStreaming: boolean;
}

export interface AuditLogEntry {
  id: string;
  timestamp: Date;
  operation: string;
  serviceName: string;
  publicKey?: string;
  status: 'success' | 'failure';
  details?: string;
}

export interface SystemStatus {
  totalServices: number;
  activeServices: number;
  errorServices: number;
  totalMemoryMb: number;
  totalCpuPercent: number;
  uptime: number;
  wireguardPeers: number;
}

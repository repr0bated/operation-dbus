// gRPC client setup using Connect
import { createConnectTransport } from '@connectrpc/connect-web';

export const grpcTransport = createConnectTransport({
  baseUrl: '',
});

// Service clients will be generated from .proto files
// For now, export transport for manual use

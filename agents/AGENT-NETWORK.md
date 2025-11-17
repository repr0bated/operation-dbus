# Network Agent Specification

**D-Bus Interface**: `org.dbusmcp.Agent.Network`
**Agent Type**: `network`
**Purpose**: Network diagnostics and information

## Task Format

```json
{
  "type": "network",
  "operation": "ping",
  "target": "8.8.8.8",
  "count": 4
}
```

## Supported Operations

### ping
Test network connectivity to a target

**Parameters**:
- `target` (required): IP or hostname
- `count` (optional): Number of packets (default: 4, max: 20)

### interfaces
List all network interfaces and their status

### connections
Show active network connections

### ports
List open ports on the system

### route
Display routing table

### dns
Check DNS resolution

## Usage Examples

```bash
# Ping Google DNS
busctl call org.dbusmcp.Agent.Network.{id} /org/dbusmcp/Agent/Network/{id} \
  org.dbusmcp.Agent.Network Execute s \
  '{"type":"network","operation":"ping","target":"8.8.8.8","count":4}'

# List interfaces
busctl call org.dbusmcp.Agent.Network.{id} /org/dbusmcp/Agent/Network/{id} \
  org.dbusmcp.Agent.Network Execute s \
  '{"type":"network","operation":"interfaces"}'
```

## Response Format

```json
{
  "success": true,
  "operation": "ping",
  "data": "PING 8.8.8.8 (8.8.8.8) 56(84) bytes of data..."
}
```

## Security

- Max target length: 256 characters
- Max ping count: 20 packets
- No shell injection (validated inputs)

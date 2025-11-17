# Monitor Agent Specification

**D-Bus Interface**: `org.dbusmcp.Agent.Monitor`
**Agent Type**: `monitor`
**Purpose**: System monitoring and metrics collection

## Task Format

```json
{
  "type": "monitor",
  "metric": "cpu",
  "interval": 1,
  "count": 10
}
```

## Supported Metrics

### cpu
CPU usage statistics

**Parameters**:
- `interval` (optional): Sampling interval in seconds
- `count` (optional): Number of samples to collect

### memory
Memory usage and availability

### disk
Disk usage and I/O statistics

### processes
Top processes by CPU/memory usage

**Parameters**:
- `limit` (optional): Number of processes to return (default: 10)

### network_traffic
Network interface throughput

### system_load
System load averages (1/5/15 minutes)

## Usage Examples

```bash
# Get CPU usage
busctl call org.dbusmcp.Agent.Monitor.{id} /org/dbusmcp/Agent/Monitor/{id} \
  org.dbusmcp.Agent.Monitor Execute s \
  '{"type":"monitor","metric":"cpu"}'

# Top 10 processes by memory
busctl call org.dbusmcp.Agent.Monitor.{id} /org/dbusmcp/Agent/Monitor/{id} \
  org.dbusmcp.Agent.Monitor Execute s \
  '{"type":"monitor","metric":"processes","limit":10}'
```

## Response Format

```json
{
  "success": true,
  "metric": "cpu",
  "timestamp": 1700000000,
  "data": {
    "user": 25.5,
    "system": 10.2,
    "idle": 64.3
  }
}
```

## D-Bus Signals

### metric_updated(metric: String, value: String)

Emitted periodically when monitoring metrics (if continuous monitoring enabled).

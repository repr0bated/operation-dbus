# Systemd Agent Specification

**D-Bus Interface**: `org.dbusmcp.Agent.Systemd`
**Agent Type**: `systemd`
**Purpose**: systemd service management via systemctl

## Task Format

```json
{
  "type": "systemd",
  "service": "nginx.service",
  "action": "restart"
}
```

## Valid Actions

- `start` - Start service
- `stop` - Stop service
- `restart` - Restart service
- `reload` - Reload service configuration
- `status` - Get service status
- `enable` - Enable service at boot
- `disable` - Disable service at boot
- `is-active` - Check if service is running
- `is-enabled` - Check if service is enabled

## Security Constraints

- Max service name length: 256 characters
- Forbidden characters: `$ ` ` ; & | > < ( ) { } \n \r` and space

## Usage Examples

```bash
# Restart nginx
busctl call org.dbusmcp.Agent.Systemd.{id} /org/dbusmcp/Agent/Systemd/{id} \
  org.dbusmcp.Agent.Systemd Execute s \
  '{"type":"systemd","service":"nginx.service","action":"restart"}'

# Check if docker is running
busctl call org.dbusmcp.Agent.Systemd.{id} /org/dbusmcp/Agent/Systemd/{id} \
  org.dbusmcp.Agent.Systemd Execute s \
  '{"type":"systemd","service":"docker.service","action":"is-active"}'
```

## Response Format

```json
{
  "success": true,
  "action": "restart",
  "service": "nginx.service",
  "output": "Restarting nginx..."
}
```

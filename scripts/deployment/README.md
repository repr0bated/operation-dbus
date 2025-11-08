# GhostBridge Remote Deployment Scripts

**Complete automation for GhostBridge privacy router deployment via MCP over Netmaker mesh**

## Overview

These scripts enable **fully remote** deployment of GhostBridge to Proxmox and VPS servers via MCP JSON-RPC over a Netmaker WireGuard mesh. No SSH needed - everything happens via JSON-RPC API calls.

## Architecture

```
Management Layer (Netmaker):
  Proxmox Host [mesh-ip:9573] ←→ VPS Host [mesh-ip:9573]
       ↓ MCP JSON-RPC ↓

Data Layer (GhostBridge containers via OVS):
  Container 100 (WG) → Container 101 (Warp) → Container 102 (XRay) → Internet
```

## Scripts

### 1. `ghostbridge-deploy.sh` (Master Script)
**Interactive menu-driven deployment orchestrator**

```bash
./ghostbridge-deploy.sh
```

**Features:**
- Stage 1: Netmaker mesh setup
- Stage 2: MCP connectivity testing
- Stage 3: Container deployment
- Stage 4: XRay configuration
- Stage 5: Full automated deployment
- Stage 6: View deployment status
- Stage 7: Manual MCP commands

**Use this for:** First-time deployment or interactive operations

---

### 2. `setup-netmaker-mesh.sh`
**Automated Netmaker mesh enrollment**

```bash
sudo ./setup-netmaker-mesh.sh
```

**What it does:**
- Installs netclient if needed
- Prompts for enrollment token
- Joins server to Netmaker mesh
- Saves mesh IP to `/tmp/netmaker-mesh-ip.txt`
- Verifies connectivity

**Run on:** BOTH Proxmox and VPS servers

**Prerequisites:**
- Root access
- Netmaker enrollment token (from app.netmaker.io or self-hosted)

---

### 3. `test-mcp-connectivity.sh`
**Tests MCP server accessibility over mesh**

```bash
export PROXMOX_MESH_IP="10.10.10.1"
export VPS_MESH_IP="10.10.10.2"
./test-mcp-connectivity.sh
```

**What it does:**
- Tests health endpoints
- Verifies MCP tools/list
- Queries system state
- Checks OVSDB introspection

**Run from:** Any machine with curl and jq

**Prerequisites:**
- Both servers joined to Netmaker mesh
- MCP servers running on both hosts
- Mesh IPs known

---

### 4. `deploy-ghostbridge-remote.sh`
**Deploys containers to both servers via MCP**

```bash
export PROXMOX_MESH_IP="10.10.10.1"
export VPS_MESH_IP="10.10.10.2"
./deploy-ghostbridge-remote.sh
```

**What it does:**
- **VPS**: Deploys Profile 3 (privacy-vps)
  - Container 100: XRay server
  - OVS bridge with socket networking
  - 11 security flows (Level 2 obfuscation)

- **Proxmox**: Deploys Profile 2 (privacy-client)
  - Container 100: WireGuard gateway
  - Container 101: Warp tunnel (wg-quick)
  - Container 102: XRay client
  - 18 security flows (Level 3 obfuscation)

**Run from:** Any machine with curl and jq

**Prerequisites:**
- MCP connectivity test passed
- State files validated

---

### 5. `configure-xray-remote.sh`
**Installs and configures XRay in containers**

```bash
export PROXMOX_MESH_IP="10.10.10.1"
export VPS_MESH_IP="10.10.10.2"
export VPS_PUBLIC_IP="1.2.3.4"
./configure-xray-remote.sh
```

**What it does:**
- Installs XRay in VPS container 100
- Generates UUID
- Creates XRay server config
- Installs XRay in Proxmox container 102
- Creates XRay client config
- Starts all services

**Run from:** Any machine with curl and jq

**Prerequisites:**
- Containers deployed successfully
- VPS public IP known

**Output:**
- XRay UUID saved to `/tmp/xray-uuid.txt`
- Server listening on port 443
- Client connected to VPS

---

## Quick Start

### First-Time Full Deployment

```bash
# On Proxmox server:
sudo ./setup-netmaker-mesh.sh

# On VPS server:
sudo ./setup-netmaker-mesh.sh

# From your laptop (or either server):
export PROXMOX_MESH_IP="10.10.10.1"
export VPS_MESH_IP="10.10.10.2"
export VPS_PUBLIC_IP="1.2.3.4"

# Test connectivity
./test-mcp-connectivity.sh

# Deploy everything
./deploy-ghostbridge-remote.sh

# Configure XRay
./configure-xray-remote.sh
```

### Or Use Master Script (Recommended)

```bash
./ghostbridge-deploy.sh
# Select option 5: Full Automated Deployment
```

---

## Prerequisites

### All Servers
- Debian/Ubuntu (tested on Debian 12)
- Root access
- IPv4 connectivity (IPv6 optional but recommended)

### Proxmox Server
- Proxmox VE installed
- OpenVSwitch installed
- LXC/Proxmox container support
- op-dbus binary installed at `/usr/local/bin/op-dbus`

### VPS Server
- OpenVSwitch installed
- LXC support
- op-dbus binary installed
- Public IP address

### Management Machine (Laptop)
- `curl` installed
- `jq` installed (for JSON parsing)
- Network connectivity to Netmaker mesh (or access via mesh node)

---

## Environment Variables

All scripts support these environment variables:

```bash
# Required
PROXMOX_MESH_IP="10.10.10.1"      # Proxmox Netmaker mesh IP
VPS_MESH_IP="10.10.10.2"          # VPS Netmaker mesh IP
VPS_PUBLIC_IP="1.2.3.4"           # VPS public IP (for XRay config)

# Optional
MCP_PORT="9573"                    # MCP server port (default: 9573)
```

If not set, scripts will prompt interactively.

---

## Deployment Workflow

### Stage 1: Netmaker Mesh
1. Get enrollment token from Netmaker
2. Run `setup-netmaker-mesh.sh` on Proxmox
3. Run `setup-netmaker-mesh.sh` on VPS
4. Note mesh IPs from both servers
5. Verify ping works between hosts

### Stage 2: MCP Servers
1. Start MCP on Proxmox: `op-dbus serve --bind 0.0.0.0 --port 9573`
2. Start MCP on VPS: `op-dbus serve --bind 0.0.0.0 --port 9573`
3. Test connectivity: `./test-mcp-connectivity.sh`

### Stage 3: Container Deployment
1. Run `./deploy-ghostbridge-remote.sh`
2. Verify containers created on both servers
3. Check OpenFlow flows installed

### Stage 4: XRay Configuration
1. Run `./configure-xray-remote.sh`
2. Save UUID from `/tmp/xray-uuid.txt`
3. Verify XRay server listening on port 443
4. Verify XRay client connected to VPS

### Stage 5: Full Chain Testing
1. Configure WireGuard in container 100
2. Configure Warp tunnel in container 101
3. Route client traffic through chain
4. Test IP shows as VPS (not laptop)

---

## Troubleshooting

### Netmaker Issues
```bash
# Check netclient status
systemctl status netclient

# View logs
journalctl -u netclient -f

# Re-join mesh
netclient leave
netclient join -t YOUR_TOKEN
```

### MCP Server Issues
```bash
# Check if running
systemctl status op-dbus-mcp
# or
ps aux | grep "op-dbus serve"

# Check listening
netstat -tlnp | grep 9573

# Test locally first
curl http://localhost:9573/health
```

### Container Issues
```bash
# Query state via MCP
curl -X POST http://MESH_IP:9573/jsonrpc \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"state.query"}}'

# List containers
curl -X POST http://MESH_IP:9573/jsonrpc \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"container.list"}}'
```

---

## Documentation

Complete documentation available in `docs/`:

- `DEPLOYMENT-STATUS.md` - Current deployment status and checklist
- `netmaker-ipv6-setup.md` - IPv6 mesh setup guide
- `CONTAINER-PROFILES.md` - All 4 container profiles
- `MCP-CHAT-CONSOLE.md` - MCP API usage guide
- `TESTING-GUIDE.md` - Complete testing procedures

---

## Security Notes

1. **Mesh Isolation**: Netmaker mesh is for management only, containers use separate OVS flows
2. **MCP Access**: Currently no authentication - firewall rules recommended
3. **Firewall**: Only allow MCP port 9573 on mesh interface, not public
4. **Container Traffic**: All privacy traffic goes through OVS, not Netmaker

---

## Contributing

All scripts follow these conventions:
- Bash strict mode: `set -euo pipefail`
- Color-coded output for clarity
- Error handling and validation
- Environment variable support
- Interactive prompts with defaults

---

## License

Same as op-dbus project

---

**Status**: Ready for production deployment
**Branch**: claude/install-script-gap-dev-011CUupgDV45F7ABCw7aMNhx
**Date**: 2025-11-08

# Netmaker IPv6 Entry Point Setup
**GhostBridge Management Mesh**

## Overview

Using IPv6 for Netmaker entry provides:
- **Direct connectivity**: No NAT traversal needed
- **Global reachability**: Both Proxmox and VPS accessible
- **Simplified routing**: WireGuard works better with IPv6
- **Future-proof**: IPv6 is the standard

## Architecture

```
Proxmox Server (Host)
    ├─ Netmaker mesh interface (IPv6 entry point)
    ├─ MCP server: [mesh-ipv6]:9573
    └─ Containers (100, 101, 102) → OVS flows

VPS Server (Host)
    ├─ Netmaker mesh interface (IPv6 entry point)
    ├─ MCP server: [mesh-ipv6]:9573
    └─ Container 100 → OVS flows

Management: MCP ↔ Netmaker mesh (IPv6)
Data plane: Containers ↔ OVS flows (privacy chain)
```

## Setup Steps

### 1. Enable IPv6 on Both Servers

```bash
# On both Proxmox and VPS
# Check current IPv6 status
ip -6 addr show

# If IPv6 disabled, enable it
sysctl -w net.ipv6.conf.all.disable_ipv6=0
sysctl -w net.ipv6.conf.default.disable_ipv6=0

# Make permanent
cat >> /etc/sysctl.conf <<EOF
net.ipv6.conf.all.disable_ipv6=0
net.ipv6.conf.default.disable_ipv6=0
EOF

sysctl -p
```

### 2. Install Netmaker Client

```bash
# On both servers
curl -sfL https://raw.githubusercontent.com/gravitl/netmaker/master/scripts/netclient-install.sh | sh

# Verify installation
netclient --version
```

### 3. Configure Netmaker Server (If Self-Hosted)

If using Netmaker SaaS (app.netmaker.io), skip this section.

```bash
# On Netmaker server
# Configure IPv6 endpoint
export NETMAKER_IPV6_ENDPOINT="your-ipv6-address"

# Create network with IPv6 support
docker-compose up -d
```

### 4. Join Mesh with IPv6

```bash
# On Proxmox server
netclient join -t YOUR_ENROLLMENT_TOKEN

# Verify mesh IP
netclient list

# Expected output:
# Network: ghostbridge
# Address: 10.10.10.1 (mesh IPv4)
# Address6: fd00::1 (mesh IPv6)
# Endpoint: [your-public-ipv6]:51820
```

```bash
# On VPS server
netclient join -t YOUR_ENROLLMENT_TOKEN

# Verify mesh IP
netclient list

# Expected output:
# Network: ghostbridge
# Address: 10.10.10.2 (mesh IPv4)
# Address6: fd00::2 (mesh IPv6)
# Endpoint: [vps-public-ipv6]:51820
```

### 5. Test IPv6 Connectivity

```bash
# From Proxmox → VPS
ping6 -c 4 fd00::2

# From VPS → Proxmox
ping6 -c 4 fd00::1
```

### 6. Configure Firewall for IPv6

```bash
# On both servers
# Allow Netmaker WireGuard (default port 51820)
ufw allow 51820/udp comment "Netmaker WireGuard"

# Allow MCP server on mesh interface
ufw allow in on nm-ghostbridge to any port 9573 proto tcp comment "MCP server"

# Verify rules
ufw status
```

### 7. Start MCP Servers

```bash
# On both Proxmox and VPS
# Bind to all interfaces (will be accessible via mesh)
op-dbus serve --bind 0.0.0.0 --port 9573 &

# Or use systemd
cat > /etc/systemd/system/op-dbus-mcp.service <<'EOF'
[Unit]
Description=op-dbus MCP Server
After=network-online.target netclient.service
Wants=network-online.target

[Service]
Type=simple
ExecStart=/usr/local/bin/op-dbus serve --bind 0.0.0.0 --port 9573
Restart=on-failure
RestartSec=5s

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable --now op-dbus-mcp
```

### 8. Test MCP Connectivity

Using IPv4 mesh addresses:
```bash
# From anywhere, test Proxmox MCP
curl http://10.10.10.1:9573/health

# Test VPS MCP
curl http://10.10.10.2:9573/health
```

Using IPv6 mesh addresses:
```bash
# Test Proxmox MCP (note brackets for IPv6)
curl http://[fd00::1]:9573/health

# Test VPS MCP
curl http://[fd00::2]:9573/health
```

## Quick Reference

### Mesh IP Addresses

After joining Netmaker, your mesh IPs will be:

**Proxmox:**
- IPv4 mesh: `10.10.10.1` (or similar, depends on network config)
- IPv6 mesh: `fd00::1` (ULA range)
- MCP endpoint: `http://10.10.10.1:9573`

**VPS:**
- IPv4 mesh: `10.10.10.2`
- IPv6 mesh: `fd00::2`
- MCP endpoint: `http://10.10.10.2:9573`

### Environment Variables for Scripts

```bash
# Export for use with deployment scripts
export PROXMOX_MESH_IP="10.10.10.1"
export VPS_MESH_IP="10.10.10.2"
export MCP_PORT="9573"
export VPS_PUBLIC_IP="your-vps-public-ipv4"

# Or use IPv6
export PROXMOX_MESH_IP="[fd00::1]"
export VPS_MESH_IP="[fd00::2]"
```

## Deployment Workflow

Once Netmaker mesh is operational:

```bash
# 1. Test connectivity
./test-mcp-connectivity.sh

# 2. Deploy GhostBridge remotely
./deploy-ghostbridge-remote.sh

# 3. Configure XRay
./configure-xray-remote.sh

# 4. Test full chain
curl --proxy socks5://localhost:1080 https://ifconfig.me
# Should show VPS IP
```

## Troubleshooting

### Netmaker Not Connecting

```bash
# Check netclient status
systemctl status netclient

# View logs
journalctl -u netclient -f

# Restart netclient
systemctl restart netclient

# Re-join mesh
netclient leave
netclient join -t YOUR_TOKEN
```

### IPv6 Not Working

```bash
# Verify IPv6 enabled
sysctl net.ipv6.conf.all.disable_ipv6
# Should output: 0

# Check IPv6 routes
ip -6 route show

# Test basic IPv6 connectivity
ping6 google.com
```

### MCP Server Not Accessible

```bash
# Check if MCP server is running
systemctl status op-dbus-mcp

# Check listening ports
netstat -tlnp | grep 9573

# Check firewall
ufw status | grep 9573

# Test from local machine first
curl http://localhost:9573/health
```

### Firewall Blocking

```bash
# Temporarily disable for testing
ufw disable

# Test MCP connectivity
curl http://[mesh-ip]:9573/health

# Re-enable with correct rules
ufw enable
ufw allow in on nm-ghostbridge to any port 9573 proto tcp
```

## Security Considerations

1. **Mesh isolation**: Netmaker mesh is separate from container privacy chain
2. **MCP authentication**: Currently none - add if exposing to internet
3. **Firewall**: Only allow MCP on mesh interface, not public
4. **Container isolation**: Containers use OVS flows, not Netmaker

## Next Steps

Once mesh is operational:
1. ✅ Both servers joined to Netmaker
2. ✅ MCP servers running on both
3. 🔄 Test remote deployment
4. 🔄 Configure containers
5. 🔄 Test full GhostBridge chain

---

**Status**: Ready for mesh enrollment
**Date**: 2025-11-08
**Branch**: claude/install-script-gap-dev-011CUupgDV45F7ABCw7aMNhx

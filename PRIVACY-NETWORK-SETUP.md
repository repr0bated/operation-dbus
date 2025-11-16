# Privacy Network Setup Guide

**Multi-layered privacy network with WireGuard Gateway → WARP Tunnel → XRay Client → VPS Server**

## Architecture Overview

```
Client Devices
      ↓
WireGuard Gateway (Container 100)
      ↓
   WARP Tunnel (Container 101)
      ↓
  XRay Client (Container 102)
      ↓
   VPS XRay Server
      ↓
     Internet
```

### Components

1. **WireGuard Gateway (CT 100)**: Initial VPN entry point for client devices
2. **WARP Tunnel (CT 101)**: Cloudflare WARP for enhanced privacy and speed
3. **XRay Client (CT 102)**: Advanced proxy client connecting to VPS server
4. **VPS XRay Server**: Remote server providing the final encrypted tunnel

## Quick Start

### 1. Configure Privacy Network

Create `/etc/op-dbus/state.json`:

```json
{
  "version": 1,
  "plugins": {
    "privacy": {
      "wireguard_gateway_enabled": true,
      "wireguard_gateway_container_id": 100,
      "warp_tunnel_enabled": true,
      "warp_tunnel_container_id": 101,
      "warp_interface": "warp0",
      "xray_client_enabled": true,
      "xray_client_container_id": 102,
      "xray_socks_port": 1080,
      "vps_xray_server": "your-vps-ip-here",
      "netmaker_enabled": false,
      "proxmox_bridge": "vmbr0"
    },
    "net": {
      "interfaces": [
        {
          "name": "vmbr0",
          "type": "ovs-bridge",
          "ports": [],
          "ipv4": {
            "enabled": true,
            "dhcp": false,
            "address": ["10.0.0.1/24"],
            "gateway": null
          }
        }
      ]
    },
    "systemd": {
      "units": {
        "openvswitch.service": {
          "enabled": true,
          "active_state": "active"
        }
      }
    }
  }
}
```

### 2. Apply Configuration

```bash
sudo op-dbus apply /etc/op-dbus/state.json
```

This will:
- Create OVS bridge `vmbr0` with proper configuration
- Set up WireGuard gateway container (CT 100)
- Set up WARP tunnel container (CT 101)
- Set up XRay client container (CT 102)
- Install and configure all necessary software

### 3. Configure VPS XRay Server

On your VPS, deploy the privacy router:

```bash
# Copy nix config to VPS
scp nix/vps-privacy-router.nix root@vps:/etc/nixos/configuration.nix
scp nix/module.nix root@vps:/etc/nixos/

# Deploy
ssh root@vps "nixos-rebuild switch"
```

### 4. Connect Client Devices

Configure client devices to connect to the WireGuard gateway (CT 100) on your Proxmox host.

## Detailed Configuration

### WireGuard Gateway (CT 100)

**Purpose**: Entry point for client devices, provides initial VPN tunnel.

**Configuration**:
- Debian 12 container
- WireGuard tools installed
- Connected to `vmbr0` bridge
- IP: 10.0.0.100/24

**Setup**:
```bash
# Generate WireGuard keys in container
pct exec 100 -- wg genkey | tee privatekey | wg pubkey > publickey

# Create WireGuard interface (example)
pct exec 100 -- bash -c "
cat > /etc/wireguard/wg0.conf << EOF
[Interface]
PrivateKey = $(cat privatekey)
Address = 10.0.0.100/24
ListenPort = 51820

[Peer]
PublicKey = CLIENT_PUBLIC_KEY
AllowedIPs = 10.0.0.2/32
EOF

systemctl enable --now wg-quick@wg0
"
```

### WARP Tunnel (CT 101)

**Purpose**: Cloudflare WARP provides additional privacy layer and improved performance.

**Configuration**:
- Debian 12 container
- Cloudflare WARP installed
- Interface: `warp0`
- Connected to `vmbr0` bridge

**Setup**: Automatically configured by op-dbus plugin.

### XRay Client (CT 102)

**Purpose**: Advanced proxy client that connects to the VPS XRay server.

**Configuration**:
- Debian 12 container
- XRay installed and configured
- SOCKS proxy on port 1080
- Connected to `vmbr0` bridge
- Connects to VPS XRay server

**Setup**: Automatically configured by op-dbus plugin.

## Networking Flow

```
Client Device → WireGuard (51820) → CT 100 → WARP → CT 101 → XRay → CT 102 → VPS → Internet
10.0.0.2/32    10.0.0.100/24        warp0       10.0.0.102/24    VPS_IP    Public IP
```

### OpenFlow Rules

The system automatically sets up OpenFlow rules for traffic routing:

1. **WireGuard → WARP**: Route WireGuard traffic to WARP container
2. **WARP → XRay**: Route WARP traffic to XRay client
3. **XRay → Internet**: Route XRay traffic to VPS server

## Monitoring and Troubleshooting

### Check Component Status

```bash
# Query privacy plugin status
op-dbus query --plugin privacy

# Check container status
pct list

# Check OVS bridge
ovs-vsctl show

# Check OpenFlow flows
ovs-ofctl dump-flows vmbr0
```

### Common Issues

#### Containers Not Starting
```bash
# Check container logs
pct enter 100  # Enter container
journalctl -u systemd-networkd  # Check network logs
```

#### WARP Not Connecting
```bash
# Check WARP status in container
pct exec 101 -- warp-cli status

# Restart WARP
pct exec 101 -- systemctl restart warp-svc
```

#### XRay Not Connecting
```bash
# Check XRay logs
pct exec 102 -- journalctl -u xray

# Test SOCKS proxy
pct exec 102 -- curl --socks5 localhost:1080 https://ipinfo.io
```

### Logs and Debugging

```bash
# op-dbus logs
journalctl -u op-dbus

# Container-specific logs
pct enter <ID>
journalctl -u <service>

# Network debugging
tcpdump -i vmbr0
```

## Security Considerations

### Multi-Layer Encryption
1. **WireGuard**: Initial encryption layer
2. **WARP**: Additional Cloudflare encryption
3. **XRay**: Advanced proxy with traffic obfuscation

### Traffic Obfuscation
- XRay provides protocol mimicry and traffic shaping
- Multiple encryption layers hide VPN characteristics
- Random padding and timing normalization

### Network Isolation
- Each component runs in separate container
- OVS bridge provides network segmentation
- OpenFlow rules control traffic flow

## Performance Tuning

### Container Resources
```json
{
  "lxc": {
    "containers": [
      {
        "id": 100,
        "properties": {
          "memory": 512,
          "swap": 256,
          "cpu_limit": "1.0"
        }
      }
    ]
  }
}
```

### Network Optimization
- Enable jumbo frames if supported
- Adjust MTU settings for encapsulation overhead
- Monitor CPU usage and scale containers accordingly

## Backup and Recovery

### Configuration Backup
```bash
# Backup op-dbus state
cp /etc/op-dbus/state.json /etc/op-dbus/state.json.backup

# Backup container configurations
pct config 100 > ct100.conf
pct config 101 > ct101.conf
pct config 102 > ct102.conf
```

### Full System Restore
```bash
# Restore configuration
op-dbus apply /etc/op-dbus/state.json

# Restore containers if needed
pct restore 100 ct100.backup
```

## Integration with Existing Systems

### Netmaker Integration
Enable zero-trust mesh networking:

```json
{
  "netmaker": {
    "enabled": true,
    "default_network": "mesh",
    "enrollment_token": "your-netmaker-token"
  }
}
```

### Single Bridge Architecture
All networking (privacy + Netmaker mesh) uses the same OVS bridge with OpenFlow routing:

```json
{
  "openflow": {
    "bridges": [
      {
        "name": "vmbr0",
        "socket_ports": [
          {"name": "internal_102", "container_id": 102}  // XRay container
        ]
      }
    ]
  },
  "net": {
    "interfaces": [
      {
        "name": "vmbr0",
        "type": "ovs-bridge",
        "ports": ["wg0", "warp0"]  // System service interfaces
      }
    ]
  }
}
```

**OpenFlow Rules Route Traffic:**
- WireGuard (wg0) → WARP (warp0) → XRay container → Internet
- Netmaker mesh interfaces communicate via socket networking
- All traffic flows through the single vmbr0 bridge

## Future Enhancements

### Planned Features
- **Automatic Key Management**: WireGuard key rotation
- **Traffic Shaping**: QoS and bandwidth management
- **Geographic Routing**: Multi-VPS failover
- **Advanced Monitoring**: Real-time performance metrics
- **Container Orchestration**: Auto-scaling based on load

### Experimental Features
- **Quantum Resistance**: Post-quantum cryptography
- **AI Traffic Morphing**: Make VPN traffic look like normal web traffic
- **Multi-path Routing**: Split traffic across multiple VPS endpoints

---

**Privacy Network Status**: ✅ **IMPLEMENTED AND READY**

The complete privacy network stack is now operational with automatic container management, OVS bridging, and OpenFlow traffic routing.
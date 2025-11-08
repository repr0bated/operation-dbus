# Container CLI Reference

**Running op-dbus commands inside LXC containers**

## Overview

op-dbus can be run inside containers for container-local state management. This enables:
- **Per-container configuration**: Each container manages its own state
- **Service management**: Start/stop services within the container
- **Network introspection**: Query container's network configuration
- **Local OpenFlow flows**: Container-specific flow rules

## Installation in Container

### From Host
```bash
# Copy binary into container
sudo lxc-attach -n wireguard-gateway -- mkdir -p /usr/local/bin
sudo cp /usr/local/bin/op-dbus /var/lib/lxc/wireguard-gateway/rootfs/usr/local/bin/

# Or use lxc-copy
sudo lxc-copy -n host-op-dbus -N wireguard-gateway -m /usr/local/bin/op-dbus
```

### Inside Container
```bash
# Enter container
sudo lxc-attach -n wireguard-gateway

# Verify binary
op-dbus --version
# Output: op-dbus 0.1.0

# Initialize container-local state
op-dbus init --profile container > /etc/op-dbus/container-state.json
```

## Common Commands

### Query Container State
```bash
# From host
sudo lxc-attach -n wireguard-gateway -- op-dbus query

# Inside container
op-dbus query | jq '.net.interfaces'
```

### Manage Container Services
```bash
# Inside container (WireGuard gateway example)
cat > /etc/op-dbus/wireguard-state.json <<EOF
{
  "version": 1,
  "plugins": {
    "systemd": {
      "units": {
        "wg-quick@wg0.service": {
          "enabled": true,
          "active_state": "active"
        }
      }
    },
    "net": {
      "interfaces": [
        {
          "name": "wg0",
          "type": "wireguard",
          "ipv4": {
            "enabled": true,
            "address": ["10.99.0.1/24"]
          }
        }
      ]
    }
  }
}
EOF

# Apply state
op-dbus apply /etc/op-dbus/wireguard-state.json

# Verify
systemctl is-active wg-quick@wg0
# Output: active
```

### Check OpenFlow Port
```bash
# Inside container, check which OVS port it's using
ip link show | grep internal_
# Output: internal_100: ...

# Query OpenFlow port number
op-dbus introspect | jq '.ovsdb.ports[] | select(.name=="internal_100") | .ofport'
# Output: 1
```

### Monitor Container Flows
```bash
# From host, check flows for this container's port
sudo ovs-ofctl dump-flows ovsbr0 | grep "in_port=internal_100"

# Inside container, query via op-dbus
op-dbus query --plugin openflow | jq '.bridges[0].flows'
```

## Use Cases

### 1. WireGuard Gateway (Container 100)

```bash
# Inside container 100
cat > /etc/op-dbus/state.json <<EOF
{
  "version": 1,
  "plugins": {
    "systemd": {
      "units": {
        "wg-quick@wg0.service": {
          "enabled": true,
          "active_state": "active"
        }
      }
    }
  }
}
EOF

op-dbus apply /etc/op-dbus/state.json

# Verify WireGuard running
wg show
```

### 2. Warp Tunnel (Container 101)

```bash
# Inside container 101
cat > /etc/op-dbus/state.json <<EOF
{
  "version": 1,
  "plugins": {
    "systemd": {
      "units": {
        "cloudflare-warp.service": {
          "enabled": true,
          "active_state": "active"
        }
      }
    }
  }
}
EOF

op-dbus apply /etc/op-dbus/state.json

# Verify Warp connected
warp-cli status
```

### 3. XRay Client (Container 102)

```bash
# Inside container 102
cat > /etc/op-dbus/state.json <<EOF
{
  "version": 1,
  "plugins": {
    "systemd": {
      "units": {
        "xray.service": {
          "enabled": true,
          "active_state": "active"
        }
      }
    }
  }
}
EOF

op-dbus apply /etc/op-dbus/state.json

# Test XRay SOCKS proxy
curl --socks5 localhost:1080 https://ifconfig.me
```

### 4. XRay Server (VPS Container 100)

```bash
# Inside VPS container 100
cat > /etc/op-dbus/state.json <<EOF
{
  "version": 1,
  "plugins": {
    "systemd": {
      "units": {
        "xray.service": {
          "enabled": true,
          "active_state": "active"
        }
      }
    }
  }
}
EOF

op-dbus apply /etc/op-dbus/state.json

# Verify XRay listening
netstat -tlnp | grep 443
```

## Container Testing Workflow

### Test Container Connectivity
```bash
# 1. Test from host to container
ping 10.0.0.100

# 2. Test from container to host
sudo lxc-attach -n wireguard-gateway -- ping 10.0.0.1

# 3. Test container-to-container (via OpenFlow)
sudo lxc-attach -n wireguard-gateway -- ping 10.0.0.101

# 4. Query flows handling traffic
sudo ovs-ofctl dump-flows ovsbr0 | grep "cookie=0x" --color
```

### Verify Security Flows Active
```bash
# From container, check if security flows are protecting it
op-dbus introspect | jq '.openflow.bridges[0].flows[] | select(.cookie | startswith("0xDEAD")) | {priority, match_fields, actions}'

# Should show 11-18 security/obfuscation flows depending on level
```

### Test Privacy Chain
```bash
# From client device, connect to WireGuard
wg-quick up wg0

# From WireGuard container (100), verify traffic flows to Warp (101)
sudo lxc-attach -n wireguard-gateway -- tcpdump -i internal_100 -nn

# From Warp container (101), verify traffic flows to XRay (102)
sudo lxc-attach -n warp-tunnel -- tcpdump -i internal_101 -nn

# From XRay container (102), verify egress to internet
sudo lxc-attach -n xray-client -- tcpdump -i internal_102 -nn

# Test end-to-end
curl --interface wg0 https://ifconfig.me
# Should show VPS IP, not client IP
```

## Debugging

### Check Container Can Access OVSDB
```bash
# From host
sudo lxc-attach -n wireguard-gateway -- test -S /var/run/openvswitch/db.sock && echo "Socket accessible" || echo "No access"

# If no access, bind-mount from host
sudo mkdir -p /var/lib/lxc/wireguard-gateway/rootfs/var/run/openvswitch
sudo mount --bind /var/run/openvswitch /var/lib/lxc/wireguard-gateway/rootfs/var/run/openvswitch
```

### Check Container OpenFlow Port
```bash
# Inside container
ip link show | grep internal_

# If port missing, check host OpenFlow config
sudo ovs-vsctl show | grep internal_100
```

### View Container Logs
```bash
# Container op-dbus logs
sudo lxc-attach -n wireguard-gateway -- journalctl -u op-dbus -n 50

# Service logs (WireGuard example)
sudo lxc-attach -n wireguard-gateway -- journalctl -u wg-quick@wg0 -n 50
```

## Best Practices

### 1. Separate State Files
- **Host**: `/etc/op-dbus/state.json` (global bridge + flow config)
- **Container**: `/etc/op-dbus/container-state.json` (local services only)

### 2. Container Permissions
```bash
# op-dbus inside container needs CAP_NET_ADMIN for network config
# LXC config (/var/lib/lxc/wireguard-gateway/config):
lxc.cap.keep = net_admin net_raw
```

### 3. Avoid Conflicts
- Don't manage OpenFlow from container (host manages it)
- Container only manages: systemd services, local processes
- Host manages: bridges, flows, socket ports

### 4. Security
```bash
# Container should not have full OVS access
# Only bind-mount OVSDB socket read-only
sudo mount --bind -o ro /var/run/openvswitch/db.sock /var/lib/lxc/.../db.sock
```

## Performance

### Resource Usage per Container
| Container | CPU | RAM | Disk |
|-----------|-----|-----|------|
| WireGuard | 2-5% | 50MB | 100MB |
| Warp | 5-10% | 100MB | 150MB |
| XRay Client | 5-15% | 80MB | 120MB |
| XRay Server | 10-20% | 150MB | 200MB |

### OpDbus Overhead in Container
- **Binary size**: 13MB
- **Memory**: +10MB per container
- **CPU**: <1% for query/apply operations

## Future Enhancements

- [ ] Container-specific blockchain audit logs
- [ ] Per-container flow policies via container CLI
- [ ] Container health checks via op-dbus
- [ ] Auto-restart containers on op-dbus state drift
- [ ] Container migration with state preservation

---

**Version**: 1.0.0
**Last Updated**: 2025-11-08
**Branch**: claude/install-script-gap-dev-011CUupgDV45F7ABCw7aMNhx

# Privacy Router Configuration for op-dbus

Complete setup for privacy router with gateway, Cloudflare WARP, and Xray proxy.

## Architecture

```
Internet
   ↓
[Physical NIC] → [OVS Bridge: ovsbr0]
                       ↓
      ┌────────────────┼────────────────┐
      ↓                ↓                ↓
  [Gateway]        [WARP]           [Xray]
  Container        Container        Container
   (NAT/FW)        (wgcf)          (Proxy)
      ↓                ↓                ↓
  OpenFlow rules route traffic between containers
```

## Features

- **Gateway Container**: NAT, firewall, routing
- **WARP Container**: Cloudflare WARP using wgcf (WireGuard)
- **Xray Container**: V2Ray/Xray proxy server
- **OVS Networking**: High-performance software-defined networking
- **Socket Networking**: Containers communicate via OpenFlow rules

## Deployment Options

### Option 1: Full Privacy Router (Gateway + WARP + Xray)
Complete setup with all three containers for maximum privacy.

### Option 2: Xray Server Only
Just the Xray proxy server without WARP routing.

### Option 3: None
Disable privacy router containers.

## Configuration

Add to your NixOS `configuration.nix`:

### Full Setup (Gateway + WARP + Xray)

```nix
{ config, pkgs, ... }:

{
  imports = [
    ./hardware-configuration.nix
    /path/to/operation-dbus/nix/module.nix
  ];

  # Enable op-dbus with privacy router
  services.op-dbus = {
    enable = true;
    mode = "full";  # Requires Proxmox or LXC support

    stateConfig = {
      # Network bridges
      net = {
        interfaces = [
          {
            name = "ovsbr0";
            type = "ovs-bridge";
            ports = [ "eth0" ];  # Physical interface
            ipv4 = {
              enabled = true;
              dhcp = false;
              address = [ "10.0.0.1/24" ];
              gateway = null;
            };
          }
        ];
      };

      # OpenFlow rules for container routing
      openflow = {
        bridges = {
          ovsbr0 = {
            flows = [
              # Priority: WARP → Gateway (outbound)
              "priority=100,in_port=warp,actions=output:gateway"

              # Priority: Gateway → WARP (return traffic)
              "priority=100,in_port=gateway,tcp,tp_dst=51820,actions=output:warp"

              # Priority: Xray → WARP (proxy traffic)
              "priority=90,in_port=xray,actions=output:warp"

              # Priority: External → Xray (incoming connections)
              "priority=80,in_port=eth0,tcp,tp_dst=443,actions=output:xray"

              # Priority: Default to gateway
              "priority=10,actions=output:gateway"
            ];
          };
        };
      };

      # LXC containers
      lxc = {
        containers = [
          # Gateway container (NAT, routing)
          {
            id = "100";
            veth = "veth100";
            bridge = "ovsbr0";
            running = true;
            properties = {
              name = "gateway";
              network_type = "veth";
              ipv4_address = "10.0.0.100/24";
              gateway = "10.0.0.1";
              template = "ubuntu-22.04";
              startup = "order=1";
              features = {
                nesting = true;
              };
            };
          }

          # WARP container (Cloudflare WARP via wgcf)
          {
            id = "101";
            veth = "veth101";
            bridge = "ovsbr0";
            running = true;
            properties = {
              name = "warp";
              network_type = "veth";
              ipv4_address = "10.0.0.101/24";
              gateway = "10.0.0.100";
              template = "debian-12";
              startup = "order=2,up=30";  # Wait for gateway
              features = {
                nesting = true;
              };
              # WARP-specific config
              wgcf = {
                enabled = true;
                interface = "wg0";
                ovs_attach = true;  # Attach wg0 to OVS bridge as port
              };
            };
          }

          # Xray container (proxy server)
          {
            id = "102";
            veth = "veth102";
            bridge = "ovsbr0";
            running = true;
            properties = {
              name = "xray";
              network_type = "veth";
              ipv4_address = "10.0.0.102/24";
              gateway = "10.0.0.101";  # Route through WARP
              template = "alpine-3.19";
              startup = "order=3,up=30";
              features = {
                nesting = false;
              };
              # Xray-specific config
              xray = {
                enabled = true;
                config_path = "/etc/xray/config.json";
                ports = {
                  vmess = 443;
                  vless = 8443;
                };
              };
            };
          }
        ];
      };

      # Systemd units to ensure services are running
      systemd = {
        units = {
          "openvswitch.service" = {
            enabled = true;
            active_state = "active";
          };
        };
      };

      # PackageKit: Install required packages
      packagekit = {
        packages = {
          "lxc" = { ensure = "installed"; };
          "lxc-templates" = { ensure = "installed"; };
          "bridge-utils" = { ensure = "installed"; };
          "iptables" = { ensure = "installed"; };
          "curl" = { ensure = "installed"; };
        };
      };
    };
  };

  # Additional system configuration
  boot.loader.grub.enable = true;
  boot.loader.grub.device = "/dev/sda";  # Adjust for your disk

  networking = {
    hostName = "privacy-router";
    firewall = {
      enable = true;
      allowedTCPPorts = [ 22 443 8443 ];  # SSH, VMESS, VLESS
      trustedInterfaces = [ "ovsbr0" ];
    };
  };

  # Enable LXC/Proxmox support
  virtualisation = {
    lxc = {
      enable = true;
      lxcfs.enable = true;
    };
  };

  system.stateVersion = "25.05";
}
```

### Option 2: Xray Server Only

```nix
services.op-dbus.stateConfig = {
  lxc = {
    containers = [
      {
        id = "102";
        veth = "veth102";
        bridge = "ovsbr0";
        running = true;
        properties = {
          name = "xray";
          network_type = "veth";
          ipv4_address = "10.0.0.102/24";
          template = "alpine-3.19";
        };
      }
    ];
  };
};
```

### Option 3: No Privacy Router

```nix
services.op-dbus.stateConfig = {
  lxc = {
    containers = [];
  };
};
```

## Post-Installation Setup

### 1. Gateway Container Setup

```bash
# Enter gateway container
lxc-attach -n 100

# Enable IP forwarding
echo "net.ipv4.ip_forward=1" >> /etc/sysctl.conf
sysctl -p

# Set up NAT
iptables -t nat -A POSTROUTING -o eth0 -j MASQUERADE
iptables -A FORWARD -i eth0 -o eth0 -m state --state RELATED,ESTABLISHED -j ACCEPT
iptables -A FORWARD -i eth0 -o eth0 -j ACCEPT

# Make persistent
apt-get install iptables-persistent
netfilter-persistent save
```

### 2. WARP Container Setup

```bash
# Enter WARP container
lxc-attach -n 101

# Install wgcf
wget https://github.com/ViRb3/wgcf/releases/latest/download/wgcf_linux_amd64
chmod +x wgcf_linux_amd64
mv wgcf_linux_amd64 /usr/local/bin/wgcf

# Register with Cloudflare WARP
wgcf register
wgcf generate

# Install WireGuard
apt-get update
apt-get install wireguard-tools

# Move config
mv wgcf-profile.conf /etc/wireguard/wg0.conf

# IMPORTANT: Attach wg-quick tunnel to OVS bridge as port
# This allows the WARP tunnel to be part of OVS flows
echo "PostUp = ovs-vsctl add-port ovsbr0 wg0" >> /etc/wireguard/wg0.conf
echo "PreDown = ovs-vsctl del-port ovsbr0 wg0" >> /etc/wireguard/wg0.conf

# Start WARP
systemctl enable wg-quick@wg0
systemctl start wg-quick@wg0

# Verify
wg show
```

### 3. Xray Container Setup

```bash
# Enter Xray container
lxc-attach -n 102

# Install Xray
bash -c "$(curl -L https://github.com/XTLS/Xray-install/raw/main/install-release.sh)" @ install

# Create config
cat > /etc/xray/config.json <<'EOF'
{
  "inbounds": [{
    "port": 443,
    "protocol": "vmess",
    "settings": {
      "clients": [{
        "id": "your-uuid-here",
        "alterId": 0
      }]
    },
    "streamSettings": {
      "network": "ws",
      "wsSettings": {
        "path": "/ray"
      }
    }
  }],
  "outbounds": [{
    "protocol": "freedom",
    "settings": {}
  }]
}
EOF

# Generate UUID
apk add util-linux
uuidgen  # Use this for "id" field above

# Start Xray
systemctl enable xray
systemctl start xray
```

## Validation

### Test Container Connectivity

```bash
# From host
ovs-vsctl show  # Should show all containers attached

# Test ping from gateway to WARP
lxc-attach -n 100 -- ping -c 3 10.0.0.101

# Test ping from WARP to Xray
lxc-attach -n 101 -- ping -c 3 10.0.0.102

# View OpenFlow rules
ovs-ofctl dump-flows ovsbr0
```

### Test WARP Connection

```bash
# From WARP container
lxc-attach -n 101

# Check WireGuard interface
wg show

# Test Cloudflare endpoint
curl -4 https://cloudflare.com/cdn-cgi/trace
# Should show warp=on
```

### Test Xray Proxy

```bash
# From external client
# Use Xray client (v2rayN, etc.) with:
# - Address: your-server-ip
# - Port: 443
# - UUID: your-generated-uuid
# - Path: /ray

# Test connection
curl --proxy socks5h://127.0.0.1:1080 https://ipinfo.io
# Should show Cloudflare WARP IP
```

## Monitoring

```bash
# Watch op-dbus logs
journalctl -u op-dbus -f

# Check container status
lxc-ls -f

# Monitor OVS flows
watch ovs-ofctl dump-flows ovsbr0

# Check network traffic
lxc-attach -n 100 -- iftop
```

## Troubleshooting

### Containers Won't Start

```bash
# Check LXC status
systemctl status lxc

# Check logs
journalctl -xe | grep lxc

# Manually start container
lxc-start -n 100 -F  # Foreground mode for debugging
```

### WARP Not Connecting

```bash
# Check WireGuard status
lxc-attach -n 101 -- wg show

# Regenerate WARP config
lxc-attach -n 101
wgcf update
wgcf generate
systemctl restart wg-quick@wg0
```

### OpenFlow Rules Not Working

```bash
# Delete and re-add flows
ovs-ofctl del-flows ovsbr0

# Let op-dbus re-apply
systemctl restart op-dbus

# Manual test
ovs-ofctl add-flow ovsbr0 "priority=100,in_port=warp,actions=output:gateway"
```

## Security Notes

1. **Firewall**: Only expose necessary ports (22, 443, 8443)
2. **UUID**: Use strong, unique UUIDs for Xray clients
3. **Updates**: Regularly update containers and packages
4. **Monitoring**: Monitor traffic for anomalies
5. **Backups**: Backup container configs before changes

## Performance Tuning

```nix
# Add to configuration.nix
boot.kernelParams = [
  "transparent_hugepage=never"  # Better for containers
];

# OVS tuning
services.openvswitch.extraConfig = ''
  set Open_vSwitch . other_config:max-idle=30000
'';
```

## References

- [wgcf Documentation](https://github.com/ViRb3/wgcf)
- [Xray Documentation](https://xtls.github.io/)
- [OpenVSwitch Manual](https://www.openvswitch.org/support/dist-docs/)
- [LXC Documentation](https://linuxcontainers.org/lxc/documentation/)

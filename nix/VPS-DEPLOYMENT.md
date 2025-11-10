# VPS Deployment Guide - Testing NixOS op-dbus Full Mode

Complete step-by-step guide to test the NixOS op-dbus configuration on your VPS at 80.209.240.244.

## Prerequisites

**VPS Info:**
- IP: `80.209.240.244`
- Gateway: `80.209.240.129`
- Network: `80.209.240.244/25`
- SSH Key: `~/.ssh/ghostbridge_key`

**What we're testing:**
- Fresh NixOS installation
- op-dbus Full (Proxmox) mode
- OpenVSwitch bridges (ovsbr0 + mesh)
- LXC container support
- Declarative configuration (no install scripts)

## Option 1: NixOS Already Installed (Quick Test)

If your VPS already has NixOS installed:

### Step 1: Connect to VPS

```bash
# From your local machine
ssh -i ~/.ssh/ghostbridge_key root@80.209.240.244
```

### Step 2: Download op-dbus Configuration

```bash
# On the VPS
cd /tmp
curl -L -o operation-dbus.tar.gz https://github.com/repr0bated/operation-dbus/archive/refs/heads/claude/install-script-gap-dev-011CUupgDV45F7ABCw7aMNhx.tar.gz
tar xzf operation-dbus.tar.gz
cd operation-dbus-claude-install-script-gap-dev-011CUupgDV45F7ABCw7aMNhx
```

### Step 3: Check nix Files

```bash
ls -la nix/
# Should show:
# - DEPENDENCIES.md
# - PROXMOX.md
# - README.md
# - flake.nix
# - module.nix
# - package.nix
```

### Step 4: Create NixOS Configuration

```bash
# Backup existing config
sudo cp /etc/nixos/configuration.nix /etc/nixos/configuration.nix.backup

# Edit configuration
sudo nano /etc/nixos/configuration.nix
```

Add this configuration:

```nix
{ config, pkgs, ... }:

{
  imports = [
    ./hardware-configuration.nix
    /tmp/operation-dbus-claude-install-script-gap-dev-011CUupgDV45F7ABCw7aMNhx/nix/module.nix
  ];

  # Basic system
  boot.loader.grub.enable = true;
  boot.loader.grub.device = "/dev/vda";  # Adjust for your disk

  networking = {
    hostName = "ghostbridge-vps";
    # Let op-dbus manage the network via OVS
    useDHCP = false;
    # Firewall
    firewall = {
      enable = true;
      allowedTCPPorts = [ 22 9573 9574 ];  # SSH, MCP, Web UI
      trustedInterfaces = [ "ovsbr0" "mesh" ];
    };
  };

  # Enable OpenVSwitch
  virtualisation.vswitch = {
    enable = true;
    resetOnStart = false;
  };

  systemd.services.openvswitch = {
    enable = true;
    wantedBy = [ "multi-user.target" ];
  };

  # Enable LXC containers
  virtualisation.lxc = {
    enable = true;
    lxcfs.enable = true;
  };

  # Enable D-Bus (default in NixOS)
  services.dbus.enable = true;

  # Enable op-dbus in Full (Proxmox) mode
  services.op-dbus = {
    enable = true;
    mode = "full";

    stateConfig = {
      # Network configuration
      net = {
        interfaces = [
          # Main bridge with public IP
          {
            name = "ovsbr0";
            type = "ovs-bridge";
            ports = [ "ens1" ];  # Your physical interface
            ipv4 = {
              enabled = true;
              dhcp = false;
              address = [
                { ip = "80.209.240.244"; prefix = 25; }
              ];
              gateway = "80.209.240.129";
            };
          }
          # Mesh bridge for containers
          {
            name = "mesh";
            type = "ovs-bridge";
            ports = [];
            ipv4 = {
              enabled = true;
              dhcp = false;
              address = [
                { ip = "10.0.0.1"; prefix = 24; }
              ];
            };
          }
        ];
      };

      # Container configuration (start with empty, add later)
      lxc = {
        containers = [];
      };

      # Systemd services
      systemd = {
        units = {
          "openvswitch.service" = {
            enabled = true;
            active_state = "active";
          };
        };
      };
    };

    enableBlockchain = true;
    enableCache = true;
  };

  # System packages
  environment.systemPackages = with pkgs; [
    openvswitch
    lxc
    vim
    git
    curl
    htop
    tcpdump
  ];

  # Enable SSH
  services.openssh = {
    enable = true;
    settings.PermitRootLogin = "yes";
  };

  # Nix settings
  nix.settings.experimental-features = [ "nix-command" "flakes" ];

  system.stateVersion = "24.05";
}
```

### Step 5: Apply Configuration

```bash
# Test the configuration first (doesn't activate)
sudo nixos-rebuild test --show-trace

# If test succeeds, apply permanently
sudo nixos-rebuild switch --show-trace
```

**This will:**
1. Install OpenVSwitch
2. Install LXC
3. Build op-dbus from source
4. Create OVS bridges (ovsbr0, mesh)
5. Configure network with your public IP
6. Start op-dbus service

### Step 6: Verify Installation

```bash
# Check op-dbus service
systemctl status op-dbus

# Check OpenVSwitch
systemctl status openvswitch
ovs-vsctl show

# Should show:
# Bridge "ovsbr0"
#     Port "ens1"
#     Port "ovsbr0"
# Bridge "mesh"
#     Port "mesh"

# Check network connectivity
ping -c 3 google.com
ip addr show ovsbr0

# Check op-dbus state
op-dbus query

# Check web UI (from your machine)
curl http://80.209.240.244:9574/api/query
```

### Step 7: Test Container Creation

Add a test container to configuration:

```bash
sudo nano /etc/nixos/configuration.nix
```

Update the `lxc.containers` section:

```nix
lxc = {
  containers = [
    {
      id = "100";
      name = "test-gateway";
      veth = "vi100";
      bridge = "mesh";
      running = true;
    }
  ];
};
```

Apply:

```bash
sudo nixos-rebuild switch
```

Verify:

```bash
# Check container created
lxc-ls -f

# Check attached to mesh bridge
ovs-vsctl list-ports mesh
# Should show: vi100

# Attach to container
lxc-attach -n test-gateway
  # Inside container
  ip addr
  ping 10.0.0.1  # Host mesh IP
  exit
```

## Option 2: Fresh NixOS Installation

If VPS doesn't have NixOS yet, follow the installation guide:

### Step 1: Boot NixOS Installer

See `docs/nixos-ghostbridge-install-guide.md` for full instructions:

1. Add netboot.xyz to GRUB
2. Reboot and select NixOS installer
3. Partition disk
4. Generate initial config

### Step 2: Download op-dbus During Installation

```bash
# In NixOS installer
curl -L -o /mnt/tmp/op-dbus.tar.gz https://github.com/repr0bated/operation-dbus/archive/refs/heads/claude/install-script-gap-dev-011CUupgDV45F7ABCw7aMNhx.tar.gz
cd /mnt/tmp
tar xzf op-dbus.tar.gz
```

### Step 3: Configure Before Install

```bash
# Generate hardware config
nixos-generate-config --root /mnt

# Edit configuration
nano /mnt/etc/nixos/configuration.nix
# (Use the configuration from Option 1, Step 4)
```

### Step 4: Install

```bash
nixos-install
reboot
```

## Testing Full Deployment

Once NixOS is running with op-dbus:

### Test 1: Basic Functionality

```bash
# Service running
systemctl status op-dbus

# Bridges created
ovs-vsctl show

# State query
op-dbus query
```

### Test 2: Container Deployment

```bash
# Add container to configuration.nix
# Apply with nixos-rebuild switch

# Verify container
lxc-ls -f
lxc-attach -n container-name
```

### Test 3: Network Connectivity

```bash
# External connectivity
ping google.com

# Bridge IPs correct
ip addr show ovsbr0
ip addr show mesh

# SSH from external
ssh root@80.209.240.244  # Should work
```

### Test 4: State Changes

Edit `/etc/nixos/configuration.nix`, add another container:

```nix
{
  id = "101";
  name = "warp";
  veth = "vi101";
  bridge = "mesh";
  running = true;
}
```

Apply:

```bash
sudo nixos-rebuild switch
```

Verify declarative behavior:
```bash
# Both containers should exist
lxc-ls -f

# Both veths on mesh
ovs-vsctl list-ports mesh
```

### Test 5: Rollback

```bash
# List generations
nixos-rebuild list-generations

# Rollback to previous
sudo nixos-rebuild --rollback

# Verify container 101 is gone
lxc-ls -f

# Re-apply to come forward
sudo nixos-rebuild switch
```

## Comparison: Before vs After

### Before (Traditional Install)

```bash
# Manual steps
./install-dependencies.sh
cargo build --release
./install.sh --full
# Edit /etc/op-dbus/state.json manually
systemctl start op-dbus
```

### After (NixOS Declarative)

```bash
# One file: /etc/nixos/configuration.nix
services.op-dbus = {
  enable = true;
  mode = "full";
  stateConfig = { ... };
};

# One command
sudo nixos-rebuild switch
```

## Troubleshooting

### Build Takes Forever

```bash
# First build downloads and compiles everything
# Can take 15-30 minutes

# Check it's actually working
top
# Look for: rustc, cc1 processes

# Use build logs
sudo nixos-rebuild switch --show-trace --print-build-logs
```

### Network Lost After Apply

```bash
# Check OVS bridge has correct IP
ip addr show ovsbr0

# Manually fix if needed
sudo ip addr add 80.209.240.244/25 dev ovsbr0
sudo ip route add default via 80.209.240.129

# Then fix configuration.nix and reapply
```

### Service Won't Start

```bash
# Check logs
journalctl -u op-dbus -f

# Check dependencies
systemctl status openvswitch
systemctl status dbus

# Manual start with debug
sudo op-dbus --help
```

### Container Creation Fails

```bash
# Check LXC is enabled
lxc-checkconfig

# Check templates
ls /var/lib/lxc/

# Manual container creation
lxc-create -n test -t download

# Check kernel support
uname -r
```

## Success Criteria

✅ **System deployed successfully when:**

1. `systemctl status op-dbus` shows active (running)
2. `ovs-vsctl show` displays both bridges
3. `ping google.com` works
4. `op-dbus query` returns state
5. `lxc-ls -f` shows declared containers
6. Can SSH to 80.209.240.244
7. Web UI accessible at http://80.209.240.244:9574
8. Configuration changes via `nixos-rebuild switch` work
9. Can rollback with `nixos-rebuild --rollback`

## Next Steps After Success

1. **Add real containers** - Gateway, Warp, XRay
2. **Configure Netmaker** - Join mesh network
3. **Enable MCP server** - For remote management
4. **Set up monitoring** - Grafana, Prometheus
5. **Backup configuration** - Git repo with configuration.nix

## Files Reference

**On VPS after deployment:**
- `/etc/nixos/configuration.nix` - Your declarative config
- `/etc/op-dbus/state.json` - Generated from stateConfig
- `/var/lib/op-dbus/` - Data directory
- `/nix/store/.../bin/op-dbus` - Binary (managed by Nix)

**From archive:**
- `nix/module.nix` - NixOS service module
- `nix/PROXMOX.md` - Full deployment guide
- `nix/DEPENDENCIES.md` - What each component does

## Support

If issues occur:
1. Check `journalctl -u op-dbus -f`
2. Review `nix/DEPENDENCIES.md` for requirements
3. Compare your config to examples in `nix/PROXMOX.md`
4. Use `nixos-rebuild --rollback` to recover

The beauty of NixOS: if something breaks, you can always rollback!

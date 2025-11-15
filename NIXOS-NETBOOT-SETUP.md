# NixOS Network Boot Setup Guide

Complete guide for setting up PXE/network boot for NixOS systems with operation-dbus.

## What is Network Boot?

**Network boot (PXE boot)** allows you to boot NixOS systems from a central server without local storage. Perfect for:
- Diskless workstations
- Rapid deployment of multiple identical systems
- Testing configurations before committing to disk
- Disaster recovery
- Container hosts that need quick redeployment

`★ Insight ─────────────────────────────────────`
**Network Boot Architecture**:
1. **DHCP** assigns IP and points to boot server
2. **TFTP** serves iPXE bootloader (small, fast)
3. **iPXE** downloads menu and displays boot options
4. **HTTP** serves NixOS kernel + initrd (faster than TFTP)
5. **System boots** with operation-dbus configuration
`─────────────────────────────────────────────────`

## Prerequisites

- NixOS server with two network interfaces (or VLANs)
- Network switch supporting PXE boot
- Client machines with PXE-capable NICs
- Basic understanding of networking (DHCP, IP addressing)

## Quick Start (5 Minutes)

```bash
# 1. Clone operation-dbus
cd /etc/nixos
git clone https://github.com/repr0bated/operation-dbus.git

# 2. Add netboot server to configuration
cat >> /etc/nixos/configuration.nix <<'EOF'
{
  imports = [
    ./operation-dbus/nixos/netboot/netboot-server.nix
  ];
}
EOF

# 3. Customize network settings (edit these!)
sudo vim operation-dbus/nixos/netboot/netboot-server.nix
# Change: netbootServerIP, netbootSubnet, etc.

# 4. Apply configuration
sudo nixos-rebuild switch

# 5. Generate netboot images
sudo netboot-generate proxmox-host
sudo netboot-generate workstation

# 6. Update boot menu
sudo netboot-update-menu

# 7. Boot a client machine via PXE
# Set BIOS to network boot, power on, select configuration
```

## Detailed Setup

### Step 1: Install Netboot Server

#### Option A: Using Flakes

```nix
# /etc/nixos/flake.nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    operation-dbus.url = "github:repr0bated/operation-dbus";
  };

  outputs = { self, nixpkgs, operation-dbus }: {
    nixosConfigurations.netboot-server = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [
        operation-dbus.nixosModules.netboot-server
        ./hardware-configuration.nix
      ];
    };
  };
}
```

#### Option B: Traditional NixOS

```bash
# Clone repository
cd /etc/nixos
git clone https://github.com/repr0bated/operation-dbus.git

# Add to configuration.nix
{
  imports = [
    ./operation-dbus/nixos/netboot/netboot-server.nix
  ];
}
```

### Step 2: Customize Network Settings

Edit `operation-dbus/nixos/netboot/netboot-server.nix`:

```nix
let
  # CUSTOMIZE THESE VALUES
  netbootInterface = "eth1";            # Network interface for PXE
  netbootSubnet = "192.168.100.0/24";   # Your PXE subnet
  netbootServerIP = "192.168.100.1";    # This server's IP
  netbootDHCPStart = "192.168.100.100"; # DHCP range start
  netbootDHCPEnd = "192.168.100.200";   # DHCP range end
  netbootGateway = "192.168.100.1";     # Gateway for clients
in
```

**Important**: Use a separate network interface or VLAN for netboot to avoid conflicts with existing DHCP servers!

### Step 3: Apply Server Configuration

```bash
# Check configuration syntax
sudo nixos-rebuild dry-build

# Apply configuration
sudo nixos-rebuild switch

# Verify services are running
sudo systemctl status dnsmasq nginx

# Check logs
sudo journalctl -u dnsmasq -u nginx -f
```

### Step 4: Customize Netboot Configurations

Create configurations for your target systems:

#### For Proxmox Hosts:

```bash
# Edit proxmox-host configuration
sudo vim /etc/nixos/operation-dbus/nixos/netboot/configs/proxmox-host.nix
```

Key settings to customize:

```nix
{
  # NUMA configuration (check with: numactl --hardware)
  services.operation-dbus.numa = {
    enable = true;
    node = 0;           # Your NUMA node
    cpuList = "0-7";    # Your CPU cores
  };

  # ML configuration
  services.operation-dbus.ml = {
    executionProvider = "cuda";  # or "cpu"
    gpuDeviceId = 0;
  };

  # SSH key (REQUIRED - replace with your key!)
  users.users.root.openssh.authorizedKeys.keys = [
    "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAYour-Actual-Key-Here"
  ];

  # Infrastructure state
  services.operation-dbus.defaultState = {
    plugins = {
      lxc = {
        containers = [
          {
            id = "100";
            hostname = "web-server";
            # ... your containers ...
          }
        ];
      };
    };
  };
}
```

#### For Workstations:

```bash
# Edit workstation configuration
sudo vim /etc/nixos/operation-dbus/nixos/netboot/configs/workstation.nix
```

### Step 5: Generate Netboot Images

```bash
# Generate images for each configuration
sudo netboot-generate proxmox-host
sudo netboot-generate workstation

# Verify images were created
ls -lh /var/lib/netboot/http/images/

# Expected output:
# proxmox-host/
#   ├── bzImage      (~10MB - Linux kernel)
#   ├── initrd       (~50MB - Initial RAM disk)
#   └── SHA256SUMS   (Checksums)
# workstation/
#   ├── bzImage
#   ├── initrd
#   └── SHA256SUMS
```

**What these files are**:
- `bzImage`: Compressed Linux kernel with all drivers
- `initrd`: Initial RAM disk containing NixOS system closure
- `SHA256SUMS`: Checksums for verification

### Step 6: Create SSH Keys for Root Access

```bash
# Generate SSH key if you don't have one
ssh-keygen -t ed25519 -f ~/.ssh/netboot_root

# Get the public key
cat ~/.ssh/netboot_root.pub

# Add to each configuration file:
users.users.root.openssh.authorizedKeys.keys = [
  "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAA... (paste your key here)"
];

# Regenerate images after adding keys
sudo netboot-generate proxmox-host
sudo netboot-generate workstation
```

### Step 7: Update Boot Menu

```bash
# Generate iPXE boot menu
sudo netboot-update-menu

# Verify menu was created
cat /var/lib/netboot/configs/boot.ipxe

# Expected: iPXE script with menu options
```

### Step 8: Test Network Boot

#### Configure Client Machine BIOS:

1. **Enter BIOS/UEFI** (usually F2, F12, or DEL during boot)
2. **Enable Network Boot** (PXE boot)
3. **Set Boot Order**: Network boot first
4. **Save and Exit**

#### Boot the Client:

```
Power on → PXE boot starts → DHCP → Download iPXE → Show menu

╔══════════════════════════════════════╗
║     NixOS Network Boot Menu          ║
╠══════════════════════════════════════╣
║ Available Configurations:            ║
║                                      ║
║ [1] Proxmox Host (Multi-socket Xeon) ║
║ [2] Workstation (Single-socket)      ║
║ [3] NixOS Installer (Live)           ║
║ [4] iPXE Shell (Debugging)           ║
║                                      ║
║ Auto-boot in 10 seconds...           ║
╚══════════════════════════════════════╝

Select option → Download kernel/initrd → Boot NixOS!
```

#### Connect via SSH:

```bash
# Find the client's IP (check DHCP server logs)
sudo journalctl -u dnsmasq | grep DHCPACK

# Example output:
# DHCPACK(eth1) 192.168.100.150 52:54:00:12:34:56 netboot-client

# SSH into the client
ssh -i ~/.ssh/netboot_root root@192.168.100.150

# Verify operation-dbus is running
systemctl status operation-dbus

# Check NUMA configuration (if enabled)
numactl --hardware
numastat -p $(pgrep op-dbus)

# Check containers/services
op-dbus query
```

## Command Reference

### Server Management

```bash
# Generate netboot image for a configuration
sudo netboot-generate <config-name>
# Example: sudo netboot-generate proxmox-host

# Update iPXE boot menu
sudo netboot-update-menu

# Check server status
sudo netboot-status
# Equivalent to: systemctl status dnsmasq nginx

# View server logs
sudo netboot-logs
# Equivalent to: journalctl -u dnsmasq -u nginx -f

# List generated images
ls -lh /var/lib/netboot/http/images/

# Check DHCP leases
sudo journalctl -u dnsmasq | grep DHCPACK

# Test TFTP server
tftp 192.168.100.1 -c get ipxe.efi /tmp/test.efi

# Test HTTP server
curl http://192.168.100.1/boot.ipxe
```

### Image Management

```bash
# Build image without installing
nix-build '<nixpkgs/nixos>' -A config.system.build.netbootRamdisk \
  -I nixos-config=/etc/nixos/operation-dbus/nixos/netboot/configs/proxmox-host.nix

# Check image size
du -h /var/lib/netboot/http/images/*/

# Verify checksums
cd /var/lib/netboot/http/images/proxmox-host
sha256sum -c SHA256SUMS

# Delete old images
rm -rf /var/lib/netboot/http/images/old-config/
```

### Client Management

```bash
# After client boots, connect via SSH
ssh -i ~/.ssh/netboot_root root@<client-ip>

# On client: Check operation-dbus status
systemctl status operation-dbus
op-dbus query

# On client: Check NUMA configuration
numactl --hardware
numastat -p $(pgrep op-dbus)

# On client: View logs
journalctl -u operation-dbus -f

# On client: Benchmark performance
op-dbus benchmark --all
```

## Network Topology Examples

### Example 1: Simple Lab Setup

```
┌─────────────────┐
│  Netboot Server │  192.168.100.1
│  (NixOS)        │
└────────┬────────┘
         │ eth1 (PXE network)
         │
    ┌────┴─────────────────┐
    │    Network Switch     │
    └────┬────┬────┬────────┘
         │    │    │
    ┌────┴┐ ┌─┴───┐ ┌──┴────┐
    │ PXE │ │ PXE │ │  PXE  │
    │ #1  │ │ #2  │ │  #3   │
    └─────┘ └─────┘ └───────┘
    .100    .101     .102
```

### Example 2: Production with VLANs

```
┌─────────────────┐
│  Netboot Server │
│  eth0: 10.0.0.1   (Management VLAN 10)
│  eth1: 192.168.100.1 (PXE VLAN 100)
└────────┬────────┘
         │
    ┌────┴────────┐
    │ L3 Switch   │
    │ VLAN 10, 100│
    └────┬────────┘
         │
    ┌────┴─────────────┐
    │ Access Switches   │
    │ VLAN 100 (PXE)   │
    └┬────┬────┬───────┘
     │    │    │
  Proxmox Nodes (PXE boot)
```

## Troubleshooting

### Client Can't Get DHCP

```bash
# On server: Check dnsmasq is running
sudo systemctl status dnsmasq

# Check dnsmasq config
cat /etc/dnsmasq.conf

# View DHCP requests
sudo journalctl -u dnsmasq -f
# Should see: DHCPDISCOVER, DHCPOFFER, DHCPREQUEST, DHCPACK

# Test DHCP from another machine
sudo nmap --script broadcast-dhcp-discover -e eth1

# Common issues:
# 1. Wrong interface: Check netbootInterface setting
# 2. Firewall blocking: Check firewall.allowedUDPPorts
# 3. Existing DHCP: Disable other DHCP servers on network
```

### Client Can't Download iPXE

```bash
# On server: Check TFTP is enabled
sudo systemctl status dnsmasq
# Should show: enable-tftp=true

# Check TFTP root exists
ls -la /var/lib/netboot/tftp/
# Should contain: ipxe.efi, undionly.kpxe

# Test TFTP manually
tftp localhost -c get ipxe.efi /tmp/test.efi
echo $?  # Should be 0

# Common issues:
# 1. Missing symlinks: Check systemd.tmpfiles.rules
# 2. Wrong permissions: Should be readable by dnsmasq
# 3. Firewall: Check UDP port 69 is open
```

### Client Can't Download Kernel/Initrd

```bash
# On server: Check nginx is running
sudo systemctl status nginx

# Check HTTP server is accessible
curl http://192.168.100.1/boot.ipxe
curl -I http://192.168.100.1/images/proxmox-host/bzImage

# Common issues:
# 1. Images not generated: Run sudo netboot-generate
# 2. Wrong permissions: Check /var/lib/netboot/http/ permissions
# 3. Firewall: Check TCP port 80 is open
# 4. Wrong IP in boot.ipxe: Check netbootServerIP matches
```

### Client Boots but Can't Find Init

```bash
# Error: "Cannot find init= parameter"

# On server: Check image was built correctly
ls -lh /var/lib/netboot/http/images/proxmox-host/
# Should see bzImage and initrd

# Rebuild image with verbose output
nix-build '<nixpkgs/nixos>' -A config.system.build.netbootRamdisk \
  -I nixos-config=/etc/nixos/operation-dbus/nixos/netboot/configs/proxmox-host.nix \
  --show-trace

# Common issues:
# 1. Configuration error: Check nixos config file syntax
# 2. Missing modules: Check imports in configuration
# 3. initrd corruption: Regenerate image
```

### operation-dbus Not Starting

```bash
# On client (after boot): Check service status
ssh root@192.168.100.150
systemctl status operation-dbus

# View logs
journalctl -u operation-dbus -b

# Common issues:
# 1. D-Bus not available: Check services.dbus.enable = true
# 2. Missing state file: Check stateFile path
# 3. BTRFS not available: Check boot.supportedFilesystems
# 4. NUMA detection failed: Check numa.enable setting
```

### Performance Issues

```bash
# On client: Benchmark operation-dbus
op-dbus benchmark --all

# Check NUMA is working
numastat -p $(pgrep op-dbus)
# Memory should be on local node

# Check CPU affinity
taskset -pc $(pgrep op-dbus)
# Should show configured CPU list

# Check ML execution provider
journalctl -u operation-dbus | grep "ML_PROVIDER"

# If slow:
# 1. NUMA not enabled: Check numa.enable = true
# 2. Wrong CPU affinity: Check cpuList matches your hardware
# 3. GPU not detected: Check nvidia-smi output
```

## Advanced Configurations

### MAC Address-Based Configuration Selection

Edit `/etc/operation-dbus/netboot.json` to map MAC addresses to configurations:

```json
{
  "netboot": {
    "targets": [
      {
        "name": "proxmox-node-01",
        "mac": "52:54:00:12:34:56",
        "config": "proxmox-host",
        "ip": "192.168.100.101"
      },
      {
        "name": "workstation-05",
        "mac": "52:54:00:12:34:60",
        "config": "workstation",
        "ip": "192.168.100.105"
      }
    ]
  }
}
```

Then update dnsmasq to use MAC-specific configs:

```nix
services.dnsmasq.extraConfig = ''
  # MAC-based configuration selection
  dhcp-host=52:54:00:12:34:56,192.168.100.101,proxmox-node-01
  dhcp-host=52:54:00:12:34:60,192.168.100.105,workstation-05
'';
```

### Hybrid: Netboot + Local Storage

Boot from network initially, then persist to local disk:

```nix
# In netboot configuration, add:
{
  # Install NixOS to local disk after first boot
  systemd.services.install-to-disk = {
    wantedBy = [ "multi-user.target" ];
    after = [ "network.target" ];
    script = ''
      if [ ! -f /mnt/nixos-installed ]; then
        echo "Installing NixOS to /dev/sda..."
        # Partition disk
        parted /dev/sda -- mklabel gpt
        parted /dev/sda -- mkpart ESP fat32 1MiB 512MiB
        parted /dev/sda -- mkpart primary btrfs 512MiB 100%

        # Format partitions
        mkfs.fat -F 32 /dev/sda1
        mkfs.btrfs /dev/sda2

        # Mount and install
        mount /dev/sda2 /mnt
        mkdir -p /mnt/boot
        mount /dev/sda1 /mnt/boot

        nixos-install --root /mnt

        touch /mnt/nixos-installed
        echo "Installation complete. System will reboot from disk next time."
      fi
    '';
  };
}
```

### Monitoring Netboot Infrastructure

```bash
# Monitor DHCP activity
watch -n 1 'sudo journalctl -u dnsmasq --since "1 minute ago" | grep DHCP'

# Monitor HTTP downloads
watch -n 1 'sudo tail -20 /var/log/nginx/access.log | grep bzImage'

# Monitor active clients
nmap -sP 192.168.100.0/24

# Create dashboard script
cat > /usr/local/bin/netboot-dashboard <<'EOF'
#!/usr/bin/env bash
while true; do
  clear
  echo "=== Netboot Server Dashboard ==="
  echo ""
  echo "Services:"
  systemctl is-active dnsmasq nginx | paste <(echo "  dnsmasq nginx") -
  echo ""
  echo "Active DHCP Leases:"
  journalctl -u dnsmasq --since "1 hour ago" | grep DHCPACK | tail -10
  echo ""
  echo "Recent HTTP Downloads:"
  tail -10 /var/log/nginx/access.log | grep -E "(bzImage|initrd)"
  sleep 5
done
EOF
chmod +x /usr/local/bin/netboot-dashboard
```

## Integration with operation-dbus

### Declarative Netboot Management

Use operation-dbus to manage netboot targets declaratively:

```json
{
  "version": "1.0",
  "netboot": {
    "server": {
      "ip": "192.168.100.1",
      "interface": "eth1",
      "dhcp_range": {
        "start": "192.168.100.100",
        "end": "192.168.100.200"
      }
    },
    "targets": [
      {
        "name": "proxmox-node-01",
        "mac": "52:54:00:12:34:56",
        "ip": "192.168.100.101",
        "config": "proxmox-host",
        "state": {
          "plugins": {
            "lxc": {
              "containers": [
                { "id": "100", "hostname": "web-01" }
              ]
            }
          }
        }
      }
    ]
  }
}
```

Apply with:
```bash
op-dbus apply /etc/operation-dbus/netboot.json
```

## Security Considerations

### Network Isolation

```
✓ Use separate VLAN for PXE boot
✓ Firewall rules to prevent PXE network from accessing management
✓ MAC address filtering in DHCP server
✗ Don't expose PXE network to internet
✗ Don't use PXE on untrusted networks
```

### SSH Key Management

```bash
# Use separate keys for netboot environments
ssh-keygen -t ed25519 -f ~/.ssh/netboot_root -C "netboot-root-access"

# Rotate keys regularly
# Add new key to configurations, rebuild images, remove old key

# Use SSH agent forwarding carefully
# Don't forward agent to netboot systems
```

### Image Verification

```bash
# Always verify image checksums
cd /var/lib/netboot/http/images/proxmox-host
sha256sum -c SHA256SUMS

# Sign images (future enhancement)
gpg --detach-sign --armor bzImage
```

## Performance Tuning

### Optimize Image Size

```nix
# In netboot configuration:
{
  # Minimize closure size
  environment.systemPackages = lib.mkForce [
    # Only essential packages
  ];

  # Remove documentation
  documentation.enable = false;
  documentation.nixos.enable = false;

  # Compression
  boot.initrd.compressor = "zstd";
  boot.initrd.compressorArgs = [ "-19" "-T0" ];
}
```

### Optimize Network Transfer

```bash
# Use nginx compression
services.nginx.virtualHosts."netboot".extraConfig = ''
  gzip on;
  gzip_types application/octet-stream;
'';

# Pre-load images on client switches (if supported)
```

### Parallel Image Generation

```bash
# Generate multiple images in parallel
for config in proxmox-host workstation installer; do
  sudo netboot-generate $config &
done
wait
```

## Next Steps

After setting up netboot:

1. **Test thoroughly** - Boot each configuration on test hardware
2. **Document MAC addresses** - Keep a registry of target systems
3. **Automate updates** - Create cron job to regenerate images nightly
4. **Monitor** - Set up alerting for netboot server failures
5. **Scale** - Add multiple netboot servers for redundancy

## Additional Resources

- [iPXE Documentation](https://ipxe.org/docs)
- [NixOS Manual: Netboot](https://nixos.org/manual/nixos/stable/#sec-booting-from-pxe)
- [BTRFS Wiki](https://btrfs.wiki.kernel.org/)
- [operation-dbus Documentation](https://github.com/repr0bated/operation-dbus)

---

**Last Updated**: 2025-01-07
**NixOS Version**: 24.11 (unstable)
**operation-dbus Version**: 0.1.0

# NixOS GhostBridge VPS Installation Guide

Complete guide for installing NixOS on your VPS for GhostBridge privacy router.

## Phase 1: Boot into NixOS Installer

### Step 1: Add netboot.xyz to GRUB (if not already done)

```bash
# On current VPS
mkdir -p /boot/netboot.xyz
cd /boot/netboot.xyz
wget https://boot.netboot.xyz/ipxe/netboot.xyz.lkrn

cat >> /etc/grub.d/40_custom << 'EOF'
menuentry 'netboot.xyz' {
    linux16 /boot/netboot.xyz/netboot.xyz.lkrn
}
EOF

chmod +x /etc/grub.d/40_custom
update-grub
```

### Step 2: Reboot and Select netboot.xyz

```bash
reboot
```

In VPS console:
1. Select **netboot.xyz** from GRUB menu
2. Choose **Linux Network Installs**
3. Select **NixOS**
4. Choose latest stable release (24.05 or newer)

## Phase 2: NixOS Installation (In Installer)

### Step 1: Network Configuration

```bash
# Verify internet connectivity
ping -c 3 google.com

# If no internet, configure manually:
# ip addr add 80.209.240.244/24 dev eth0
# ip route add default via YOUR.GATEWAY.IP
# echo "nameserver 8.8.8.8" > /etc/resolv.conf
```

### Step 2: Partition Disk

**Warning**: This will ERASE all data!

```bash
# Find your disk
lsblk

# Assuming /dev/vda (change if different)
DISK=/dev/vda

# Partition (simple single partition)
parted $DISK -- mklabel msdos
parted $DISK -- mkpart primary 1MiB 100%
parted $DISK -- set 1 boot on

# Format
mkfs.ext4 -L nixos ${DISK}1

# Mount
mount /dev/disk/by-label/nixos /mnt
```

**For UEFI systems:**
```bash
# UEFI partitioning
parted $DISK -- mklabel gpt
parted $DISK -- mkpart ESP fat32 1MiB 512MiB
parted $DISK -- set 1 esp on
parted $DISK -- mkpart primary 512MiB 100%

# Format
mkfs.fat -F 32 -n boot ${DISK}1
mkfs.ext4 -L nixos ${DISK}2

# Mount
mount /dev/disk/by-label/nixos /mnt
mkdir -p /mnt/boot
mount /dev/disk/by-label/boot /mnt/boot
```

### Step 3: Generate Base Configuration

```bash
# Generate hardware config
nixos-generate-config --root /mnt

# This creates:
# /mnt/etc/nixos/configuration.nix
# /mnt/etc/nixos/hardware-configuration.nix
```

### Step 4: Download GhostBridge Configuration

```bash
# Download our prepared config
curl -L https://raw.githubusercontent.com/repr0bated/operation-dbus/claude/install-script-gap-dev-011CUupgDV45F7ABCw7aMNhx/docs/examples/nixos-ghostbridge-vps.nix \
  -o /mnt/etc/nixos/configuration.nix

# Or manually create it (see below)
```

**Manual configuration.nix** (if download fails):

```bash
nano /mnt/etc/nixos/configuration.nix
```

Paste the configuration from `/tmp/nixos-ghostbridge-vps-configuration.nix`

**Important edits:**
1. Change `boot.loader.grub.device` to match your disk
2. Add your SSH public key to `users.users.root.openssh.authorizedKeys.keys`
3. Verify network interface name (eth0, ens3, etc.)

### Step 5: Install NixOS

```bash
# Install
nixos-install

# Set root password when prompted
# (You can skip if using SSH keys only)

# Reboot
reboot
```

## Phase 3: Post-Installation (After Reboot)

### Step 1: SSH into New NixOS System

```bash
# From your machine
ssh root@80.209.240.244
```

### Step 2: Install op-dbus

```bash
# Clone repo
cd /root
git clone https://github.com/repr0bated/operation-dbus.git
cd operation-dbus
git checkout claude/install-script-gap-dev-011CUupgDV45F7ABCw7aMNhx

# Build with Nix
nix build .#op-dbus

# Install to system
cp result/bin/op-dbus /usr/local/bin/
chmod +x /usr/local/bin/op-dbus

# Verify
op-dbus --version
```

### Step 3: Initialize op-dbus

```bash
# Create directories
mkdir -p /var/lib/op-dbus
mkdir -p /etc/op-dbus

# Create privacy-vps state file
cat > /etc/op-dbus/state.json << 'EOF'
{
  "version": 1,
  "plugins": {
    "lxc": {
      "containers": [
        {
          "id": 100,
          "name": "xray-server",
          "template": "debian-12",
          "autostart": true,
          "network": {
            "bridge": "ovsbr0",
            "socket_networking": true,
            "port_name": "internal_100",
            "ipv4": "10.0.0.100/24"
          }
        }
      ]
    },
    "openflow": {
      "enable_security_flows": true,
      "obfuscation_level": 2,
      "bridges": [
        {
          "name": "ovsbr0",
          "datapath_type": "netdev",
          "socket_ports": [
            {"name": "internal_100", "container_id": "100"}
          ]
        }
      ]
    }
  }
}
EOF
```

### Step 4: Start Services

```bash
# Reload systemd (services defined in configuration.nix)
systemctl daemon-reload

# Start op-dbus
systemctl start op-dbus
systemctl start op-dbus-webui

# Check status
systemctl status op-dbus
systemctl status op-dbus-webui

# Enable on boot
systemctl enable op-dbus
systemctl enable op-dbus-webui
```

### Step 5: Verify Installation

```bash
# Check web UI (from your machine)
curl http://80.209.240.244:9574/api/query

# Apply state
op-dbus apply --state-file /etc/op-dbus/state.json

# Verify containers
op-dbus query
```

## Phase 4: GhostBridge Deployment

Once op-dbus is running, deploy containers:

```bash
# Apply privacy-vps profile
op-dbus apply --state-file /etc/op-dbus/state.json

# Verify OVS bridge
ovs-vsctl show

# Check containers
lxc-ls -f
```

## Troubleshooting

### No Internet After Install

```bash
# Check network
ip addr show
ip route show

# Restart networking
systemctl restart systemd-networkd
```

### op-dbus Service Fails

```bash
# Check logs
journalctl -u op-dbus -f

# Run manually for debugging
/usr/local/bin/op-dbus run
```

### Can't Access Web UI

```bash
# Check firewall
nix-shell -p iptables --run "iptables -L -n"

# Verify port listening
ss -tlnp | grep 9574
```

## Rollback if Needed

NixOS makes rollback easy:

```bash
# List generations
nix-env --list-generations

# Rollback to previous
nixos-rebuild switch --rollback

# Or boot into previous generation from GRUB menu
```

## Next Steps After Installation

1. ✅ NixOS installed and booted
2. ✅ op-dbus running
3. ✅ Web UI accessible
4. → Deploy containers (privacy-vps profile)
5. → Install XRay in container 100
6. → Configure GhostBridge on castlebox
7. → Test full privacy chain

---

**Ready!** Boot into netboot.xyz and follow this guide. Let me know when you reach each phase! 🚀

# netboot.xyz Integration for operation-dbus

Quick guide for adding operation-dbus NixOS configurations to your existing netboot.xyz setup.

`★ Insight ─────────────────────────────────────`
**netboot.xyz Simplifies Setup**: Since you already have netboot.xyz
running, you just need to:
1. Host NixOS images on HTTP server
2. Add custom boot entries to netboot.xyz menu
3. Boot and configure via operation-dbus
`─────────────────────────────────────────────────`

## Prerequisites

- netboot.xyz already running (you have this ✓)
- HTTP server to host NixOS images
- NixOS build machine (can be same as HTTP server)

## Quick Setup (3 Commands)

```bash
# 1. Build NixOS netboot images
nix-build -A netboot-proxmox -o /tmp/proxmox

# 2. Copy to HTTP server
scp -r /tmp/proxmox/* user@webserver:/var/www/netboot/

# 3. Add to netboot.xyz custom menu
# (see step 3 below)
```

## Step 1: Build NixOS Images

### Create build configuration

```nix
# /etc/nixos/netboot-build.nix
{ pkgs ? import <nixpkgs> {} }:

{
  # Proxmox host with operation-dbus
  netboot-proxmox = import (pkgs.path + "/nixos") {
    configuration = {
      imports = [
        (pkgs.path + "/nixos/modules/installer/netboot/netboot-minimal.nix")
        ./operation-dbus/nixos/modules/operation-dbus.nix
      ];

      # Netboot-specific
      boot.supportedFilesystems = [ "btrfs" ];
      boot.kernelParams = [
        "boot.shell_on_fail"
        "numa_balancing=enable"
      ];

      # operation-dbus configuration
      services.operation-dbus = {
        enable = true;
        numa = { enable = true; node = 0; cpuList = "0-7"; };
        btrfs.enable = true;
        ml = { enable = true; executionProvider = "cuda"; };
      };

      # SSH access (ADD YOUR KEY!)
      users.users.root.openssh.authorizedKeys.keys = [
        "ssh-ed25519 AAAAC3... your-key-here"
      ];

      services.openssh.enable = true;
      system.stateVersion = "24.11";
    };
  }.config.system.build.netbootRamdisk;

  # Workstation (simpler)
  netboot-workstation = import (pkgs.path + "/nixos") {
    configuration = {
      imports = [
        (pkgs.path + "/nixos/modules/installer/netboot/netboot-minimal.nix")
        ./operation-dbus/nixos/modules/operation-dbus.nix
      ];

      services.operation-dbus = {
        enable = true;
        numa.enable = false;
        ml = { executionProvider = "cpu"; numThreads = 4; };
      };

      users.users.root.openssh.authorizedKeys.keys = [
        "ssh-ed25519 AAAAC3... your-key-here"
      ];

      services.openssh.enable = true;
      system.stateVersion = "24.11";
    };
  }.config.system.build.netbootRamdisk;
}
```

### Build the images

```bash
# Build both configurations
nix-build /etc/nixos/netboot-build.nix -A netboot-proxmox -o /tmp/proxmox
nix-build /etc/nixos/netboot-build.nix -A netboot-workstation -o /tmp/workstation

# Check what was built
ls -lh /tmp/proxmox/
# bzImage  - Kernel (~10MB)
# initrd   - Initial RAM disk (~100MB with operation-dbus)
```

## Step 2: Host Images on HTTP Server

### Option A: Use Existing Web Server

```bash
# Copy to your web server
WEB_ROOT="/var/www/html/netboot"  # Adjust path
mkdir -p $WEB_ROOT/{proxmox,workstation}

cp /tmp/proxmox/bzImage $WEB_ROOT/proxmox/
cp /tmp/proxmox/initrd $WEB_ROOT/proxmox/

cp /tmp/workstation/bzImage $WEB_ROOT/workstation/
cp /tmp/workstation/initrd $WEB_ROOT/workstation/

# Make readable
chmod -R 755 $WEB_ROOT

# Test access
curl -I http://your-server/netboot/proxmox/bzImage
# Should return: HTTP/1.1 200 OK
```

### Option B: Quick Python HTTP Server

```bash
# Serve from /tmp for testing
cd /tmp
python3 -m http.server 8080

# Images available at:
# http://your-ip:8080/proxmox/bzImage
# http://your-ip:8080/proxmox/initrd
```

### Option C: nginx on NixOS

```nix
# Add to your NixOS configuration
{
  services.nginx = {
    enable = true;
    virtualHosts."netboot.local" = {
      root = "/var/www/netboot";
      locations."/".extraConfig = "autoindex on;";
    };
  };

  networking.firewall.allowedTCPPorts = [ 80 ];
}
```

## Step 3: Add to netboot.xyz Menu

### Find your netboot.xyz custom menu

netboot.xyz stores custom menus in `/config/menus` (if using Docker) or you can use their web interface.

### Create custom menu entry

```bash
# Create custom.ipxe (or add to existing)
cat > /path/to/netboot.xyz/custom.ipxe <<'EOF'
#!ipxe

:custom
menu Custom Boot Options
item --gap -- operation-dbus NixOS Configurations:
item nixos-proxmox   Proxmox Host (NUMA + GPU + operation-dbus)
item nixos-workstation Workstation (CPU-only + operation-dbus)
item --gap --
item return Return to main menu
choose custom_choice || goto custom
goto custom_${custom_choice}

:nixos-proxmox
echo Booting NixOS Proxmox Host with operation-dbus...
kernel http://YOUR-SERVER-IP/netboot/proxmox/bzImage init=/nix/store/.../init loglevel=4
initrd http://YOUR-SERVER-IP/netboot/proxmox/initrd
boot

:nixos-workstation
echo Booting NixOS Workstation with operation-dbus...
kernel http://YOUR-SERVER-IP/netboot/workstation/bzImage init=/nix/store/.../init
initrd http://YOUR-SERVER-IP/netboot/workstation/initrd
boot

:return
chain utils.ipxe
boot
EOF
```

**IMPORTANT**: Replace `YOUR-SERVER-IP` with your actual HTTP server IP!

### Integration methods:

#### Method A: Docker netboot.xyz

```bash
# If using netboot.xyz Docker container
docker cp custom.ipxe netboot_xyz:/config/menus/
docker restart netboot_xyz
```

#### Method B: Self-hosted netboot.xyz

```bash
# Copy to your netboot.xyz assets directory
cp custom.ipxe /var/www/netboot.xyz/assets/custom.ipxe

# Update netboot.xyz menu.ipxe to include:
# :custom
# chain custom.ipxe || goto error
```

#### Method C: netboot.xyz Cloud (hosted)

If using netboot.xyz's hosted service:
1. Go to netboot.xyz web interface
2. Navigate to "Custom" menu
3. Add boot entries via web UI
4. Point to your HTTP server URLs

## Step 4: Boot and Test

### Boot a machine

1. **PXE boot** → Select "Custom" from netboot.xyz menu
2. **Choose** "Proxmox Host" or "Workstation"
3. **Wait** for kernel/initrd download (~1-2 minutes)
4. **System boots** with operation-dbus running

### Connect via SSH

```bash
# Get IP from DHCP server logs or scan network
nmap -sP 192.168.1.0/24 | grep -B 2 "Nmap done"

# SSH into the system
ssh root@<discovered-ip>

# Verify operation-dbus
systemctl status operation-dbus
op-dbus query

# Check NUMA (if enabled)
numactl --hardware
numastat -p $(pgrep op-dbus)

# Check ML provider
journalctl -u operation-dbus | grep "ML_PROVIDER"
```

## Complete Example Files

### Minimal custom.ipxe

```ipxe
#!ipxe

:custom
menu operation-dbus NixOS
item nixos-opdbus NixOS with operation-dbus
choose --default nixos-opdbus --timeout 30000 target && goto ${target}

:nixos-opdbus
kernel http://192.168.1.100/netboot/proxmox/bzImage init=/nix/store/.../init console=ttyS0
initrd http://192.168.1.100/netboot/proxmox/initrd
boot
```

### Quick build script

```bash
#!/usr/bin/env bash
# build-netboot.sh - Build and deploy netboot images

set -e

CONFIGS="proxmox workstation"
WEB_ROOT="/var/www/netboot"
BUILD_FILE="/etc/nixos/netboot-build.nix"

for config in $CONFIGS; do
  echo "Building netboot-$config..."
  nix-build "$BUILD_FILE" -A "netboot-$config" -o "/tmp/$config"

  echo "Deploying to $WEB_ROOT/$config..."
  mkdir -p "$WEB_ROOT/$config"
  cp "/tmp/$config/bzImage" "$WEB_ROOT/$config/"
  cp "/tmp/$config/initrd" "$WEB_ROOT/$config/"

  # Generate checksums
  cd "$WEB_ROOT/$config"
  sha256sum bzImage initrd > SHA256SUMS

  echo "✓ $config deployed"
done

echo ""
echo "All images built and deployed!"
echo "Test with: curl -I http://localhost/netboot/proxmox/bzImage"
```

Make it executable:
```bash
chmod +x build-netboot.sh
sudo ./build-netboot.sh
```

## Troubleshooting

### Can't access HTTP server from netboot.xyz

```bash
# Test from another machine
curl -I http://your-server/netboot/proxmox/bzImage

# If fails:
# 1. Check firewall
sudo firewall-cmd --add-port=80/tcp  # Fedora/RHEL
sudo ufw allow 80                     # Ubuntu
# 2. Check nginx/apache is running
systemctl status nginx
# 3. Check file permissions
ls -la /var/www/netboot/
```

### Boot hangs at "Downloading kernel"

```bash
# Large initrd can take time (100MB+)
# Watch progress in netboot.xyz boot screen

# If timeout:
# 1. Increase timeout in iPXE menu
#    choose --timeout 60000 target  # 60 seconds
# 2. Optimize initrd size
#    boot.initrd.compressor = "zstd";
#    boot.initrd.compressorArgs = [ "-19" ];
```

### System boots but operation-dbus fails

```bash
# SSH in and check logs
ssh root@<ip>
journalctl -u operation-dbus -xe

# Common issues:
# 1. Missing D-Bus: services.dbus.enable = true
# 2. No state file: stateFile must exist or use defaultState
# 3. BTRFS unavailable: boot.supportedFilesystems = [ "btrfs" ]
```

### Want to update configuration

```bash
# 1. Edit configuration file
vim /etc/nixos/netboot-build.nix

# 2. Rebuild
nix-build /etc/nixos/netboot-build.nix -A netboot-proxmox -o /tmp/proxmox

# 3. Redeploy
cp /tmp/proxmox/* /var/www/netboot/proxmox/

# 4. Reboot client machines
# They'll get new configuration on next boot
```

## Advanced: Automated Builds

### Nightly rebuild cron job

```bash
# /etc/cron.daily/netboot-rebuild
#!/usr/bin/env bash
cd /etc/nixos/operation-dbus
git pull
nix-build /etc/nixos/netboot-build.nix -A netboot-proxmox -o /tmp/proxmox
cp /tmp/proxmox/* /var/www/netboot/proxmox/
```

### Build on commit hook

```bash
# .git/hooks/post-receive
#!/usr/bin/env bash
GIT_WORK_TREE=/etc/nixos/operation-dbus git checkout -f
cd /etc/nixos/operation-dbus
./build-netboot.sh
```

## Performance Tips

### Reduce initrd size

```nix
{
  # In your netboot configuration:

  # Minimal package set
  environment.systemPackages = lib.mkForce [
    pkgs.operation-dbus
    pkgs.btrfs-progs
  ];

  # No documentation
  documentation.enable = false;

  # Aggressive compression
  boot.initrd.compressor = "zstd";
  boot.initrd.compressorArgs = [ "-19" "-T0" ];
}
```

### Pre-cache images

If using the same configurations frequently, clients will re-download. Consider:
- Using a local cache/proxy
- Running a local mirror
- Using netboot.xyz's caching features

## Integration with operation-dbus State

### Fetch state from HTTP on boot

```nix
{
  # In netboot configuration:
  systemd.services.fetch-opdbus-state = {
    wantedBy = [ "operation-dbus.service" ];
    before = [ "operation-dbus.service" ];
    script = ''
      ${pkgs.curl}/bin/curl -o /etc/operation-dbus/state.json \
        http://your-server/states/$(hostname).json
    '';
  };
}
```

### MAC-based state files

```bash
# On HTTP server:
# /var/www/states/52-54-00-12-34-56.json  # Proxmox node 1
# /var/www/states/52-54-00-12-34-57.json  # Proxmox node 2

# In netboot config:
systemd.services.fetch-opdbus-state.script = ''
  MAC=$(cat /sys/class/net/eth0/address | tr : -)
  curl -o /etc/operation-dbus/state.json \
    http://your-server/states/$MAC.json
'';
```

## Summary: What You Need

```bash
# 1. Build NixOS images with operation-dbus
nix-build -A netboot-proxmox

# 2. Host on HTTP server
cp result/* /var/www/netboot/proxmox/

# 3. Add to netboot.xyz custom menu
# Point to: http://your-server/netboot/proxmox/{bzImage,initrd}

# 4. Boot machines via netboot.xyz
# Select your custom menu → operation-dbus boots!
```

That's it! Much simpler than setting up a full PXE server since netboot.xyz handles the boot infrastructure.

---

**Next Steps**:
1. Build your first image: `nix-build -A netboot-proxmox`
2. Test with Python HTTP server: `python3 -m http.server`
3. Add to netboot.xyz menu
4. Boot and verify!

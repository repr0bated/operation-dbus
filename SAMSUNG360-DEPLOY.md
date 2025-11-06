# Samsung 360 Pro Deployment Guide

## Hardware Profile: Samsung 360 Pro

**Known Issues:**
- Buggy BIOS prevents standard NixOS installer from booting
- Cannot boot graphical installer (tested for 3 days)
- Requires netboot.xyz for system access
- Needs specific kernel parameters to function correctly

**Required Workarounds:**
- Boot parameter: `acpi=off` (bypasses buggy ACPI tables)
- Power management: `intel_idle.max_cstate=1` (prevents CPU state bugs)
- PCIe ASPM: `pcie_aspm=off` (fixes PCI Express power management)
- Intel graphics: `i915.enable_psr=0` (prevents screen flickering)

This laptop represents a **worst-case deployment scenario** - if op-dbus can handle this hardware, it can handle anything.

---

## Prerequisites

### Required Access
- Network boot capability (netboot.xyz)
- Internet connection for downloading NixOS environment
- GitHub access to clone repository

### Verification
You should be booted into a NixOS live environment via netboot.xyz. Verify:

```bash
# Check you're in NixOS
uname -a
# Should show: Linux ... NixOS

# Check required tools are available
which git curl cargo rustc nix-env

# Check internet connectivity
ping -c 3 github.com
```

---

## Stage 1: Environment Setup

### 1.1 Install Missing Dependencies (if needed)

```bash
# Check Rust version
rustc --version
cargo --version

# If Rust is missing or old, install via nixpkgs
nix-env -iA nixpkgs.rustc nixpkgs.cargo

# Verify installation
cargo --version  # Should be 1.70+
```

### 1.2 Clone Repository

```bash
# Clone the repository
cd ~
git clone https://github.com/repr0bated/operation-dbus.git
cd operation-dbus

# Checkout the correct branch with all new features
git checkout claude/this-is-no-011CUr6iUZZC7YKhhLSY2MK6

# Verify you're on the right branch
git branch
# Should show: * claude/this-is-no-011CUr6iUZZC7YKhhLSY2MK6

# Check what's new
git log --oneline -5
```

---

## Stage 2: Build op-dbus

### 2.1 Compile the Project

```bash
# Build in release mode (faster execution)
cargo build --release

# This will take 5-10 minutes on first build
# Watch for compilation errors
```

### 2.2 Expected Build Issues & Fixes

If the build fails, common issues:

#### Missing Dependencies
```bash
# If you see "could not find crate" errors
cargo fetch  # Download all dependencies first
cargo build --release
```

#### Out of Memory
```bash
# If build crashes with OOM on low-RAM systems
# Build with limited parallelism
cargo build --release -j 2  # Use only 2 cores
```

#### Permission Errors
```bash
# If you see permission errors
chmod -R u+w ~/.cargo
cargo build --release
```

### 2.3 Verify Binary

```bash
# Check binary was created
ls -lh target/release/op-dbus

# Should show file size ~15-30 MB

# Quick test (don't run full discover yet)
./target/release/op-dbus --version
```

---

## Stage 3: System Introspection

### 3.1 Run Discovery on Samsung 360 Pro

```bash
# Run introspection as root (needs hardware access)
sudo ./target/release/op-dbus discover

# You should see:
# - Hardware detection: SAMSUNG + 360 model
# - Known issues identified (buggy BIOS warning)
# - Current kernel parameters captured
# - D-Bus services discovered
```

### 3.2 Export Configuration

```bash
# Export full introspection + generate NixOS config
sudo ./target/release/op-dbus discover \
  --export \
  --generate-nix \
  --output samsung360-reference

# This creates two files:
# - samsung360-reference.json       (introspection data)
# - samsung360-reference.nix        (NixOS configuration)
```

### 3.3 Review Generated Configuration

```bash
# Look at the generated NixOS config
cat samsung360-reference.nix

# Should contain:
# 1. Hardware comments with known issues
# 2. boot.kernelParams with acpi=off, intel_idle.max_cstate=1, etc.
# 3. boot.kernelModules with detected modules
# 4. CPU mitigation summary
# 5. op-dbus service configuration
```

**Expected Output:**
```nix
{ config, pkgs, ... }:

{
  imports = [
    ./hardware-configuration.nix
  ];

  # Hardware: SAMSUNG 360 Pro
  # BIOS Version: [detected]
  # Known Issues:
  #   - Buggy BIOS: Requires acpi=off kernel parameter
  #   - Power management: Use intel_idle.max_cstate=1
  #   - PCIe ASPM: Use pcie_aspm=off
  #   - Screen flickering: Use i915.enable_psr=0

  boot.kernelParams = [
    "acpi=off"
    "intel_idle.max_cstate=1"
    "pcie_aspm=off"
    "i915.enable_psr=0"
    "quiet"
    "splash"
  ];

  # CPU Vulnerability Mitigations: X of Y active
  # For QEMU/KVM hosts: Consider performance vs security tradeoff
  #   - mitigations=off → Fast but vulnerable
  #   - mitigations=auto → Secure but slower (default)

  boot.kernelModules = [ ... ];

  # op-dbus configuration
  services.op-dbus = {
    enable = true;
    blockchain.enable = true;
    numa.enable = true;
  };
}
```

---

## Stage 4: Permanent Installation

### 4.1 Prepare Installation

```bash
# Generate hardware configuration
nixos-generate-config --root /mnt

# This creates:
# /mnt/etc/nixos/hardware-configuration.nix
# /mnt/etc/nixos/configuration.nix
```

### 4.2 Integrate op-dbus Configuration

```bash
# Copy generated config
sudo cp samsung360-reference.nix /mnt/etc/nixos/

# Edit main configuration to import it
sudo nano /mnt/etc/nixos/configuration.nix
```

Add this import:
```nix
{ config, pkgs, ... }:

{
  imports = [
    ./hardware-configuration.nix
    ./samsung360-reference.nix  # <-- Add this
  ];

  # Your other configuration...
  networking.hostName = "samsung360";

  # Enable NetworkManager for WiFi
  networking.networkmanager.enable = true;

  # Create your user
  users.users.yourname = {
    isNormalUser = true;
    extraGroups = [ "wheel" "networkmanager" ];
  };
}
```

### 4.3 Install NixOS

```bash
# Partition disk (adjust device name as needed)
# WARNING: This will ERASE the disk!
sudo parted /dev/sda -- mklabel gpt
sudo parted /dev/sda -- mkpart ESP fat32 1MiB 512MiB
sudo parted /dev/sda -- set 1 esp on
sudo parted /dev/sda -- mkpart primary btrfs 512MiB 100%

# Format partitions
sudo mkfs.fat -F 32 -n boot /dev/sda1
sudo mkfs.btrfs -L nixos /dev/sda2

# Create BTRFS subvolumes for op-dbus
sudo mount /dev/sda2 /mnt
sudo btrfs subvolume create /mnt/root
sudo btrfs subvolume create /mnt/home
sudo btrfs subvolume create /mnt/nix

# Create op-dbus subvolumes
sudo btrfs subvolume create /mnt/opdbus
sudo btrfs subvolume create /mnt/opdbus/state
sudo btrfs subvolume create /mnt/opdbus/timing
sudo btrfs subvolume create /mnt/opdbus/vectors
sudo btrfs subvolume create /mnt/opdbus/snapshots

sudo umount /mnt

# Mount with proper structure
sudo mount -o subvol=root,compress=zstd,noatime /dev/sda2 /mnt
sudo mkdir -p /mnt/{boot,home,nix,var/lib/op-dbus}
sudo mount /dev/sda1 /mnt/boot
sudo mount -o subvol=home,compress=zstd,noatime /dev/sda2 /mnt/home
sudo mount -o subvol=nix,compress=zstd,noatime /dev/sda2 /mnt/nix
sudo mount -o subvol=opdbus,compress=zstd,noatime /dev/sda2 /mnt/var/lib/op-dbus

# Install NixOS
sudo nixos-install

# Set root password when prompted
```

### 4.4 First Boot

```bash
# Reboot into installed system
sudo reboot

# You should boot directly without graphical installer issues
# The acpi=off parameter should be active from bootloader
```

---

## Stage 5: Verification

### 5.1 Verify op-dbus Service

```bash
# Check service is running
systemctl status op-dbus

# Should show: active (running)

# Check logs
journalctl -u op-dbus -f

# Verify subvolumes are mounted
df -h | grep opdbus
mount | grep btrfs
```

### 5.2 Test Introspection Again

```bash
# Run discovery on installed system
sudo op-dbus discover

# Compare with netboot results
# Should show same hardware detection
# Should show same kernel parameters
# May show more services (since full system is running)
```

### 5.3 Create First Snapshot

```bash
# Capture initial system state
sudo op-dbus apply --dry-run

# This should create blockchain entry and snapshot
# Check snapshot was created
sudo btrfs subvolume list /var/lib/op-dbus
```

---

## Stage 6: Replication Test

### 6.1 Use This Laptop as Reference

```bash
# This Samsung 360 Pro is now your AUTHORITATIVE reference
# for problematic hardware deployment

# Export complete machine profile
sudo op-dbus discover \
  --export \
  --generate-nix \
  --include-packages \
  --output /var/lib/op-dbus/state/samsung360-master.json

# This captures:
# - Exact hardware configuration
# - All kernel workarounds
# - Installed packages
# - D-Bus service state
# - System topology
```

### 6.2 Test Replication to Another Machine

On a different machine (VM or another laptop):

```bash
# Copy the samsung360-master.json and .nix files
scp samsung360:/var/lib/op-dbus/state/samsung360-master.* .

# Install NixOS using the generated .nix configuration
# The new machine will have:
# - Same kernel parameters (if same hardware)
# - Same package set
# - Same op-dbus configuration
# - Different hardware-specific settings (auto-detected)
```

---

## Troubleshooting

### Issue: Build Fails with Network Errors

```bash
# If cargo can't download crates
# Check DNS
ping -c 3 crates.io

# Try alternative crates.io mirror
mkdir -p ~/.cargo
cat > ~/.cargo/config.toml <<EOF
[source.crates-io]
replace-with = "tuna"

[source.tuna]
registry = "https://mirrors.tuna.tsinghua.edu.cn/git/crates.io-index.git"
EOF

# Retry build
cargo build --release
```

### Issue: Screen Flickers After Boot

```bash
# The i915.enable_psr=0 parameter should prevent this
# If it still happens, add more Intel graphics workarounds

# Edit /etc/nixos/samsung360-reference.nix
boot.kernelParams = [
  "acpi=off"
  "intel_idle.max_cstate=1"
  "pcie_aspm=off"
  "i915.enable_psr=0"
  "i915.enable_fbc=0"        # Add this
  "i915.enable_rc6=0"        # And this
];

# Rebuild
sudo nixos-rebuild switch
```

### Issue: WiFi Not Working

```bash
# Check WiFi driver is loaded
lspci -k | grep -A 3 -i network

# If driver missing, add to kernel modules
boot.kernelModules = [ "iwlwifi" ];  # For Intel WiFi

# Rebuild
sudo nixos-rebuild switch
```

### Issue: op-dbus Service Fails to Start

```bash
# Check detailed logs
journalctl -u op-dbus -b

# Common issues:
# 1. BTRFS subvolumes not mounted
mount | grep opdbus

# 2. Permission issues
ls -la /var/lib/op-dbus
sudo chown -R root:root /var/lib/op-dbus
sudo chmod 755 /var/lib/op-dbus

# 3. D-Bus connection issues
systemctl status dbus
```

---

## Success Criteria

Your Samsung 360 Pro deployment is successful when:

✅ **System boots reliably** without graphical installer bugs
✅ **No screen flickering** or power management issues
✅ **op-dbus service runs** on startup
✅ **Introspection detects** Samsung 360 Pro hardware and known issues
✅ **BTRFS subvolumes** are properly mounted (state/timing/vectors/snapshots)
✅ **Blockchain footprints** are being created in timing/ subvolume
✅ **State snapshots** are being saved to state/ subvolume
✅ **Generated NixOS config** can be used to replicate to other machines

---

## What This Proves

By successfully deploying op-dbus on the Samsung 360 Pro, you demonstrate:

1. **Resilience**: Works on hardware with known BIOS bugs
2. **Introspection Quality**: Correctly detects and documents hardware issues
3. **Replication Accuracy**: Captures exact kernel workarounds needed
4. **Production Readiness**: If it works here, it works anywhere
5. **Enterprise Viability**: Can handle problematic fleet hardware

This laptop becomes the **reference implementation** for:
- Worst-case hardware scenarios
- BIOS workaround documentation
- Kernel parameter tuning
- Hardware issue detection algorithms

---

## Next Steps

After successful deployment:

1. **Document Results**: Record any additional issues discovered
2. **Update Hardware Database**: Add Samsung 360 Pro specifics to known_hardware_issues
3. **Test Replication**: Deploy to a second machine using generated config
4. **Enterprise Testing**: Test with more problematic hardware (Dell XPS, Lenovo X1)
5. **Update NVIDIA Inception Application**: With real deployment data

---

## Support

If you encounter issues not covered here:

1. Check logs: `journalctl -u op-dbus -b`
2. Review introspection output: `sudo op-dbus discover`
3. Check GitHub issues: https://github.com/repr0bated/operation-dbus/issues
4. Document new hardware quirks for the community

---

## File Locations Reference

**Configuration:**
- `/etc/nixos/configuration.nix` - Main NixOS config
- `/etc/nixos/samsung360-reference.nix` - Generated op-dbus config
- `/etc/nixos/hardware-configuration.nix` - Hardware detection

**op-dbus Data:**
- `/var/lib/op-dbus/state/` - Current system state (DR snapshots)
- `/var/lib/op-dbus/timing/` - Blockchain audit trail
- `/var/lib/op-dbus/vectors/` - ML embeddings
- `/var/lib/op-dbus/snapshots/` - BTRFS snapshots

**Logs:**
- `journalctl -u op-dbus` - Service logs
- `/var/log/nixos/` - System logs

**Build Artifacts:**
- `~/operation-dbus/target/release/op-dbus` - Binary
- `~/operation-dbus/samsung360-reference.json` - Introspection export
- `~/operation-dbus/samsung360-reference.nix` - NixOS config

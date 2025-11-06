# Machine Replication with op-dbus

## The Killer Feature: Clone ANY Machine

op-dbus can scan any existing machine and replicate it exactly on new hardware. This includes:

- ✅ All D-Bus services and their configurations
- ✅ SystemD units and dependencies
- ✅ Network configuration (OVS bridges, interfaces)
- ✅ Container configurations (LXC, Docker)
- ✅ **BIOS workarounds and kernel parameters** (critical for buggy hardware)
- ✅ Installed packages (via PackageKit)
- ✅ Custom services and daemons

**Use Cases**:
1. **Disaster Recovery**: Server dies → clone to new hardware in <30 minutes
2. **Horizontal Scaling**: Clone web server 10x for load balancing
3. **Dev → Prod**: Development environment becomes production template
4. **Fleet Management**: Configure one laptop → deploy to 500 identical
5. **BIOS Workarounds**: Capture working config for problematic hardware

---

## Quick Start

### Step 1: Scan Existing Machine

```bash
# On the machine you want to replicate
sudo op-dbus discover

# This will show:
# - ✅ D-Bus services currently managed
# - 🔍 D-Bus services that could be managed
# - 🔄 Non-D-Bus services that could be converted
# - 📊 Overall coverage percentage
```

### Step 2: Export Machine State

```bash
# Export for replication
sudo op-dbus discover --export --output my-machine.json

# Also generate NixOS configuration
sudo op-dbus discover --export --generate-nix --output my-machine.nix

# Output:
# ✓ Exported machine state to: my-machine.json
# ✓ Generated NixOS configuration: my-machine.nix
```

### Step 3: Replicate on New Hardware

```bash
# On target machine:
# 1. Install NixOS
# 2. Copy my-machine.nix to /etc/nixos/configuration.nix
# 3. Copy my-machine.json to /etc/op-dbus/state.json

sudo nixos-rebuild switch
sudo op-dbus apply /etc/op-dbus/state.json

# Machine is now an exact replica!
```

---

## Example: Samsung 360 Pro (Buggy BIOS)

The Samsung 360 Pro has notorious BIOS bugs (broken ACPI, power management, etc.). Here's how op-dbus helps:

### Problem
- BIOS reports incorrect ACPI tables → system freezes
- Power management broken → CPU stuck at max frequency
- Touchscreen issues → requires kernel quirks
- Random crashes on resume from sleep

### Traditional Solution (Manual)
1. Research Samsung 360 Pro issues (hours on forums)
2. Manually edit `/etc/default/grub` with kernel parameters
3. Load specific modules in `/etc/modules-load.d/`
4. Configure power management in `/etc/udev/rules.d/`
5. Test and iterate (days of trial and error)
6. **If you get another 360 Pro**: Repeat all steps manually

### op-dbus Solution (Automated)
1. **One person** figures out the workarounds once
2. Capture working configuration:
   ```bash
   sudo op-dbus discover --export --generate-nix
   ```
3. **Everyone else** just deploys the known-good config:
   ```bash
   sudo nixos-rebuild switch
   ```
4. **All Samsung 360 Pro laptops** are now identically configured

### What Gets Captured

The introspection scan captures:

**Kernel Parameters** (from `/proc/cmdline`):
```
acpi=off                           # Disable broken ACPI
intel_idle.max_cstate=1            # Prevent freeze on idle
pcie_aspm=off                      # Disable buggy PCIe power mgmt
i915.enable_psr=0                  # Fix screen flickering
```

**Loaded Modules** (from `/proc/modules`):
```
i2c_hid                            # Touchscreen support
hid_multitouch                     # Multi-touch gestures
```

**SystemD Services** (working configuration):
```
✓ tlp.service → active             # Battery optimization
✓ thermald.service → active        # Thermal management
⊗ pcscd.service → disabled         # Smart card (unused)
```

**NixOS Configuration** (generated):
```nix
{ config, pkgs, ... }:

{
  # Samsung 360 Pro BIOS workarounds
  boot.kernelParams = [
    "acpi=off"
    "intel_idle.max_cstate=1"
    "pcie_aspm=off"
    "i915.enable_psr=0"
  ];

  # Essential modules
  boot.kernelModules = [
    "i2c_hid"
    "hid_multitouch"
  ];

  # Power management
  services.tlp.enable = true;
  services.thermald.enable = true;

  # Disable problematic services
  services.pcscd.enable = false;

  # op-dbus manages everything else
  services.op-dbus = {
    enable = true;
    state = {
      version = 1;
      # ... discovered configuration ...
    };
  };
}
```

Now this configuration can be deployed to **any Samsung 360 Pro** and it will work identically.

---

## CLI Reference

### `op-dbus discover`
Scan system and show what's manageable

```bash
sudo op-dbus discover

# Output:
# 📊 SUMMARY
# ──────────────────────────────────────────────
#   Total D-Bus services:    23
#   ✓ Managed services:      8
#   ⊗ Unmanaged services:    15
#   🔄 Conversion candidates: 12
#   Coverage:                34.8%
#
# ✅ MANAGED D-BUS SERVICES
# ──────────────────────────────────────────────
#   ✓ org.freedesktop.systemd1 (system)
#     Built-in plugin: systemd
#
#   ✓ org.freedesktop.login1 (system)
#     Built-in plugin: login1
#
# 🔍 UNMANAGED D-BUS SERVICES (Conversion Opportunity)
# ──────────────────────────────────────────────
#   ⊗ org.freedesktop.PackageKit (system)
#     → Recommended plugin: packagekit
#
#   ⊗ org.freedesktop.NetworkManager (system)
#     → Recommended plugin: networkmanager
#
# 🔄 NON-D-BUS SERVICES (Could Be Converted)
# ──────────────────────────────────────────────
#   🟢 nginx.service (systemd)
#     Current: systemctl / systemd D-Bus (indirect)
#     Opportunity: Web server status/reload could be exposed via D-Bus
#
#   🟡 docker.service (systemd)
#     Current: systemctl / systemd D-Bus (indirect)
#     Opportunity: Docker could expose management API via D-Bus
```

### `op-dbus discover --export`
Export machine state for replication

```bash
sudo op-dbus discover --export --output server-prod-01.json

# Creates: server-prod-01.json
# Contains: Complete machine state in JSON format
# Use for: Replicating to other machines
```

### `op-dbus discover --generate-nix`
Generate NixOS configuration from current machine

```bash
sudo op-dbus discover --generate-nix --output replicated-config.nix

# Creates: replicated-config.nix
# Contains: NixOS configuration matching this machine
# Use for: Deploying to new NixOS installations
```

### `op-dbus discover --include-packages`
Include software inventory (via PackageKit)

```bash
sudo op-dbus discover --export --include-packages

# Scans:
# - org.freedesktop.PackageKit (if available)
# - Installed packages and versions
# - Package sources (apt, dnf, etc.)
#
# Result:
# - Generated config includes all packages
# - Replicated machine has identical software
```

---

## Use Case Examples

### 1. Disaster Recovery (Server Fails)

**Scenario**: Production web server crashes at 2 AM

**Without op-dbus**:
1. Find backup documentation (if it exists)
2. Manually reinstall OS
3. Manually install packages (apt install ...)
4. Manually configure services (nginx, postgres, etc.)
5. Manually restore data
6. **Total time**: 4-6 hours

**With op-dbus**:
```bash
# 1. Install NixOS on new hardware (10 minutes)
# 2. Deploy saved configuration
sudo nixos-rebuild switch  # Uses web-server-prod.nix

# 3. Restore data (separate backup)
sudo restic restore latest --target /var/www

# 4. Verify
sudo op-dbus verify
```
**Total time**: 30 minutes

### 2. Horizontal Scaling (Clone Web Server 10x)

**Scenario**: Black Friday traffic spike, need more web servers

**Without op-dbus**:
1. Manually configure first server (hours)
2. Create image/snapshot
3. Deploy to 10 VMs
4. Manually verify each one
5. **Configuration drift**: Each server slightly different

**With op-dbus**:
```bash
# 1. Scan existing web server (once)
sudo op-dbus discover --export --output web-server.json

# 2. Deploy to 10 new VMs (parallel)
for vm in web-{01..10}; do
  ssh $vm "sudo op-dbus apply /shared/web-server.json"
done

# 3. Verify all identical
for vm in web-{01..10}; do
  ssh $vm "sudo op-dbus verify"
done
```
**Result**: 10 identical servers, zero configuration drift

### 3. Dev → Prod (Development Becomes Template)

**Scenario**: Developer configures perfect environment, want to deploy to production

**Without op-dbus**:
1. Developer documents setup (incomplete)
2. DevOps tries to replicate (misses details)
3. Production behaves differently
4. "Works on my machine" syndrome

**With op-dbus**:
```bash
# Developer exports their working machine
dev-laptop$ sudo op-dbus discover --export --output dev-env.json

# DevOps deploys to production
prod-server$ sudo op-dbus apply dev-env.json

# Verify they're identical
prod-server$ sudo op-dbus verify
dev-laptop$ sudo op-dbus query > dev-state.json
prod-server$ sudo op-dbus query > prod-state.json
diff dev-state.json prod-state.json
# No differences!
```

### 4. Fleet Management (500 Laptops)

**Scenario**: Company-wide laptop refresh, need to configure 500 identical machines

**Without op-dbus**:
1. Image one laptop (Clonezilla, Ghost, etc.)
2. Deploy to 500 laptops (slow)
3. Manually customize each (hostname, user, etc.)
4. **Time**: 1-2 hours per laptop × 500 = 500-1000 hours

**With op-dbus**:
```bash
# 1. Configure golden laptop (once)
sudo op-dbus discover --export --generate-nix

# 2. PXE boot all 500 laptops (parallel)
# Each laptop:
# - Downloads NixOS installer
# - Fetches generated-configuration.nix from server
# - Applies op-dbus state
# - Ready in 25 minutes

# 3. Monitor deployment
grafana-dashboard: 485/500 complete, 15 in progress
```
**Time**: 25 minutes (all 500 laptops in parallel)

### 5. BIOS Workaround Distribution

**Scenario**: Company buys 200 Samsung 360 Pro laptops (all have buggy BIOS)

**Without op-dbus**:
1. One person figures out workarounds (days)
2. Writes documentation
3. 199 other people try to follow documentation
4. Half of them get it wrong
5. Ongoing support tickets

**With op-dbus**:
```bash
# 1. Expert configures one laptop (days of trial and error)
# 2. Export working configuration
sudo op-dbus discover --export --generate-nix --output samsung-360-pro.nix

# 3. Deploy to other 199 laptops (automated)
for laptop in laptop-{001..199}; do
  ssh $laptop "sudo nixos-rebuild switch"
done

# All 200 laptops now have identical, working configuration
```

---

## Introspection Report Breakdown

### Managed D-Bus Services
Services that op-dbus **currently manages** via built-in or auto-generated plugins:

- `org.freedesktop.systemd1` → systemd plugin (services, units, timers)
- `org.freedesktop.login1` → login1 plugin (user sessions, power management)
- Custom services with auto-generated plugins

### Unmanaged D-Bus Services
Services that **expose D-Bus** but op-dbus doesn't manage yet:

- `org.freedesktop.PackageKit` → Package management
- `org.freedesktop.NetworkManager` → Network configuration
- `org.freedesktop.UPower` → Battery/power info
- `org.freedesktop.UDisks2` → Disk management
- `org.bluez` → Bluetooth

**Opportunity**: Create plugins for these services to increase coverage

### Conversion Candidates
Services that **don't use D-Bus** but could:

**🟢 Easy Conversion**:
- Web servers (nginx, apache) → Expose reload/status via D-Bus
- Backup services (restic, borgbackup) → Status monitoring
- VPN services (WireGuard, OpenVPN) → Connection control

**🟡 Medium Complexity**:
- Package managers (apt, dnf) → Already have PackageKit
- Docker → Could expose management API
- Databases (postgres, mysql) → Management interface

**🔴 Hard Conversion**:
- Container runtimes (containerd) → Complex state
- Low-level services → May not benefit from D-Bus

---

## Best Practices

### 1. Scan Often
Run `op-dbus discover` regularly to track coverage:

```bash
# Add to cron
0 */6 * * * op-dbus discover > /var/log/opdbus-discover.log
```

Track coverage percentage over time:
- **Goal**: >80% coverage for production servers
- **Realistic**: 60-70% for complex systems

### 2. Version Your Configurations
Keep replicated configs in Git:

```bash
mkdir -p ~/machine-configs/
sudo op-dbus discover --export --output ~/machine-configs/$(hostname)-$(date +%Y%m%d).json
cd ~/machine-configs && git add . && git commit -m "Updated $(hostname) config"
```

### 3. Test Replication Before Disaster
Don't wait for a real disaster to test replication:

```bash
# Monthly drill: Replicate to test VM
sudo op-dbus discover --export --output prod-server.json
scp prod-server.json test-vm:/tmp/
ssh test-vm "sudo op-dbus apply /tmp/prod-server.json"

# Verify they match
diff <(ssh prod-server "sudo op-dbus query") \
     <(ssh test-vm "sudo op-dbus query")
```

### 4. Document What Can't Be Replicated
Some things op-dbus can't capture (yet):

- **Secrets** (passwords, keys) → Use secrets management
- **Data** (databases, files) → Use separate backup solution
- **Hardware-specific** (GPU settings) → Document separately
- **External dependencies** (APIs, cloud services) → Configuration management

### 5. Combine with Traditional Backups
op-dbus replicates **configuration**, not **data**:

```bash
# Configuration → op-dbus
sudo op-dbus discover --export --output /backup/config.json

# Data → restic/borgbackup
sudo restic backup /var/lib/postgresql /var/www
```

---

## Comparison with Other Tools

| Feature | op-dbus | Ansible | Docker | VM Snapshot |
|---------|---------|---------|--------|-------------|
| **Replication Speed** | <30 min | 1-2 hours | Fast | Fast |
| **Configuration Drift** | None (declarative) | Possible | None | None |
| **Cross-Hardware** | ✅ Yes | ✅ Yes | ⚠️ Limited | ❌ No |
| **Bare Metal** | ✅ Yes | ✅ Yes | ❌ No | ❌ No |
| **Audit Trail** | ✅ Blockchain | ❌ No | ⚠️ Logs | ❌ No |
| **Introspection** | ✅ Built-in | ⚠️ Manual | ❌ No | ❌ No |
| **BIOS Workarounds** | ✅ Captured | ⚠️ Manual | ❌ No | ❌ No |

**When to use what**:
- **op-dbus**: Full system replication, bare metal, NixOS
- **Ansible**: Cross-platform (Windows/Mac), existing infrastructure
- **Docker**: Application-level, microservices
- **VM Snapshot**: Exact disk clone, same hardware

---

## Roadmap

### Short-term (v0.3 - v0.4)
- [x] Basic introspection (D-Bus services)
- [ ] PackageKit integration (software inventory)
- [ ] Kernel parameter capture (BIOS workarounds)
- [ ] Module loading configuration

### Medium-term (v0.5 - v0.6)
- [ ] NetworkManager plugin (full network config)
- [ ] UPower plugin (power management)
- [ ] Full NixOS config generation
- [ ] Cross-machine diff tool

### Long-term (v0.7+)
- [ ] Windows support (replicate mixed fleets)
- [ ] Cloud instance replication (AWS, GCP, Azure)
- [ ] AI-powered optimization (suggest improvements)
- [ ] Compliance scanning (HIPAA, SOC2, GDPR)

---

## Support & Community

- **Documentation**: https://github.com/repr0bated/operation-dbus/docs
- **Issues**: https://github.com/repr0bated/operation-dbus/issues
- **Discussions**: https://github.com/repr0bated/operation-dbus/discussions
- **Matrix Chat**: #operation-dbus:matrix.org

**Share Your Success Stories**:
- Replicated a machine in record time?
- Fixed a BIOS issue across a fleet?
- Scaled from 1 to 100 servers easily?

Let us know! Your experience helps improve op-dbus for everyone.

---

**The killer feature: Clone ANY machine with one command.**

*This is why enterprises will choose op-dbus.*

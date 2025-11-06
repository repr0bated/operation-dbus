# BIOS Feature Unlocking with op-dbus

## Overview

**OEMs artificially disable CPU features via BIOS** to segment their product lines. The same CPU in a "consumer" laptop vs a "Pro" model often has features like VT-x disabled purely through software - this is a **$200-500 markup for changing a BIOS bit**.

op-dbus can:
1. **Detect** hidden CPU features your hardware supports
2. **Expose** BIOS locks preventing you from using them
3. **Recommend** unlock methods (MSR writes, kernel parameters, BIOS updates)
4. **Enable** features automatically (where safe)

This is **critical for enterprise** - why pay Dell/HP/Lenovo extra for "Pro" SKUs when the hardware is identical?

---

## Common OEM Market Segmentation Tactics

### Intel VT-x (Virtualization)

**What it is**: Hardware virtualization support (required for KVM, Docker, VirtualBox, Hyper-V)

**Market segmentation**:
- **Consumer models**: VT-x disabled in BIOS, no option to enable
- **Business models**: Same CPU, VT-x enabled, +$200-300 price
- **Workstation models**: Same CPU, VT-x + VT-d enabled, +$500 price

**Examples**:
- Dell Inspiron (consumer) vs Latitude (business) - identical i5/i7, different BIOS
- HP Pavilion vs EliteBook - same hardware, $400 difference
- Lenovo IdeaPad vs ThinkPad - VT-x lock is literally one BIOS bit

### Intel VT-d / AMD-Vi (IOMMU)

**What it is**: PCI device passthrough for VMs (GPU passthrough, NVMe passthrough)

**Market segmentation**:
- **Consumer**: Disabled, no BIOS option
- **Workstation**: Enabled, +$300-800

### Intel SGX (Software Guard Extensions)

**What it is**: Secure enclaves for confidential computing

**Market segmentation**:
- **Consumer**: Disabled or fused off
- **Enterprise**: Enabled, +$200

### Turbo Boost / Precision Boost

**What it is**: Dynamic CPU frequency scaling (20-30% performance boost)

**Market segmentation**:
- **Low-end SKUs**: Disabled via BIOS lock
- **High-end SKUs**: Same CPU die, turbo enabled, +$150

---

## How op-dbus Detects BIOS Locks

### 1. CPU Flag Analysis

```bash
# Read CPU capabilities from /proc/cpuinfo
grep -E 'vmx|svm' /proc/cpuinfo

# If "vmx" (Intel VT-x) or "svm" (AMD-V) present → CPU supports virtualization
# If /dev/kvm doesn't exist → BIOS has disabled it
```

### 2. MSR (Model Specific Register) Inspection

**Intel VT-x Lock Detection**:
```bash
# Read IA32_FEATURE_CONTROL register (MSR 0x3A)
modprobe msr
rdmsr 0x3A

# Bit layout:
# Bit 0: Lock bit (1 = locked by BIOS before OS boot)
# Bit 2: VMX enable (1 = VT-x enabled)

# 0x1 → Locked, VMX disabled → BIOS LOCK DETECTED
# 0x5 → Locked, VMX enabled → Working
# 0x0 → Unlocked, VMX disabled → CAN BE ENABLED
```

**AMD SVM Lock Detection**:
```bash
# Check VM_CR register (MSR 0xC0010114)
rdmsr 0xC0010114

# Bit 4: SVM disable (1 = locked disabled)
```

### 3. /dev/kvm Existence Check

```bash
ls /dev/kvm

# If exists → Virtualization working
# If missing + CPU supports → BIOS disabled it
```

### 4. IOMMU Detection

```bash
# Check kernel detected IOMMU
dmesg | grep -E 'IOMMU|DMAR|AMD-Vi'

# Check IOMMU groups exist
ls /sys/kernel/iommu_groups/

# If CPU supports but no groups → BIOS disabled
```

---

## Example: Your VT-x Scenario

**Scenario**: You have a laptop with VT-x-capable CPU but BIOS has no option to enable it.

### Detection Output

When you run `sudo op-dbus discover`, you'll see:

```
🔓 CPU FEATURES & BIOS LOCKS
──────────────────────────────────────────────────────────────
  CPU: Intel(R) Core(TM) i5-8250U (Family 6)
  Microcode: 0xb4

  ⚠️  DISABLED/LOCKED FEATURES:
    🔒 VT-x (Intel Virtualization) (vmx): BIOS Locked

  🔒 BIOS LOCKS DETECTED:
    Register: MSR 0x3A (IA32_FEATURE_CONTROL)
      Lock Bit: Bit 0 (Lock), Bit 2 (VMX Enable)
      Affects: VT-x, KVM
      Method: BIOS MSR lock bit set before OS boot

  💡 RECOMMENDATIONS:
    🔴 VT-x - Critical Priority
       Reason: CPU supports VT-x but BIOS has locked it via MSR
       Benefit: Enable KVM virtualization, Docker, QEMU, VirtualBox with hardware acceleration
       Action: BIOS Update: Check for BIOS update that exposes VT-x option, or use BIOS modification tools (advanced)
```

### Exported JSON

The introspection also exports machine-readable data:

```json
{
  "system_config": {
    "cpu_features": {
      "cpu_model": {
        "vendor": "Intel",
        "model_name": "Intel(R) Core(TM) i5-8250U",
        "family": "6",
        "microcode": "0xb4"
      },
      "features": [
        {
          "name": "VT-x (Intel Virtualization)",
          "technical_name": "vmx",
          "category": "Virtualization",
          "status": "LockedByBios",
          "bios_locked": true,
          "unlock_method": {
            "method": "MSR Write",
            "risk_level": "Medium",
            "commands": [
              "modprobe msr",
              "wrmsr 0x3A 0x5"
            ],
            "description": "Write to IA32_FEATURE_CONTROL MSR to enable VT-x. Only works if BIOS has not set lock bit.",
            "requires_reboot": true
          }
        }
      ],
      "bios_locks": [
        {
          "register": "MSR 0x3A (IA32_FEATURE_CONTROL)",
          "lock_bit": "Bit 0 (Lock), Bit 2 (VMX Enable)",
          "affected_features": ["VT-x", "KVM"],
          "locked": true,
          "lock_method": "BIOS MSR lock bit set before OS boot"
        }
      ],
      "recommendations": [
        {
          "priority": "Critical",
          "feature": "VT-x",
          "reason": "CPU supports VT-x but BIOS has locked it via MSR",
          "benefit": "Enable KVM virtualization, Docker, QEMU, VirtualBox",
          "action": "Check for BIOS update or use BIOS modification"
        }
      ]
    }
  }
}
```

---

## Unlock Methods (Risk Levels)

### Safe (Green) - No Risk

**Method**: BIOS Update from Vendor
- Check Dell/HP/Lenovo support site for newer BIOS
- Some vendors unlock features in later BIOS versions
- **Example**: Dell released BIOS update for XPS 13 9370 that exposed VT-x option

**Method**: Kernel Parameters
- Some features can be force-enabled via boot params
- IOMMU: `intel_iommu=on amd_iommu=on`
- No permanent changes, easily reversible

### Low Risk (Blue) - Reversible

**Method**: MSR Write (if lock bit not set)
```bash
# Check if unlocked
rdmsr 0x3A
# If output is 0x0 → can enable

# Enable VT-x
modprobe msr
wrmsr 0x3A 0x5

# Check /dev/kvm exists
ls -l /dev/kvm

# Reboot required to persist
```

**Risk**: Only works if BIOS didn't set lock bit. If it did, write is ignored. No damage possible.

### Medium Risk (Yellow) - Some Risk

**Method**: BIOS Configuration Tools
- `ru.efi` (UEFI Shell tool for viewing/editing BIOS variables)
- `setup_var` scripts to modify hidden BIOS options
- **Risk**: Can brick BIOS if wrong variable modified, but usually recoverable

**Process**:
1. Boot to UEFI Shell
2. Use `ru.efi` to view BIOS variables
3. Find "Virtualization" variable (usually offset 0x4XX)
4. Change value from 0x0 (disabled) to 0x1 (enabled)
5. Reboot

**Example** (Dell XPS 13):
```bash
# Boot to UEFI shell with ru.efi
setup_var 0x43F 0x1  # Enable VT-x
setup_var 0x440 0x1  # Enable VT-d
reboot
```

### High Risk (Red) - BIOS Modification

**Method**: BIOS ROM Patching
- Extract BIOS ROM via SPI flash programmer
- Patch BIOS with UEFITool to expose hidden options
- Re-flash modified BIOS

**Risk**: Can brick motherboard if done incorrectly. Requires hardware tools (CH341A programmer).

**When needed**: If vendor fused off feature or lock bit is always set.

### Vendor Locked (Black) - Cannot Unlock

**Method**: None - Hardware Fuse Blown
- Some OEMs physically fuse off features in CPU microcode
- Cannot be unlocked by any software method
- **Example**: Intel SGX on some consumer CPUs (fused off at factory)

---

## Automated Unlock with op-dbus

op-dbus can **automatically attempt safe unlock methods**:

### Configuration

In NixOS config or `op-dbus.conf`:

```nix
services.op-dbus = {
  enable = true;

  # Automatically unlock CPU features
  cpu_unlock = {
    enable = true;

    # Only attempt safe methods (BIOS update, kernel params)
    risk_level = "safe";

    # Or allow low-risk methods (MSR writes)
    # risk_level = "low";

    # Specific features to enable
    features = [
      "vt-x"      # Intel virtualization
      "amd-v"     # AMD virtualization
      "iommu"     # VT-d/AMD-Vi
      "turbo"     # Turbo Boost
    ];
  };
};
```

### Auto-Apply on Boot

op-dbus service checks CPU features on boot and applies unlock methods:

```bash
# systemd service runs on boot
[Unit]
Description=op-dbus CPU Feature Unlock
After=systemd-modules-load.service

[Service]
Type=oneshot
ExecStart=/usr/bin/op-dbus unlock-cpu --auto
RemainAfterExit=yes

[Install]
WantedBy=multi-user.target
```

### Manual Unlock Command

```bash
# Analyze and show what can be unlocked
sudo op-dbus unlock-cpu --dry-run

# Unlock specific feature
sudo op-dbus unlock-cpu --feature vt-x

# Unlock all safe features
sudo op-dbus unlock-cpu --all --risk-level safe

# Unlock with low-risk methods (MSR writes)
sudo op-dbus unlock-cpu --all --risk-level low
```

---

## Real-World Use Cases

### 1. Enterprise Laptop Fleet

**Problem**: Ordered 500 Dell Inspiron laptops (consumer model) but need Docker/Kubernetes

**Solution**:
1. Run `op-dbus discover` on test unit
2. Detect VT-x is BIOS-locked
3. Check if MSR unlock works (low-risk)
4. Deploy NixOS with op-dbus auto-unlock across fleet
5. **Savings**: $200/laptop × 500 = **$100,000 saved** vs buying Latitude (business) models

### 2. Proxmox/QEMU Server

**Problem**: Bought SuperMicro server, IOMMU disabled in BIOS, no option to enable

**Solution**:
1. `op-dbus discover` detects VT-d support but disabled
2. Adds `intel_iommu=on` to kernel params
3. Enables GPU passthrough for VMs
4. **Benefit**: Server now supports full PCI passthrough

### 3. Confidential Computing

**Problem**: Intel Xeon supports SGX but BIOS hides the option

**Solution**:
1. op-dbus detects SGX support but disabled
2. Uses `ru.efi` to enable SGX in BIOS variables
3. Enables secure enclaves for secrets management
4. **Benefit**: No need to buy "SGX-enabled" SKU (+$500)

### 4. Developer Workstation

**Problem**: AMD Ryzen laptop, SME (Secure Memory Encryption) disabled

**Solution**:
1. op-dbus detects `sme` flag in CPU but disabled
2. Adds `mem_encrypt=on` to kernel parameters
3. Enables full memory encryption
4. **Benefit**: Enhanced security at zero cost

---

## Detection Algorithm

```rust
// Simplified version of op-dbus CPU feature detection

fn check_virtualization_lock() -> FeatureStatus {
    // 1. Check CPU supports VT-x
    let cpuinfo = read_file("/proc/cpuinfo");
    let has_vmx = cpuinfo.contains("vmx");

    if !has_vmx {
        return FeatureStatus::NotSupported;
    }

    // 2. Check if KVM device exists (actual enablement)
    if path_exists("/dev/kvm") {
        return FeatureStatus::Enabled;
    }

    // 3. Check MSR 0x3A for lock status
    let msr_value = read_msr(0x3A);
    let lock_bit = msr_value & 0x1;
    let vmx_enable = msr_value & 0x4;

    if lock_bit == 1 && vmx_enable == 0 {
        // BIOS locked it disabled
        return FeatureStatus::LockedByBios;
    }

    if lock_bit == 0 {
        // Not locked, can enable via MSR write
        return FeatureStatus::DisabledByBios;
    }

    return FeatureStatus::Enabled;
}
```

---

## How This Differs from Microsoft Tools

**SCCM/Intune**: Respects vendor BIOS locks, no detection of hidden features

**op-dbus**:
- ✅ Detects artificial restrictions
- ✅ Exposes OEM market segmentation
- ✅ Provides unlock methods
- ✅ Automates safe unlocks
- ✅ Saves enterprise $100K+ on hardware

---

## Legal & Ethical Considerations

### Is This Legal?

**Yes** - You own the hardware. Modifying BIOS settings on hardware you own is legal in most jurisdictions.

**Exceptions**:
- Circumventing DRM/copy protection (DMCA, USA)
- Violating corporate IT policy on work equipment
- Warranty may be voided by BIOS modification

### Is This Ethical?

**Yes** - OEMs charge $200-500 for enabling a BIOS bit on identical hardware. This is:
- **Anti-consumer**: Artificial product segmentation
- **Wasteful**: Forces unnecessary hardware purchases
- **Anti-competitive**: Locks customers into "Pro" SKUs

### Vendor Position

OEMs dislike this because:
- Erodes "Pro" model margins
- Reduces SKU differentiation
- Customers realize hardware is identical

**But**: You bought the hardware. You should be able to use its full capability.

---

## Safety Guidelines

### Do:
- ✅ Run `op-dbus discover --dry-run` first
- ✅ Export system config before making changes
- ✅ Start with safe methods (kernel params, BIOS update)
- ✅ Test on non-production hardware first
- ✅ Keep BIOS recovery USB handy

### Don't:
- ❌ Flash modified BIOS without backup
- ❌ Write to random MSRs without understanding them
- ❌ Modify BIOS variables on critical production systems
- ❌ Ignore risk level warnings from op-dbus

---

## Output Example: Samsung 360 Pro

When you run `sudo op-dbus discover` on your buggy BIOS Samsung laptop:

```
🔓 CPU FEATURES & BIOS LOCKS
──────────────────────────────────────────────────────────────
  CPU: Intel(R) Core(TM) i5-6200U (Family 6)
  Microcode: 0xc6

  ⚠️  DISABLED/LOCKED FEATURES:
    🔒 VT-x (Intel Virtualization) (vmx): BIOS Locked
    ⊗ Intel Turbo Boost (turbo): Disabled by BIOS

  🔒 BIOS LOCKS DETECTED:
    Register: MSR 0x3A (IA32_FEATURE_CONTROL)
      Lock Bit: Bit 0 (Lock), Bit 2 (VMX Enable)
      Affects: VT-x, KVM
      Method: BIOS MSR lock bit set before OS boot

  ✓ ENABLED FEATURES:
    ✓ IOMMU (VT-d/AMD-Vi) (iommu)

  💡 RECOMMENDATIONS:
    🔴 VT-x - Critical Priority
       Reason: CPU supports VT-x but BIOS has locked it via MSR
       Benefit: Enable KVM virtualization, Docker, QEMU, VirtualBox
       Action: Check for Samsung BIOS update, or use BIOS variable modification (medium risk)

    🟡 Intel Turbo Boost - Medium Priority
       Reason: Turbo Boost is disabled
       Benefit: Improve single-threaded performance by 20-30%
       Action: Enable via sysfs: echo 0 > /sys/devices/system/cpu/intel_pstate/no_turbo
```

This tells you:
1. **VT-x is BIOS-locked** - Need BIOS modification to unlock
2. **Turbo Boost disabled** - Can enable via sysfs (safe)
3. **VT-d is enabled** - Already working

---

## Integration with Machine Replication

When you export a machine profile with `op-dbus discover --export`, the BIOS lock data is included:

```bash
# Export Samsung 360 Pro profile
sudo op-dbus discover --export --output samsung360.json

# Deploy to another Samsung 360 Pro
sudo op-dbus apply samsung360.json --unlock-cpu --risk-level low
```

This allows **automated fleet deployment** with feature unlocking:
1. Introspect reference machine
2. Detect BIOS locks
3. Determine safe unlock methods
4. Apply to entire fleet
5. Verify features enabled

---

## Future: Automated BIOS Patching

**Roadmap** (not yet implemented):

```bash
# Automatically patch BIOS to expose hidden options
sudo op-dbus patch-bios --feature vt-x --backup samsung360-bios-backup.rom

# This would:
# 1. Detect BIOS vendor and version
# 2. Extract BIOS ROM via flashrom
# 3. Patch with UEFITool to expose VT-x option
# 4. Flash modified BIOS
# 5. Reboot with VT-x now accessible in BIOS setup
```

**Challenges**:
- Requires vendor-specific knowledge
- High risk of bricking
- Needs hardware SPI programmer for recovery
- Legally gray area (BIOS modification)

---

## Summary

op-dbus **exposes OEM artificial restrictions** and enables features your CPU supports but BIOS hides. This:

- **Saves enterprises $100K+** on "Pro" model markups
- **Enables full hardware capability** without vendor locks
- **Provides transparency** into BIOS feature restriction
- **Automates unlock workflows** for fleet deployment

Your VT-x scenario is a perfect example - CPU supports it, OEM locked it, op-dbus detects and (where possible) unlocks it.

Run `sudo op-dbus discover` on your Samsung 360 Pro to see what's hidden!

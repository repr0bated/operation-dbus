# BTRFS Subvolume Management

## Overview

The install script intelligently manages BTRFS subvolumes for blockchain storage to prevent accumulation of orphaned subvolumes.

## Problem Solved

**Without Management:**
```bash
$ sudo btrfs subvolume list /
...
ID 100 path @var/lib/op-dbus/blockchain
ID 101 path @var/lib/op-dbus/blockchain  # Duplicate!
ID 102 path @var/lib/op-dbus/blockchain  # Another!
ID 103 path @blockchain/op-dbus          # Old location!
```

**With Management:**
- Detects existing subvolumes
- Offers to reuse or clean up
- Prevents million subvolumes problem

## Install Script Behavior

### Detection Phase

**File:** `install.sh:68-145`

```bash
# 1. Detect BTRFS filesystem
if df -T /var/lib | grep -q btrfs; then
    # System uses BTRFS

# 2. Search for existing subvolumes
EXISTING_SUBVOLS=$(sudo btrfs subvolume list / | grep -E "@var/lib/op-dbus/blockchain|@blockchain/op-dbus")

# 3. If found, ask user
if [ -n "$EXISTING_SUBVOLS" ]; then
    echo "Found existing blockchain subvolumes:"
    echo "$EXISTING_SUBVOLS"
    read -p "Reuse existing subvolumes? [Y/n]"
```

### Three Scenarios

#### Scenario 1: Fresh Install (No Existing Subvolumes)
```
Installing op-dbus...
Setting up blockchain storage...
Detected BTRFS filesystem
Created blockchain BTRFS subvolume
✓ Created: /var/lib/op-dbus/blockchain
```

**Result:**
- New subvolume created
- Clean installation

#### Scenario 2: Reinstall (Reuse Existing)
```
Installing op-dbus...
Setting up blockchain storage...
Detected BTRFS filesystem
⚠ Found existing blockchain subvolumes:
  ID 290 gen 46763 path @var/lib/blockchain/timing
  ID 291 gen 46763 path @var/lib/blockchain/vectors
Reuse existing subvolumes? [Y/n] Y
✓ Reusing existing blockchain subvolumes
✓ Blockchain subvolume already exists
```

**Result:**
- Existing subvolume reused
- Blockchain data preserved
- No duplicate subvolumes created

#### Scenario 3: Reinstall (Clean Start)
```
Installing op-dbus...
Setting up blockchain storage...
Detected BTRFS filesystem
⚠ Found existing blockchain subvolumes:
  ID 290 gen 46763 path @var/lib/blockchain/timing
Reuse existing subvolumes? [Y/n] n
Cleaning up old subvolumes...
✓ Cleared old blockchain data
✓ Blockchain subvolume already exists
```

**Result:**
- Old data cleared (files deleted)
- Subvolume structure reused
- No orphaned subvolumes

### Special Case: Converting Regular Directory

**If blockchain directory exists as regular directory:**

```bash
⚠ /var/lib/op-dbus/blockchain exists as regular directory with files
⚠ Converting to BTRFS subvolume...
✓ Converted to BTRFS subvolume with data preserved
```

**How it works:**
```bash
# Move existing data
mv /var/lib/op-dbus/blockchain /tmp/backup

# Create subvolume
btrfs subvolume create /var/lib/op-dbus/blockchain

# Restore data
mv /tmp/backup/* /var/lib/op-dbus/blockchain/
```

## Non-BTRFS Systems

**If not using BTRFS:**
```
Setting up blockchain storage...
Using regular directory (not BTRFS)
✓ Created blockchain directory: /var/lib/op-dbus/blockchain
```

**Behavior:**
- Standard directory creation
- No subvolume management
- Works on ext4, xfs, etc.

## Uninstall Script Behavior

### Cleanup Options

**File:** `uninstall.sh:59-106`

```bash
# 1. Ask about blockchain data
Blockchain data: /var/lib/op-dbus/blockchain
⚠ This is a BTRFS subvolume
Remove blockchain data? [y/N]
```

#### Option A: Remove Data
```bash
Remove blockchain data? [y/N] y
Deleting BTRFS subvolume...
✓ Blockchain subvolume deleted
✓ Removed empty /var/lib/op-dbus
```

**Result:**
- Subvolume deleted properly
- No orphaned structures
- Clean removal

#### Option B: Preserve Data
```bash
Remove blockchain data? [y/N] n
⚠ Blockchain data preserved at /var/lib/op-dbus/blockchain
```

**Result:**
- Data kept for reinstall
- Can be reused by next install

### Detection of Orphaned Subvolumes

**After uninstall:**
```bash
Remaining op-dbus subvolumes:
  ID 290 gen 46763 path @var/lib/blockchain/timing
⚠ Clean these up manually if needed:
    sudo btrfs subvolume delete /path/to/subvolume
```

**Helps user find forgotten subvolumes**

## Manual Subvolume Management

### List All Subvolumes
```bash
# All subvolumes
sudo btrfs subvolume list /

# Only op-dbus related
sudo btrfs subvolume list / | grep op-dbus
```

### Check Specific Path
```bash
# Is this a subvolume?
sudo btrfs subvolume show /var/lib/op-dbus/blockchain
```

### Delete Orphaned Subvolume
```bash
# Delete specific subvolume
sudo btrfs subvolume delete /var/lib/op-dbus/blockchain

# Delete with snapshot handling
sudo btrfs subvolume delete --commit-after /path/to/subvolume
```

### Find Orphaned Subvolumes
```bash
# List all op-dbus subvolumes
sudo btrfs subvolume list / | grep -E "op-dbus|blockchain" | grep -v "@var/lib/op-dbus/blockchain"

# These are orphans if not the current path
```

## Best Practices

### For Users

**1. Reinstalling:**
```bash
# Reuse existing data (recommended)
sudo ./install.sh
# Press Y when asked about existing subvolumes
```

**2. Fresh Start:**
```bash
# Clean install
sudo ./uninstall.sh
# Remove blockchain data: Y

sudo ./install.sh
# No existing subvolumes, fresh start
```

**3. Migrating Data:**
```bash
# Backup before cleaning
sudo cp -a /var/lib/op-dbus/blockchain /backup/

# Clean install
sudo ./uninstall.sh  # Remove data
sudo ./install.sh    # Fresh subvolume

# Restore data if needed
sudo cp -a /backup/blockchain/* /var/lib/op-dbus/blockchain/
```

### For Developers

**Don't create subvolumes manually:**
```bash
# Bad - creates orphan
sudo btrfs subvolume create /var/lib/op-dbus/blockchain

# Good - let install script manage
sudo ./install.sh
```

**Always delete subvolumes properly:**
```bash
# Bad - leaves subvolume structure
sudo rm -rf /var/lib/op-dbus/blockchain/*

# Good - removes subvolume
sudo btrfs subvolume delete /var/lib/op-dbus/blockchain
```

## Troubleshooting

### Problem: Can't Delete Directory
```bash
$ rm -rf /var/lib/op-dbus/blockchain
rm: cannot remove '/var/lib/op-dbus/blockchain': Operation not permitted
```

**Cause:** It's a BTRFS subvolume

**Solution:**
```bash
sudo btrfs subvolume delete /var/lib/op-dbus/blockchain
```

### Problem: Multiple Subvolumes Exist
```bash
$ sudo btrfs subvolume list / | grep op-dbus
ID 100 path @var/lib/op-dbus/blockchain
ID 200 path @blockchain/op-dbus-old
ID 300 path @var/lib/blockchain/op-dbus
```

**Solution:**
```bash
# Delete old/unused ones
sudo btrfs subvolume delete /var/lib/blockchain/op-dbus
sudo btrfs subvolume delete /blockchain/op-dbus-old

# Keep only: /var/lib/op-dbus/blockchain
```

### Problem: Subvolume Not Detected
```bash
Installing op-dbus...
Using regular directory (not BTRFS)
```

**Cause:** `/var/lib` not on BTRFS, or BTRFS not detected

**Check:**
```bash
df -T /var/lib
# Should show: btrfs

# If not, your system uses different filesystem
# This is OK - will use regular directory
```

### Problem: Permission Denied
```bash
$ ./install.sh
Error: Cannot create subvolume: Permission denied
```

**Solution:**
```bash
# Must run as root
sudo ./install.sh
```

## Technical Details

### Subvolume Creation
```bash
# Create subvolume
sudo btrfs subvolume create /var/lib/op-dbus/blockchain

# Verify
sudo btrfs subvolume show /var/lib/op-dbus/blockchain
# Output:
# /var/lib/op-dbus/blockchain
#   Name:           blockchain
#   UUID:           <uuid>
#   Parent UUID:    -
#   Received UUID:  -
#   Creation time:  2025-10-20 ...
```

### Subvolume Properties
- **Independent inode space** - Can have own quotas
- **Snapshot-able** - Can create instant snapshots
- **Deletable as unit** - Remove all files at once
- **Mount-able** - Can mount separately

### Benefits for Blockchain Data
1. **Snapshots** - Quick backup before updates
2. **Rollback** - Revert to previous state
3. **Quotas** - Limit blockchain size
4. **Atomic Delete** - Clean removal

### Example: Using Snapshots
```bash
# Before upgrade, snapshot blockchain
sudo btrfs subvolume snapshot /var/lib/op-dbus/blockchain \
    /var/lib/op-dbus/blockchain-backup

# Upgrade
sudo ./install.sh

# If something wrong, rollback
sudo mv /var/lib/op-dbus/blockchain /var/lib/op-dbus/blockchain-broken
sudo mv /var/lib/op-dbus/blockchain-backup /var/lib/op-dbus/blockchain
```

## Summary

### Install Script
✅ **Detects BTRFS** - Automatic detection
✅ **Finds existing subvolumes** - Prevents duplicates
✅ **Asks user preference** - Reuse or clean
✅ **Converts if needed** - Regular dir → subvolume
✅ **Works without BTRFS** - Falls back to regular directory

### Uninstall Script
✅ **Detects subvolumes** - Knows the difference
✅ **Proper deletion** - Uses `btrfs subvolume delete`
✅ **Lists orphans** - Helps cleanup
✅ **Preserves option** - Keep data for reinstall

### User Benefits
✅ **No million subvolumes** - Intelligent reuse
✅ **Clean installs** - No orphaned structures
✅ **Data preservation** - Option to keep blockchain
✅ **Works everywhere** - BTRFS and non-BTRFS systems

**Problem solved!**

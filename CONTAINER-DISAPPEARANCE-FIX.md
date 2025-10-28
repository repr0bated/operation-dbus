# Container Disappearance Fix

## Problem
Containers were being created and started, but then disappearing. Investigation revealed:
1. Containers were created successfully via `pct create`
2. Containers were started successfully via `pct start`
3. The `find_container_veth()` function was failing to locate the veth interface
4. With the veth not found, the network enrollment process couldn't complete
5. Containers were left in an orphaned state (stopped but not properly configured)

## Root Causes

### Primary Issue: Missing Template
Containers were failing to create because the default template `debian-13-netmaker_custom.tar.zst` **does not exist**. The actual available template is `debian-13-standard_13.1-2_amd64.tar.zst`.

When containers were created, they had **no rootfs** directory, causing them to crash immediately on startup (~30 seconds).

### Secondary Issue: Flawed Veth Detection Logic
The `find_container_veth()` function in `src/state/plugins/lxc.rs` had **flawed logic**:

### Original Implementation (BROKEN)
```rust
// Lines 218-219: Assumed wrong naming pattern
if line.contains("eth0") {
    return Ok(format!("veth{}", ct_id));  // WRONG: assumes vethXXX
}
```

The code assumed Proxmox creates veth interfaces with the pattern `veth{VMID}`, but Proxmox actually generates **random names** like `vethRANDOM`.

### Why Containers Disappeared
1. Container created ✓
2. Container started ✓
3. Wait 2 seconds for veth to appear ✓
4. `find_container_veth()` called
5. Function tries to find veth with pattern `veth100` (for example)
6. **FAILS** because actual interface is `veth0x1a2b3c` (random)
7. Error logged but container left running
8. Subsequent operations fail because veth was never found
9. Container appears "disappeared" because it's in a broken state

## Fix Applied

### 1. Fixed Template Reference

Updated the default template to match what's actually available on the system:

```rust
// Line 283: Fixed template path
unwrap_or("local-btrfs:vztmpl/debian-13-standard_13.1-2_amd64.tar.zst");
```

**Before:**
- Referenced non-existent template: `debian-13-netmaker_custom.tar.zst`
- Result: Container created without rootfs, crashed immediately

**After:**
- Uses actual template: `debian-13-standard_13.1-2_amd64.tar.zst`
- Result: Container has proper filesystem, stays running

### 2. Improved Veth Detection Logic
Updated `find_container_veth()` to properly detect the actual veth interface name:

```rust
// Lines 175-220: Proper detection from container namespace
let output = tokio::process::Command::new("ip")
    .args([
        "netns",
        "exec",
        &format!("ct{}", ct_id),
        "ip",
        "link",
        "show",
        "eth0",
    ])
    .output()
    .await?;

// Parse the peer link index and match to host-side veth
// Iterate through host veth interfaces to find matching peer
```

**Changes:**
- Inspects container's eth0 interface to get peer information
- Scans host-side veth interfaces looking for the matching peer
- Extracts the actual interface name from the `@if` notation
- Adds fallback to find ANY veth interface if peer matching fails

### 3. Cleanup on Failure
Added automatic cleanup when veth detection fails:

```rust
// Lines 510-521: Stop container if veth not found
Err(e) => {
    // If we can't find the veth, stop the container to prevent orphan
    log::warn!(
        "Failed to find veth for container {}, stopping container",
        container.id
    );
    let _ = Self::stop_container(&container.id).await;
    errors.push(format!(
        "Failed to find veth for container {}: {}",
        container.id, e
    ));
}
```

### 4. Added Stop Container Function
Created dedicated `stop_container()` helper:

```rust
// Lines 343-356: Stop LXC container
async fn stop_container(ct_id: &str) -> Result<()> {
    let output = tokio::process::Command::new("pct")
        .args(["stop", ct_id])
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("pct stop failed: {}", stderr));
    }

    Ok(())
}
```

## Testing

### Verify Fix
```bash
# Build the updated code
cd /git/operation-dbus
cargo build --release

# Test container creation
sudo ./target/release/op-dbus apply /etc/op-dbus/state.json

# Check that containers now stay visible
pct list
```

### Expected Behavior
1. Container created ✓
2. Container started ✓
3. Veth interface **properly detected** ✓
4. Veth renamed to `vi{VMID}` ✓
5. Veth added to bridge (mesh or vmbr0) ✓
6. Container remains running and accessible ✓

### Cleanup Orphaned Containers
If you have existing orphaned containers:

```bash
# List containers
pct list

# Stop and destroy orphaned containers
sudo pct stop 100
sudo pct destroy 100

# Repeat for other orphaned containers
```

## Files Modified
- `src/state/plugins/lxc.rs`:
  - Line 283: Fixed template path to use actual template
  - Lines 173-251: Improved `find_container_veth()` detection logic
  - Lines 343-356: Added `stop_container()` helper function
  - Lines 510-521: Added cleanup on veth detection failure

## Impact
- **Prevents**: Containers from disappearing
- **Fixes**: Veth interface detection for Proxmox containers
- **Improves**: Error handling and cleanup on failure
- **Status**: Build successful, ready for testing

## Next Steps
1. Test container creation with real state file
2. Verify veth interfaces are properly detected
3. Confirm containers remain running after creation
4. Monitor logs for any remaining issues


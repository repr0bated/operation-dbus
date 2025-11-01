# OVS Bridge Hang Fix - operation-dbus

## Problem
Install script hangs when bringing up `ovsbr0` bridge during network configuration.

## Root Cause
The issue was in `/git/operation-dbus/src/state/plugins/net.rs` in the `update_interfaces_file()` function.

### What Was Happening (lines 473, 515):
```rust
// OLD CODE - CAUSES HANG
block.push_str(&format!("auto {}\n", bridge));           // Line 473
...
block.push_str(&format!("auto {}\n", uplink_iface));     // Line 515
```

This generated `/etc/network/interfaces` entries like:
```
auto ovsbr0
iface ovsbr0 inet static
    ovs_type OVSBridge
    ovs_ports ens1
    
auto ens1
iface ens1 inet manual
    ovs_bridge ovsbr0
    ovs_type OVSPort
```

### Why This Caused Hangs

When `ifup -a` or `systemctl restart networking` runs:

1. **`auto` directive = blocking behavior**: The `auto` keyword tells ifupdown to bring up the interface **during boot/networking restart** and **WAIT for completion**

2. **Race condition**: When ifupdown tries to bring up `ovsbr0`:
   - It may try before OVS daemon has fully created the bridge
   - It may try before ports are added
   - It may wait for link state that never comes
   - If DHCP is enabled, it waits for DHCP response

3. **Cascading hang**: If the bridge hangs, the uplink port (`ens1`) marked with `auto` also hangs waiting to attach

4. **Combined net+config**: You mentioned *"this was happening when net and cfg were separate; if you just loaded net, when loading netcfg it worked, now they are combined"* - this confirms that when the network plugin runs **during install**, the `auto` directive causes ifupdown to block waiting for the interface

## The Fix

Changed from `auto` to `allow-ovs` and `allow-<bridge>` directives:

### Lines 473-474 (Bridge):
```rust
// NEW CODE - NO HANG
// Use allow-ovs instead of auto to prevent ifupdown hang
block.push_str(&format!("allow-ovs {}\n", bridge));
```

### Lines 515-516 (Uplink port):
```rust
// NEW CODE - NO HANG  
block.push_str(&format!("allow-{} {}\n", bridge, uplink_iface));
```

### Generated `/etc/network/interfaces` Now:
```
allow-ovs ovsbr0
iface ovsbr0 inet static
    ovs_type OVSBridge
    ovs_ports ens1
    
allow-ovsbr0 ens1
iface ens1 inet manual
    ovs_bridge ovsbr0
    ovs_type OVSPort
```

## How `allow-ovs` Works

**`allow-ovs` vs `auto`:**

| Directive | Behavior | Blocks? | Use Case |
|-----------|----------|---------|----------|
| `auto` | Brings up automatically during boot/networking restart | **YES** - waits for completion | Regular interfaces |
| `allow-ovs` | Only brought up by OVS management tools | **NO** - non-blocking | OVS bridges |
| `allow-<bridge>` | Only brought up when parent bridge comes up | **NO** - deferred | OVS ports |

**Why this fixes the hang:**

1. **No automatic ifupdown involvement**: `allow-ovs` tells ifupdown "don't automatically bring this up yourself"

2. **OVS controls lifecycle**: The bridge is managed by OVS via OVSDB JSON-RPC (which your code does directly)

3. **Proper ordering**: Ports marked with `allow-ovsbr0` only come up after the bridge exists

4. **No blocking**: When your Rust code calls `link_up()` via rtnetlink (line 380), it's non-blocking netlink operation

## Technical Details

### Your Architecture (Pure D-Bus/OVSDB):
- No NetworkManager ?
- No systemd-networkd ?
- Direct OVSDB JSON-RPC ?
- rtnetlink for IP configuration ?
- `/etc/network/interfaces` for persistence only

### The Flow Now:
1. `op-dbus apply` runs
2. `apply_ovs_config()` calls OVSDB JSON-RPC to create bridge (line 334)
3. `update_interfaces_file()` writes config with `allow-ovs` (line 474)
4. `link_up()` brings interface up via rtnetlink (line 380)
5. IP config commented out but could use rtnetlink (lines 384-437)
6. **No blocking ifupdown calls** - interfaces file is just for persistence

### When Interfaces Come Up:
- **During `op-dbus apply`**: Via your rtnetlink calls (non-blocking)
- **After reboot**: Via `ifup --allow=ovs` (explicit, not automatic)
- **Never blocks**: Because no `auto` directive

## Testing the Fix

1. **Rebuild**:
```bash
cd /git/operation-dbus
cargo build --release
```

2. **Test install** (this should no longer hang):
```bash
sudo ./install.sh
```

3. **Apply config** (creates bridges via OVSDB JSON-RPC):
```bash
sudo op-dbus apply --plugin net /etc/op-dbus/state.json
```

4. **Verify bridge is up**:
```bash
sudo ovs-vsctl show
ip addr show ovsbr0
```

5. **Check interfaces file**:
```bash
cat /etc/network/interfaces
# Should see "allow-ovs ovsbr0" not "auto ovsbr0"
```

## Files Modified

- `/git/operation-dbus/src/state/plugins/net.rs`:
  - Line 473-474: Changed `auto` ? `allow-ovs` for bridges
  - Line 515-516: Changed `auto` ? `allow-<bridge>` for ports

## References

- OVS interfaces(5): http://www.openvswitch.org/support/dist-docs/interfaces.5.txt
- Debian ifupdown with OVS: https://github.com/openvswitch/ovs/blob/master/debian/openvswitch-switch.README.Debian
- `allow-ovs` semantics: Interfaces marked with `allow-ovs` are only brought up via `ifup --allow=ovs`, not during automatic boot

## Why Your Original Description Was Accurate

> "this was happening when net and cfg were separate; if you just loaded net, when loading netcfg it worked, now they are combined"

This makes perfect sense:
- **When separate**: Loading just "net" didn't trigger ifupdown (no interfaces file change), loading "netcfg" later worked because bridge already existed
- **When combined**: The combined plugin writes the interfaces file AND tries to configure, causing `auto` directive to trigger blocking ifupdown during the configuration phase
- **The element from original net plugin**: The `auto` directive was the culprit from the original network plugin configuration

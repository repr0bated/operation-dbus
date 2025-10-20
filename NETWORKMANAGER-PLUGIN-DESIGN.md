# NetworkManager Plugin Design & Analysis

**STATUS: REFERENCE ONLY - NOT PLANNED FOR IMPLEMENTATION**

op-dbus is **server-focused** and will continue using the OVS/Netlink approach exclusively. This document exists as reference material for:
- Understanding why we chose OVS/Netlink over NetworkManager
- Future contributors who might want to fork for desktop use cases
- Technical comparison for documentation purposes

Design document for a potential NetworkManager plugin and comparison with current OVS/Netlink approach.

## NetworkManager Plugin Architecture

### D-Bus API Structure

NetworkManager exposes comprehensive D-Bus API at `org.freedesktop.NetworkManager`:

```
org.freedesktop.NetworkManager
├─ /org/freedesktop/NetworkManager
│  ├─ .Manager (main interface)
│  ├─ .Settings (connection profiles)
│  └─ .AgentManager (secrets/authentication)
├─ /org/freedesktop/NetworkManager/Devices/{n}
│  ├─ .Device (generic device)
│  ├─ .Device.Wired (ethernet)
│  ├─ .Device.Wireless (WiFi)
│  ├─ .Device.Bridge (bridge device)
│  ├─ .Device.Bond (bonding)
│  └─ .Device.Vlan (VLANs)
├─ /org/freedesktop/NetworkManager/Settings/{n}
│  └─ .Connection (stored connection profile)
└─ /org/freedesktop/NetworkManager/ActiveConnection/{n}
   └─ .ActiveConnection (active connection state)
```

### Plugin Implementation

```rust
// src/state/plugins/networkmanager.rs

use crate::state::plugin::*;
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use zbus::Connection;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkManagerState {
    pub connections: Vec<ConnectionProfile>,
    pub devices: Vec<Device>,
    pub active_connections: Vec<ActiveConnection>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConnectionProfile {
    pub id: String,
    pub uuid: String,
    pub type_: String,  // "802-3-ethernet", "bridge", "bond", "wifi", etc.
    pub autoconnect: bool,

    // Connection-specific settings
    #[serde(flatten)]
    pub settings: ConnectionSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum ConnectionSettings {
    Ethernet {
        interface: String,
        ipv4: Option<IPv4Config>,
        ipv6: Option<IPv6Config>,
    },
    Bridge {
        interface: String,
        bridge_ports: Vec<String>,
        stp: bool,
        ipv4: Option<IPv4Config>,
    },
    Bond {
        interface: String,
        mode: String,  // "balance-rr", "active-backup", "802.3ad", etc.
        slaves: Vec<String>,
    },
    WiFi {
        ssid: String,
        mode: String,  // "infrastructure", "ap", "adhoc"
        security: Option<WiFiSecurity>,
    },
    Vlan {
        parent: String,
        id: u16,
        ipv4: Option<IPv4Config>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IPv4Config {
    pub method: String,  // "auto" (DHCP), "manual", "link-local", "disabled"
    pub addresses: Vec<AddressData>,
    pub gateway: Option<String>,
    pub dns: Vec<String>,
    pub routes: Vec<RouteData>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IPv6Config {
    pub method: String,  // "auto", "manual", "link-local", "disabled"
    pub addresses: Vec<AddressData>,
    pub gateway: Option<String>,
    pub dns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AddressData {
    pub address: String,
    pub prefix: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RouteData {
    pub dest: String,
    pub prefix: u8,
    pub next_hop: String,
    pub metric: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WiFiSecurity {
    pub key_mgmt: String,  // "none", "wpa-psk", "wpa-eap"
    pub auth_alg: Option<String>,
    // Note: passwords stored separately via SecretAgent
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Device {
    pub interface: String,
    pub device_type: String,
    pub hw_address: Option<String>,
    pub state: String,  // "unavailable", "disconnected", "connected", etc.
    pub driver: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ActiveConnection {
    pub id: String,
    pub uuid: String,
    pub state: String,  // "activating", "activated", "deactivating", etc.
    pub devices: Vec<String>,
    pub default: bool,
    pub default6: bool,
}

pub struct NetworkManagerPlugin {
    connection: Connection,
}

impl NetworkManagerPlugin {
    pub async fn new() -> Result<Self> {
        let connection = Connection::system().await?;
        Ok(Self { connection })
    }

    /// Query all connection profiles from NetworkManager
    async fn query_connections(&self) -> Result<Vec<ConnectionProfile>> {
        let proxy = zbus::Proxy::new(
            &self.connection,
            "org.freedesktop.NetworkManager",
            "/org/freedesktop/NetworkManager/Settings",
            "org.freedesktop.NetworkManager.Settings",
        )
        .await?;

        // Get list of connection paths
        let connections: Vec<zbus::zvariant::OwnedObjectPath> =
            proxy.call("ListConnections", &()).await?;

        let mut profiles = Vec::new();
        for conn_path in connections {
            // Get connection settings
            let conn_proxy = zbus::Proxy::new(
                &self.connection,
                "org.freedesktop.NetworkManager",
                conn_path.as_str(),
                "org.freedesktop.NetworkManager.Settings.Connection",
            )
            .await?;

            let settings: std::collections::HashMap<
                String,
                std::collections::HashMap<String, zbus::zvariant::OwnedValue>,
            > = conn_proxy.call("GetSettings", &()).await?;

            // Parse settings into ConnectionProfile
            let profile = self.parse_connection_settings(settings)?;
            profiles.push(profile);
        }

        Ok(profiles)
    }

    /// Query all network devices
    async fn query_devices(&self) -> Result<Vec<Device>> {
        let proxy = zbus::Proxy::new(
            &self.connection,
            "org.freedesktop.NetworkManager",
            "/org/freedesktop/NetworkManager",
            "org.freedesktop.NetworkManager",
        )
        .await?;

        let device_paths: Vec<zbus::zvariant::OwnedObjectPath> =
            proxy.get_property("Devices").await?;

        let mut devices = Vec::new();
        for dev_path in device_paths {
            let dev_proxy = zbus::Proxy::new(
                &self.connection,
                "org.freedesktop.NetworkManager",
                dev_path.as_str(),
                "org.freedesktop.NetworkManager.Device",
            )
            .await?;

            let interface: String = dev_proxy.get_property("Interface").await?;
            let device_type: u32 = dev_proxy.get_property("DeviceType").await?;
            let hw_address: Option<String> = dev_proxy.get_property("HwAddress").await.ok();
            let state: u32 = dev_proxy.get_property("State").await?;

            devices.push(Device {
                interface,
                device_type: self.device_type_to_string(device_type),
                hw_address,
                state: self.device_state_to_string(state),
                driver: dev_proxy.get_property("Driver").await.ok(),
            });
        }

        Ok(devices)
    }

    /// Create or update a connection profile
    async fn apply_connection(&self, profile: &ConnectionProfile) -> Result<()> {
        let settings_proxy = zbus::Proxy::new(
            &self.connection,
            "org.freedesktop.NetworkManager",
            "/org/freedesktop/NetworkManager/Settings",
            "org.freedesktop.NetworkManager.Settings",
        )
        .await?;

        // Convert ConnectionProfile to NM settings format
        let settings = self.profile_to_nm_settings(profile)?;

        // Check if connection exists
        let existing = self.find_connection_by_uuid(&profile.uuid).await?;

        if let Some(conn_path) = existing {
            // Update existing connection
            let conn_proxy = zbus::Proxy::new(
                &self.connection,
                "org.freedesktop.NetworkManager",
                conn_path.as_str(),
                "org.freedesktop.NetworkManager.Settings.Connection",
            )
            .await?;

            conn_proxy.call("Update", &(settings,)).await?;
            log::info!("Updated connection: {}", profile.id);
        } else {
            // Create new connection
            settings_proxy.call("AddConnection", &(settings,)).await?;
            log::info!("Created connection: {}", profile.id);
        }

        // Activate connection if autoconnect is true
        if profile.autoconnect {
            self.activate_connection(&profile.uuid).await?;
        }

        Ok(())
    }

    /// Activate a connection
    async fn activate_connection(&self, uuid: &str) -> Result<()> {
        let nm_proxy = zbus::Proxy::new(
            &self.connection,
            "org.freedesktop.NetworkManager",
            "/org/freedesktop/NetworkManager",
            "org.freedesktop.NetworkManager",
        )
        .await?;

        let conn_path = self.find_connection_by_uuid(uuid).await?
            .ok_or_else(|| anyhow::anyhow!("Connection not found: {}", uuid))?;

        // Activate on appropriate device (or "/" for any device)
        nm_proxy
            .call("ActivateConnection", &(conn_path, "/", "/"))
            .await?;

        Ok(())
    }

    fn parse_connection_settings(
        &self,
        settings: std::collections::HashMap<
            String,
            std::collections::HashMap<String, zbus::zvariant::OwnedValue>,
        >,
    ) -> Result<ConnectionProfile> {
        // Parse NM settings dict into ConnectionProfile
        // (implementation details omitted for brevity)
        todo!("Parse NM settings")
    }

    fn profile_to_nm_settings(
        &self,
        profile: &ConnectionProfile,
    ) -> Result<
        std::collections::HashMap<
            String,
            std::collections::HashMap<String, zbus::zvariant::Value>,
        >,
    > {
        // Convert ConnectionProfile to NM settings format
        // (implementation details omitted for brevity)
        todo!("Convert to NM settings")
    }

    async fn find_connection_by_uuid(&self, uuid: &str) -> Result<Option<String>> {
        // Find connection by UUID
        // (implementation details omitted for brevity)
        todo!("Find connection")
    }

    fn device_type_to_string(&self, type_: u32) -> String {
        match type_ {
            1 => "ethernet".to_string(),
            2 => "wifi".to_string(),
            5 => "bluetooth".to_string(),
            13 => "bridge".to_string(),
            14 => "vlan".to_string(),
            _ => format!("unknown-{}", type_),
        }
    }

    fn device_state_to_string(&self, state: u32) -> String {
        match state {
            10 => "unmanaged".to_string(),
            20 => "unavailable".to_string(),
            30 => "disconnected".to_string(),
            100 => "activated".to_string(),
            _ => format!("state-{}", state),
        }
    }
}

#[async_trait]
impl StatePlugin for NetworkManagerPlugin {
    fn name(&self) -> &str {
        "networkmanager"
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    async fn query_current_state(&self) -> Result<Value> {
        let connections = self.query_connections().await?;
        let devices = self.query_devices().await?;
        let active_connections = Vec::new(); // TODO: query active connections

        Ok(serde_json::to_value(NetworkManagerState {
            connections,
            devices,
            active_connections,
        })?)
    }

    async fn calculate_diff(&self, current: &Value, desired: &Value) -> Result<StateDiff> {
        // Compare current and desired state
        // Generate actions for connection changes
        todo!("Calculate diff")
    }

    async fn apply_state(&self, diff: &StateDiff) -> Result<ApplyResult> {
        let mut changes_applied = Vec::new();
        let mut errors = Vec::new();

        for action in &diff.actions {
            match action {
                StateAction::Create { resource: _, config } => {
                    let profile: ConnectionProfile = serde_json::from_value(config.clone())?;
                    match self.apply_connection(&profile).await {
                        Ok(_) => changes_applied.push(format!("Created connection: {}", profile.id)),
                        Err(e) => errors.push(format!("Failed to create {}: {}", profile.id, e)),
                    }
                }
                StateAction::Modify { resource: _, changes } => {
                    let profile: ConnectionProfile = serde_json::from_value(changes.clone())?;
                    match self.apply_connection(&profile).await {
                        Ok(_) => changes_applied.push(format!("Updated connection: {}", profile.id)),
                        Err(e) => errors.push(format!("Failed to update {}: {}", profile.id, e)),
                    }
                }
                StateAction::Delete { resource } => {
                    // Delete connection by UUID
                    changes_applied.push(format!("Deleted connection: {}", resource));
                }
                StateAction::NoOp { .. } => {}
            }
        }

        Ok(ApplyResult {
            success: errors.is_empty(),
            changes_applied,
            errors,
            checkpoint: None,
        })
    }

    async fn verify_state(&self, _desired: &Value) -> Result<bool> {
        Ok(true)
    }

    async fn create_checkpoint(&self) -> Result<Checkpoint> {
        // NetworkManager has built-in checkpoint support!
        let nm_proxy = zbus::Proxy::new(
            &self.connection,
            "org.freedesktop.NetworkManager",
            "/org/freedesktop/NetworkManager",
            "org.freedesktop.NetworkManager",
        )
        .await?;

        // Create NM checkpoint (timeout in seconds, can be rolled back)
        let checkpoint_path: zbus::zvariant::OwnedObjectPath = nm_proxy
            .call("CheckpointCreate", &(Vec::<String>::new(), 600u32, 1u32))
            .await?;

        Ok(Checkpoint {
            id: checkpoint_path.to_string(),
            plugin: self.name().into(),
            timestamp: chrono::Utc::now().timestamp(),
            state_snapshot: serde_json::json!({}),
            backend_checkpoint: Some(checkpoint_path.to_string()),
        })
    }

    async fn rollback(&self, checkpoint: &Checkpoint) -> Result<()> {
        if let Some(checkpoint_path) = &checkpoint.backend_checkpoint {
            let nm_proxy = zbus::Proxy::new(
                &self.connection,
                "org.freedesktop.NetworkManager",
                "/org/freedesktop/NetworkManager",
                "org.freedesktop.NetworkManager",
            )
            .await?;

            nm_proxy
                .call("CheckpointRollback", &(checkpoint_path,))
                .await?;

            log::info!("Rolled back NetworkManager to checkpoint: {}", checkpoint_path);
        }

        Ok(())
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            supports_rollback: true,  // NM has native checkpoints!
            supports_checkpoints: true,
            supports_verification: true,
            atomic_operations: true,  // NM handles atomicity
        }
    }
}
```

### State File Example

```json
{
  "version": 1,
  "plugins": {
    "networkmanager": {
      "connections": [
        {
          "id": "eth0-static",
          "uuid": "a1b2c3d4-e5f6-1234-5678-90abcdef1234",
          "type": "ethernet",
          "autoconnect": true,
          "interface": "eth0",
          "ipv4": {
            "method": "manual",
            "addresses": [
              {"address": "192.168.1.100", "prefix": 24}
            ],
            "gateway": "192.168.1.1",
            "dns": ["8.8.8.8", "8.8.4.4"]
          }
        },
        {
          "id": "office-wifi",
          "uuid": "b2c3d4e5-f6a7-2345-6789-01bcdef23456",
          "type": "wifi",
          "autoconnect": true,
          "ssid": "OfficeNetwork",
          "mode": "infrastructure",
          "security": {
            "key_mgmt": "wpa-psk"
          }
        },
        {
          "id": "br0",
          "uuid": "c3d4e5f6-a7b8-3456-7890-12cdef345678",
          "type": "bridge",
          "autoconnect": true,
          "interface": "br0",
          "bridge_ports": ["eth1", "eth2"],
          "stp": true,
          "ipv4": {
            "method": "manual",
            "addresses": [
              {"address": "10.0.0.1", "prefix": 24}
            ]
          }
        }
      ]
    }
  }
}
```

## Comparison: NetworkManager vs OVS/Netlink

### Current Approach: OVS + Netlink (Direct)

**What it does:**
- OVSDB JSON-RPC: Direct communication with OpenVSwitch daemon
- Netlink: Direct kernel API for IP addresses, routes, links
- No abstraction layer, talks directly to the source

**Architecture:**
```
op-dbus
  ↓ (native)
OVSDB socket (/var/run/openvswitch/db.sock)
  ↓
ovs-vswitchd
  ↓
Linux kernel

op-dbus
  ↓ (native)
Netlink socket
  ↓
Linux kernel
```

### NetworkManager Approach

**What it does:**
- D-Bus API: Talk to NetworkManager daemon
- NetworkManager manages configuration
- NetworkManager talks to backends (wpa_supplicant, DHCP, etc.)

**Architecture:**
```
op-dbus
  ↓ (D-Bus)
NetworkManager
  ↓
Multiple backends:
  - wpa_supplicant (WiFi)
  - dhclient/dhcpcd (DHCP)
  - dnsmasq (DNS)
  - iptables (firewall)
  - OpenVSwitch (optional)
  - Direct netlink (fallback)
```

## Pros and Cons

### NetworkManager Plugin - PROS ✅

1. **Higher-Level Abstraction**
   - Manages WiFi, VPNs, mobile broadband automatically
   - Handles connection profiles (save/restore)
   - Auto-reconnection logic built-in
   - Roaming support (WiFi handoff)

2. **Desktop/Laptop Use Cases**
   - Perfect for workstations that move between networks
   - WiFi management with WPA/WPA2/WPA3
   - VPN integration (OpenVPN, WireGuard, etc.)
   - Mobile broadband (3G/4G/5G)

3. **User Experience**
   - Integration with desktop environments (GNOME, KDE)
   - GUI tools already exist (nmcli, nmtui, nm-applet)
   - User-friendly connection management

4. **Built-in Features**
   - Connection priorities
   - Automatic fallback (WiFi → Ethernet → Mobile)
   - Hotspot/AP mode
   - Connection sharing
   - DNS management
   - Hostname management

5. **Native Checkpoint Support**
   - NetworkManager has built-in `CheckpointCreate` / `CheckpointRollback`
   - Can rollback network changes atomically
   - Perfect for op-dbus rollback feature

6. **Complex Scenarios Made Easy**
   - Bond interfaces (LACP)
   - VLANs
   - Bridges with STP
   - Team interfaces
   - IPv6 auto-configuration

7. **Secrets Management**
   - Integrates with system keyring
   - WiFi passwords stored securely
   - VPN credentials

### NetworkManager Plugin - CONS ❌

1. **Additional Layer of Abstraction**
   - op-dbus → NetworkManager → Backend → Kernel
   - More potential points of failure
   - Debugging harder (which layer is broken?)

2. **Not Universal**
   - Many servers don't run NetworkManager
   - Proxmox doesn't use NetworkManager
   - Minimal/embedded systems avoid it
   - Cloud instances often use systemd-networkd or direct config

3. **Performance Overhead**
   - Extra D-Bus round-trips
   - NetworkManager adds latency
   - May restart connections unnecessarily
   - Background scanning (WiFi)

4. **Complexity**
   - NetworkManager is a large daemon
   - More memory footprint
   - More attack surface
   - More dependencies

5. **Server Use Case Mismatch**
   - Servers need static, predictable networking
   - Don't need WiFi, VPN auto-connect, roaming
   - NetworkManager can interfere with manual config
   - "Helpful" auto-configuration can break things

6. **OVS Integration Limited**
   - NetworkManager can manage OVS, but it's not the primary use case
   - Direct OVSDB is more powerful for complex OVS scenarios
   - Some OVS features not exposed via NM

7. **State Conflicts**
   - NetworkManager vs manual configuration conflicts
   - Can fight with other network managers
   - Need to mark interfaces as "unmanaged"

### Current OVS/Netlink Approach - PROS ✅

1. **Direct Control**
   - No abstraction layer
   - Exactly what you configure is what happens
   - Predictable, deterministic

2. **Performance**
   - Minimal overhead
   - Native protocols (JSON-RPC, Netlink)
   - No background daemons needed
   - 10x faster than CLI wrappers

3. **Server-Optimized**
   - Perfect for static infrastructure
   - No unnecessary features
   - Minimal dependencies
   - Small attack surface

4. **Universal**
   - Works anywhere (Proxmox, Ubuntu, Debian, etc.)
   - No daemon required (besides ovs-vswitchd for OVS)
   - Container-friendly
   - Cloud-compatible

5. **Precision**
   - Full access to kernel networking
   - All netlink features available
   - All OVSDB features available
   - No limitations

6. **Lightweight**
   - Minimal memory footprint
   - No background processes
   - Fast startup
   - Suitable for embedded

### Current OVS/Netlink Approach - CONS ❌

1. **Lower-Level**
   - Need to manage more details manually
   - No auto-reconnection logic
   - No connection profiles
   - No roaming support

2. **Limited Scope**
   - No WiFi support (need wpa_supplicant integration)
   - No VPN management
   - No mobile broadband
   - Desktop features require separate implementation

3. **No Built-in Rollback**
   - Must implement our own checkpoint logic
   - NetworkManager has this natively

4. **More Code**
   - Need to handle edge cases ourselves
   - No existing profile management
   - More testing required

## Recommendation: Hybrid Approach

**Best solution: Support BOTH as separate plugins!**

### Use NetworkManager Plugin When:
✅ Desktop/laptop deployments
✅ WiFi management needed
✅ VPN integration required
✅ User-facing systems
✅ Systems already running NetworkManager
✅ Need connection profiles
✅ Mobile devices

**Example state file:**
```json
{
  "plugins": {
    "networkmanager": {
      "connections": [
        {"id": "wifi-home", "type": "wifi", "ssid": "HomeNet"},
        {"id": "wifi-office", "type": "wifi", "ssid": "OfficeNet"},
        {"id": "vpn-work", "type": "vpn", "gateway": "vpn.company.com"}
      ]
    }
  }
}
```

### Use OVS/Netlink Plugin When:
✅ Server deployments
✅ Proxmox hosts
✅ Static infrastructure
✅ Container hosts
✅ Cloud instances
✅ Minimal systems
✅ Performance-critical
✅ Complex OVS scenarios

**Example state file:**
```json
{
  "plugins": {
    "net": {
      "interfaces": [
        {"name": "vmbr0", "type": "ovs-bridge", "ports": ["eth0"]},
        {"name": "mesh", "type": "ovs-bridge", "ports": ["nm-network"]}
      ]
    }
  }
}
```

### Detection Logic

```rust
// In src/main.rs or plugin loader

async fn load_network_plugin() -> Box<dyn StatePlugin> {
    // Check if NetworkManager is running
    if is_networkmanager_running().await {
        log::info!("Detected NetworkManager, using NM plugin");
        Box::new(NetworkManagerPlugin::new().await.unwrap())
    } else {
        log::info!("Using direct OVS/Netlink plugin");
        Box::new(NetPlugin::new())
    }
}

async fn is_networkmanager_running() -> bool {
    match Connection::system().await {
        Ok(conn) => {
            zbus::Proxy::new(
                &conn,
                "org.freedesktop.NetworkManager",
                "/org/freedesktop/NetworkManager",
                "org.freedesktop.NetworkManager",
            )
            .await
            .is_ok()
        }
        Err(_) => false,
    }
}
```

## Implementation Priority

**Phase 1: Keep current OVS/Netlink** (✅ Done)
- Server use case is primary
- Works on Proxmox
- Universal compatibility

**Phase 2: Add NetworkManager plugin** (Future)
- Optional feature flag: `--features networkmanager`
- Auto-detection at runtime
- Documented for desktop users

**Phase 3: Plugin selection** (Future)
- CLI flag: `op-dbus --network-plugin=nm` or `--network-plugin=netlink`
- Config file option
- Auto-detection by default

## Conclusion

**op-dbus is server-only. We will NOT implement NetworkManager plugin.**

**Why OVS/Netlink is the correct choice:**
- ✅ Direct control for infrastructure
- ✅ 10x better performance
- ✅ Universal compatibility (works on all server distros)
- ✅ No extra dependencies
- ✅ Perfect for Proxmox/LXC use case
- ✅ Full access to kernel networking
- ✅ Lightweight and deterministic

**Why NetworkManager is wrong for servers:**
- ❌ Unnecessary abstraction
- ❌ Performance overhead
- ❌ Most servers don't run NetworkManager
- ❌ Adds complexity for no benefit
- ❌ Desktop-focused features we don't need
- ❌ Can interfere with static server configs

**This document exists only as reference** for understanding the technical decision and for potential desktop forks in the future.

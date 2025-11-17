# Common D-Bus Interfaces Reference

Public domain D-Bus interface specifications from freedesktop.org and common system services.

## Standard Interfaces (org.freedesktop.DBus.*)

### org.freedesktop.DBus.Introspectable
**Purpose**: Introspection support for D-Bus objects

**Methods**:
- `Introspect() → (xml: s)` - Returns XML description of object

**Example**:
```bash
busctl introspect org.freedesktop.systemd1 /org/freedesktop/systemd1
```

### org.freedesktop.DBus.Properties
**Purpose**: Generic property access

**Methods**:
- `Get(interface: s, property: s) → (value: v)`
- `Set(interface: s, property: s, value: v)`
- `GetAll(interface: s) → (properties: a{sv})`

**Signals**:
- `PropertiesChanged(interface: s, changed: a{sv}, invalidated: as)`

### org.freedesktop.DBus.ObjectManager
**Purpose**: Bulk object and interface discovery

**Methods**:
- `GetManagedObjects() → (objects: a{oa{sa{sv}}})`
  - Returns: object_path → interface → property → value

**Signals**:
- `InterfacesAdded(object: o, interfaces: a{sa{sv}})`
- `InterfacesRemoved(object: o, interfaces: as)`

**When to use**: Services with many objects (systemd, NetworkManager, BlueZ, UDisks2)

### org.freedesktop.DBus.Peer
**Purpose**: Peer-to-peer connection management

**Methods**:
- `Ping()` - Test if peer is alive
- `GetMachineId() → (machine_id: s)`

## systemd Interfaces (org.freedesktop.systemd1.*)

### org.freedesktop.systemd1.Manager
**Service**: org.freedesktop.systemd1
**Object**: /org/freedesktop/systemd1

**Key Methods**:
```
StartUnit(name: s, mode: s) → (job: o)
StopUnit(name: s, mode: s) → (job: o)
RestartUnit(name: s, mode: s) → (job: o)
ReloadUnit(name: s, mode: s) → (job: o)
EnableUnitFiles(files: as, runtime: b, force: b) → (changes: a(sss))
DisableUnitFiles(files: as, runtime: b) → (changes: a(sss))
ListUnits() → (units: a(ssssssouso))
ListUnitFiles() → (unit_files: a(ss))
GetUnit(name: s) → (unit: o)
```

**Properties**:
- `Version: s` - systemd version
- `Features: s` - Compiled features
- `Virtualization: s` - VM/container detection

### org.freedesktop.systemd1.Unit
**Object**: /org/freedesktop/systemd1/unit/*

**Properties**:
- `ActiveState: s` - "active", "inactive", "failed", etc.
- `LoadState: s` - "loaded", "not-found", "masked", etc.
- `SubState: s` - Unit-specific substate
- `Description: s` - Unit description
- `MainPID: u` - Main process PID (services)

**Methods**:
- `Start(mode: s) → (job: o)`
- `Stop(mode: s) → (job: o)`
- `Restart(mode: s) → (job: o)`

## NetworkManager Interfaces (org.freedesktop.NetworkManager.*)

### org.freedesktop.NetworkManager
**Service**: org.freedesktop.NetworkManager
**Object**: /org/freedesktop/NetworkManager

**Methods**:
```
GetDevices() → (devices: ao)
GetAllDevices() → (devices: ao)
ActivateConnection(connection: o, device: o, specific_object: o) → (active: o)
DeactivateConnection(active: o)
AddAndActivateConnection(connection: a{sa{sv}}, device: o, specific_object: o) → (path: o, active: o)
```

**Properties**:
- `State: u` - Overall NetworkManager state
- `Connectivity: u` - Internet connectivity state
- `PrimaryConnection: o` - Active primary connection
- `ActiveConnections: ao` - All active connections

### org.freedesktop.NetworkManager.Device
**Object**: /org/freedesktop/NetworkManager/Devices/*

**Properties**:
- `Interface: s` - Interface name (eth0, wlan0, etc.)
- `DeviceType: u` - Device type enum
- `State: u` - Device state
- `Ip4Address: u` - IPv4 address
- `Ip4Config: o` - IPv4 configuration object
- `Ip6Config: o` - IPv6 configuration object

### org.freedesktop.NetworkManager.Settings.Connection
**Object**: /org/freedesktop/NetworkManager/Settings/*

**Methods**:
```
Update(properties: a{sa{sv}})
Delete()
GetSettings() → (settings: a{sa{sv}})
```

## login1 (logind) Interfaces

### org.freedesktop.login1.Manager
**Service**: org.freedesktop.login1
**Object**: /org/freedesktop/login1

**Methods**:
```
ListSessions() → (sessions: a(susso))
ListUsers() → (users: a(uso))
ListSeats() → (seats: a(so))
PowerOff(interactive: b)
Reboot(interactive: b)
Suspend(interactive: b)
Hibernate(interactive: b)
```

### org.freedesktop.login1.Session
**Object**: /org/freedesktop/login1/session/*

**Properties**:
- `Id: s` - Session ID
- `User: (uo)` - User UID and object path
- `Name: s` - Username
- `Remote: b` - Remote session
- `State: s` - Session state
- `Type: s` - Session type (x11, wayland, tty, etc.)

## UDisks2 Interfaces

### org.freedesktop.UDisks2.Drive
**Service**: org.freedesktop.UDisks2
**Object**: /org/freedesktop/UDisks2/drives/*

**Properties**:
- `Vendor: s` - Drive vendor
- `Model: s` - Drive model
- `Serial: s` - Serial number
- `Size: t` - Size in bytes
- `Removable: b` - Removable media
- `MediaAvailable: b` - Media inserted

**Methods**:
```
Eject(options: a{sv})
PowerOff(options: a{sv})
```

### org.freedesktop.UDisks2.Block
**Object**: /org/freedesktop/UDisks2/block_devices/*

**Properties**:
- `Device: ay` - Device file path
- `Size: t` - Block device size
- `Drive: o` - Parent drive object
- `IdUUID: s` - Filesystem UUID
- `IdType: s` - Filesystem type

**Methods**:
```
Format(type: s, options: a{sv})
Rescan(options: a{sv})
```

## BlueZ Interfaces (org.bluez.*)

### org.bluez.Adapter1
**Service**: org.bluez
**Object**: /org/bluez/hci*

**Methods**:
```
StartDiscovery()
StopDiscovery()
RemoveDevice(device: o)
```

**Properties**:
- `Address: s` - Adapter MAC address
- `Name: s` - Adapter name
- `Powered: b` - Powered state
- `Discovering: b` - Discovery active

### org.bluez.Device1
**Object**: /org/bluez/hci*/dev_*

**Methods**:
```
Connect()
Disconnect()
Pair()
CancelPairing()
```

**Properties**:
- `Address: s` - Device MAC address
- `Name: s` - Device name
- `Paired: b` - Paired state
- `Connected: b` - Connected state
- `Trusted: b` - Trusted state

## PackageKit Interfaces

### org.freedesktop.PackageKit
**Service**: org.freedesktop.PackageKit
**Object**: /org/freedesktop/PackageKit

**Methods**:
```
CreateTransaction() → (transaction: o)
GetActions() → (actions: s)
GetGroups() → (groups: s)
```

### org.freedesktop.PackageKit.Transaction
**Object**: /org/freedesktop/PackageKit/Transaction/*

**Methods**:
```
SearchNames(filter: s, values: as)
InstallPackages(transaction_flags: t, package_ids: as)
RemovePackages(transaction_flags: t, package_ids: as, allow_deps: b, autoremove: b)
UpdatePackages(transaction_flags: t, package_ids: as)
RefreshCache(force: b)
```

**Signals**:
- `Package(info: s, package_id: s, summary: s)`
- `Finished(exit: s, runtime: u)`
- `ErrorCode(code: s, details: s)`

## Notification Daemon

### org.freedesktop.Notifications
**Service**: org.freedesktop.Notifications
**Object**: /org/freedesktop/Notifications

**Methods**:
```
Notify(app_name: s, replaces_id: u, icon: s, summary: s, body: s, actions: as, hints: a{sv}, timeout: i) → (id: u)
CloseNotification(id: u)
GetCapabilities() → (capabilities: as)
GetServerInformation() → (name: s, vendor: s, version: s, spec_version: s)
```

**Signals**:
- `NotificationClosed(id: u, reason: u)`
- `ActionInvoked(id: u, action_key: s)`

## Common Patterns

### Asynchronous Operations (Jobs)
Many D-Bus services use job objects for long-running operations:

```
1. Call method (e.g., systemd StartUnit)
2. Receive job object path
3. Monitor job properties/signals
4. Job completes with result
```

### Property Change Notifications
Subscribe to property changes:

```
busctl monitor org.freedesktop.NetworkManager \
    --match "type='signal',interface='org.freedesktop.DBus.Properties',member='PropertiesChanged'"
```

### Object Discovery Hierarchy
```
1. Try ObjectManager.GetManagedObjects (fastest)
2. If not available, introspect root object
3. Recursively introspect child nodes
4. Build complete object tree
```

## Error Handling

### Common D-Bus Errors
- `org.freedesktop.DBus.Error.ServiceUnknown` - Service not running
- `org.freedesktop.DBus.Error.UnknownMethod` - Method doesn't exist
- `org.freedesktop.DBus.Error.UnknownObject` - Object path not found
- `org.freedesktop.DBus.Error.AccessDenied` - Permission denied
- `org.freedesktop.DBus.Error.Failed` - Generic failure

### Service-Specific Errors
Each service defines its own error domain:
- `org.freedesktop.systemd1.NoSuchUnit`
- `org.freedesktop.NetworkManager.InvalidConnection`
- `org.bluez.Error.NotReady`

## References

- D-Bus Specification: https://dbus.freedesktop.org/doc/dbus-specification.html
- systemd D-Bus API: https://www.freedesktop.org/wiki/Software/systemd/dbus/
- NetworkManager D-Bus API: https://networkmanager.dev/docs/api/latest/
- BlueZ D-Bus API: https://github.com/bluez/bluez/tree/master/doc
- UDisks2 D-Bus API: http://storaged.org/doc/udisks2-api/latest/

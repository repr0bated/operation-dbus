# Install Script Requirements - Complete Infrastructure Setup

## Context
operation-dbus is a declarative infrastructure management system that uses:
- D-Bus introspection for system discovery
- BTRFS copy-on-write for instant container provisioning
- Blockchain timing database for audit trail
- ML vectorization for search
- Dual networking: socket (local) + Netmaker mesh (distributed)

## User Workflow This Enables

1. **Admin runs install script once** → All infrastructure in place
2. **Admin creates new container** → Choice presented:
   - Socket networking (local Unix sockets, fast, simple)
   - Netmaker mesh (WireGuard encrypted, distributed across hosts)
3. **Container created instantly** → Cloned from BTRFS template subvolume (CoW)
4. **Netmaker option auto-enrolls** → Join key embedded in template, automatic mesh join
5. **Plugins auto-discover** → D-Bus introspection finds available services
6. **All changes audited** → Blockchain timing DB + ML vectors for search

## Infrastructure Components Required

### 1. BTRFS Subvolume Structure

```
/var/lib/op-dbus/
├── @cache/                    # ML embeddings cache (zstd:3 compression)
│   ├── embeddings/            # 384-dim vectors, ~60-70% compression ratio
│   ├── queries/               # Query result cache
│   └── blocks/                # Block cache
│
├── @timing/                   # Blockchain timing database (zstd:3)
│   └── blockchain.db          # SQLite audit trail
│
├── @vectors/                  # Vector search index (zstd:3)
│   └── vector-index.db        # FAISS/Qdrant index
│
├── @state/                    # Current infrastructure state (zstd:1)
│   └── current-state.json     # Live system state
│
├── @snapshots/                # Plugin snapshots (zstd:1)
│   ├── plugin-lxc-20250108/   # Timestamped plugin snapshots
│   └── plugin-netmaker-*/     # For distribution
│
├── @plugins/                  # Active plugins (zstd:1)
│   ├── lxc/                   # LXC plugin state
│   ├── netmaker/              # Netmaker plugin state
│   └── [plugin-name]/         # Per-plugin directories
│
└── @templates/                # Container templates (zstd:1)
    ├── debian-base/           # Base Debian template subvolume
    │   ├── rootfs/            # Root filesystem
    │   ├── netmaker-join-key  # Embedded join key
    │   └── config/            # Default container config
    └── [distro-name]/         # Other distro templates
```

**Why each subvolume:**
- `@cache` - Persistent ML embeddings survive reboots, huge performance gain
- `@timing` - Immutable audit trail, cannot be tampered
- `@vectors` - Fast semantic search across all operations
- `@state` - Current desired state, declarative source of truth
- `@snapshots` - Plugin versioning, rollback capability
- `@plugins` - Per-plugin isolated state
- `@templates` - CoW source for instant container creation

### 2. OVS Bridge Configuration

```bash
# Main bridge for all container traffic
ovsbr0
├── Port: eth0 (uplink to physical network)
├── Port: veth-socket (socket networking namespace)
└── Port: veth-mesh (Netmaker mesh interface)
```

**OpenFlow Rules Required:**

```
Table 0: Classification
- Priority 100: Match dst=socket → goto Table 10 (Socket Path)
- Priority 100: Match dst=mesh → goto Table 20 (Mesh Path)
- Priority 50: Default → goto Table 30 (Normal Bridge)

Table 10: Socket Networking
- Action: Output to Unix socket path
- Used for: Same-host container-to-container (fast)

Table 20: Netmaker Mesh
- Action: Encap WireGuard, output to mesh interface
- Used for: Cross-host container-to-container (distributed)

Table 30: Normal Bridge
- Action: Normal L2 switching
- Used for: External network access
```

**Why this matters:**
- Socket networking = microsecond latency for local containers
- Netmaker mesh = encrypted distributed networking across datacenters
- Same OVS bridge handles both, container chooses at creation time

### 3. Netmaker Infrastructure

**Install script must:**
1. Install netmaker client (`netclient`)
2. Store join key in `/var/lib/op-dbus/@templates/*/netmaker-join-key`
3. Create systemd service that enrolls containers on first boot
4. Configure WireGuard kernel module

**Template Integration:**
```bash
# In each template subvolume:
/var/lib/op-dbus/@templates/debian-base/
├── rootfs/
│   ├── usr/bin/netclient                    # Pre-installed
│   ├── etc/systemd/system/netmaker-enroll.service  # Auto-enroll on boot
│   └── etc/netmaker/
│       └── join-key                         # Embedded enrollment key
```

**User experience:**
1. Admin creates container: `lxc-create -t debian-base -n web-01 --netmaker`
2. Container boots → systemd runs `netmaker-enroll.service`
3. Service reads `/etc/netmaker/join-key`
4. Calls `netclient join -t <key>`
5. Container now in mesh, can reach containers on other hosts

### 4. Container Template System (BTRFS Snapshots)

**How it works:**
```bash
# Install script creates base template
btrfs subvolume create /var/lib/op-dbus/@templates/debian-base
# Populate with minimal Debian rootfs
debootstrap stable /var/lib/op-dbus/@templates/debian-base/rootfs
# Install netclient, configure defaults
chroot /var/lib/op-dbus/@templates/debian-base/rootfs apt install netclient
# Embed join key, socket paths, etc.

# User creates container (instant CoW snapshot)
btrfs subvolume snapshot /var/lib/op-dbus/@templates/debian-base /var/lib/lxc/web-01/rootfs
# Container creation takes ~50ms instead of 5 minutes
```

**Template defaults to embed:**
- `/etc/netmaker/join-key` - Netmaker enrollment
- `/etc/systemd/system/netmaker-enroll.service` - Auto-enrollment
- `/etc/systemd/system/socket-network.service` - Socket networking setup
- `/etc/opdbus/` - operation-dbus client config
- Default user, SSH keys, timezone, locale

**Proxmox integration:**
```bash
# Proxmox uses /var/lib/vz/template/cache/ for templates
# Install script creates symlinks:
ln -s /var/lib/op-dbus/@templates/debian-base \
      /var/lib/vz/template/cache/debian-opdbus.tar.gz

# In Proxmox UI, template appears as "debian-opdbus"
# User clicks "Create CT" → Selects template → Instant deployment
```

### 5. Proxmox LXC Defaults

**Install script must configure:**
```bash
# /etc/pve/lxc/[VMID].conf defaults
lxc.apparmor.profile: unconfined
lxc.cgroup.devices.allow: c 10:200 rwm  # TUN device for WireGuard
lxc.mount.entry: /var/lib/op-dbus/@plugins bind,create=dir 0 0
lxc.hook.start: /usr/local/bin/opdbus-container-start
```

**Default resource allocation:**
```
cores: 2
memory: 2048
swap: 512
rootfs: 8
net0: name=eth0,bridge=ovsbr0,firewall=1,gw=10.0.0.1,ip=dhcp
```

**Container creation hook:**
```bash
#!/bin/bash
# /usr/local/bin/opdbus-container-start
# Runs when container starts

VMID=$1
CONTAINER_NAME=$(pct config $VMID | grep hostname | cut -d: -f2)

# Check if Netmaker or socket networking
if grep -q "netmaker" /var/lib/lxc/$CONTAINER_NAME/config; then
    # Netmaker path: trigger enrollment
    pct exec $VMID -- systemctl start netmaker-enroll
else
    # Socket path: configure Unix socket
    pct exec $VMID -- systemctl start socket-network
fi

# Notify operation-dbus of new container
/usr/local/bin/op-dbus container-created $VMID $CONTAINER_NAME
```

### 6. NixOS Integration Paths

**Install script must support both traditional and NixOS deployments:**

**Option A: Traditional install (Debian/Proxmox/Other Linux)**
```bash
# Install operation-dbus binary
cargo build --release
cp target/release/op-dbus /usr/local/bin/
# Create subvolumes, OVS bridges, templates (imperative)
# Configure Proxmox defaults
```

**Option B: NixOS declarative install**
```nix
# /etc/nixos/configuration.nix
{
  imports = [ ./operation-dbus/nixos/modules/operation-dbus.nix ];

  services.operation-dbus = {
    enable = true;

    btrfs = {
      enable = true;
      basePath = "/var/lib/op-dbus";
      compressionLevel = 3;
      subvolumes = [ "@cache" "@timing" "@vectors" "@state" "@snapshots" "@plugins" "@templates" ];
    };

    numa = {
      enable = true;  # For multi-socket systems
      node = 0;
      cpuList = "0-7";
    };

    ml = {
      enable = true;
      executionProvider = "cuda";  # or "cpu"
      numThreads = 8;
    };

    netmaker = {
      enable = true;
      joinKey = "your-netmaker-join-key";
    };

    mcp = {
      enable = true;
      port = 8080;
    };

    defaultState = {
      version = "1.0";
      plugins = {};
    };
  };
}
```

**NixOS Module Files (already in repo):**
- `nixos/modules/operation-dbus.nix` - Main module with all options
- `nixos/examples/proxmox-host.nix` - Full production example
- `nixos/examples/workstation.nix` - Minimal example
- `nixos/netboot/configs/*.nix` - Netboot installer configs

**Install script NixOS detection:**

```bash
#!/bin/bash
# install-opdbus.sh

if [ -f /etc/NIXOS ]; then
    echo "═══════════════════════════════════════════════════════"
    echo "NixOS detected!"
    echo "═══════════════════════════════════════════════════════"
    echo ""
    echo "For NixOS, use the declarative configuration instead of this script."
    echo ""
    echo "Quick start:"
    echo "1. Clone this repo to /etc/nixos/operation-dbus:"
    echo "   cd /etc/nixos"
    echo "   git clone https://github.com/repr0bated/operation-dbus.git"
    echo ""
    echo "2. Import the module in /etc/nixos/configuration.nix:"
    echo "   imports = [ ./operation-dbus/nixos/modules/operation-dbus.nix ];"
    echo ""
    echo "3. Enable and configure:"
    echo "   services.operation-dbus.enable = true;"
    echo "   services.operation-dbus.btrfs.enable = true;"
    echo ""
    echo "4. Apply configuration:"
    echo "   sudo nixos-rebuild switch"
    echo ""
    echo "See nixos/examples/ for complete configuration examples."
    echo ""
    echo "Exiting (no changes made)."
    exit 0
fi

# Traditional Linux path continues...
echo "Traditional Linux detected, proceeding with installation..."
```

**What NixOS module handles automatically:**
- BTRFS subvolume creation (via systemd.tmpfiles)
- OVS bridge configuration (via systemd.services.openvswitch)
- Netmaker installation and enrollment (via services.netmaker)
- MCP chat server systemd service
- operation-dbus systemd service with CPUAffinity, environment vars
- Firewall rules (networking.firewall.allowedTCPPorts)
- All in `/etc/nixos/configuration.nix` - version controlled, declarative

**Hybrid approach (for testing NixOS module on traditional system):**
```bash
# Install script can optionally generate NixOS-compatible config
./install-opdbus.sh --generate-nix-config

# Creates: /tmp/operation-dbus-config.nix
# User can copy to NixOS system or compare declarative vs imperative
```

**Integration test:**
```bash
# Test NixOS module works
nix-build '<nixpkgs/nixos>' \
  -A config.system.build.toplevel \
  -I nixos-config=./nixos/examples/workstation.nix

# Test traditional install works
./install-opdbus.sh  # On Debian/Ubuntu/etc
```

**Deployment type marker:**
```bash
# Install script creates
/etc/opdbus/deployment-type
# Contains: "traditional" or "nixos"

# operation-dbus binary checks this to know:
# - Where to find configs (traditional: /etc/opdbus/, NixOS: /nix/store/...)
# - How to suggest updates (apt/dnf vs nixos-rebuild)
# - Audit trail metadata (deployment method)
```

**Why both paths matter:**
- **Traditional**: Most users on Debian/Proxmox, familiar imperative approach
- **NixOS**: Declarative, reproducible, version-controlled infrastructure-as-code
- **Same architecture**: Both create identical BTRFS/OVS/Netmaker infrastructure
- **Migration path**: Start traditional, migrate to NixOS later (or vice versa)

### 7. Plugin Subvolume Structure

**Per-plugin isolation:**
```
/var/lib/op-dbus/@plugins/
├── lxc/
│   ├── plugin.toml              # Plugin metadata
│   ├── semantic-mapping.toml    # Write operation mappings
│   ├── state/                   # Plugin-specific state
│   └── cache/                   # Plugin-specific cache
├── netmaker/
│   ├── plugin.toml
│   ├── mesh-topology.json       # Current mesh state
│   └── enrollment-keys/         # Join keys per network
└── ovs/
    ├── plugin.toml
    ├── flows.json               # Current OpenFlow rules
    └── bridges.json             # Bridge configurations
```

**Install script creates skeleton:**
```bash
for plugin in lxc netmaker ovs systemd; do
    btrfs subvolume create /var/lib/op-dbus/@plugins/$plugin
    mkdir -p /var/lib/op-dbus/@plugins/$plugin/{state,cache}
    # Create default plugin.toml
    cat > /var/lib/op-dbus/@plugins/$plugin/plugin.toml <<EOF
name = "$plugin"
version = "1.0.0"
introspection_path = "org.freedesktop.$plugin"
capabilities = ["read", "write"]
EOF
done
```

**Snapshot for distribution:**
```bash
# When plugin reaches stable state, create snapshot
btrfs subvolume snapshot \
    /var/lib/op-dbus/@plugins/lxc \
    /var/lib/op-dbus/@snapshots/plugin-lxc-$(date +%Y%m%d)

# Other users can receive snapshot
btrfs send /var/lib/op-dbus/@snapshots/plugin-lxc-20250108 | \
    ssh user@remote btrfs receive /var/lib/op-dbus/@plugins/
```

### 8. Blockchain Timing Database

**Structure:**
```sql
CREATE TABLE operations (
    id INTEGER PRIMARY KEY,
    timestamp INTEGER NOT NULL,
    operation TEXT NOT NULL,        -- e.g., "container_create"
    target TEXT NOT NULL,            -- e.g., "web-01"
    previous_hash TEXT,              -- SHA256 of previous operation
    current_hash TEXT NOT NULL,      -- SHA256(timestamp + operation + target + prev_hash)
    embedding BLOB,                  -- 384-dim ML vector
    metadata JSON                    -- Additional context
);

CREATE INDEX idx_timestamp ON operations(timestamp);
CREATE INDEX idx_operation ON operations(operation);
CREATE INDEX idx_target ON operations(target);
```

**Install script must:**
1. Create `/var/lib/op-dbus/@timing/blockchain.db`
2. Initialize schema
3. Insert genesis block:
```sql
INSERT INTO operations VALUES (
    0,
    strftime('%s', 'now'),
    'genesis',
    'system',
    NULL,
    '0000000000000000000000000000000000000000000000000000000000000000',
    NULL,
    '{"version": "1.0", "deployment": "operation-dbus"}'
);
```

**Why blockchain:**
- Immutable audit trail
- Cryptographic verification of operation sequence
- Cannot be tampered (hash chain)
- ML embeddings enable semantic search: "show me all container operations last week"

### 9. Socket Networking Infrastructure

**Unix socket paths:**
```
/var/lib/op-dbus/sockets/
├── containers/
│   ├── web-01.sock     # Container-to-container communication
│   ├── web-02.sock
│   └── db-01.sock
├── plugins/
│   ├── lxc.sock        # Plugin control sockets
│   ├── netmaker.sock
│   └── ovs.sock
└── api/
    └── opdbus.sock     # Main API socket
```

**Install script creates:**
```bash
mkdir -p /var/lib/op-dbus/sockets/{containers,plugins,api}
chmod 770 /var/lib/op-dbus/sockets
chown root:opdbus /var/lib/op-dbus/sockets

# systemd socket activation
cat > /etc/systemd/system/opdbus.socket <<EOF
[Unit]
Description=operation-dbus API Socket

[Socket]
ListenStream=/var/lib/op-dbus/sockets/api/opdbus.sock
SocketMode=0660
SocketUser=root
SocketGroup=opdbus

[Install]
WantedBy=sockets.target
EOF
```

**Container socket setup:**
```bash
# When container created, create its socket
container_name="web-01"
socket_path="/var/lib/op-dbus/sockets/containers/${container_name}.sock"

# OVS flow to route container traffic through socket
ovs-ofctl add-flow ovsbr0 \
    "table=10,priority=100,ip,nw_dst=${container_ip},\
     actions=output:${socket_path}"
```

**Performance benefit:**
- Socket: ~1μs latency, 40Gbps+ throughput
- Mesh (WireGuard): ~10ms latency, 1-10Gbps (encrypted)
- Choice depends on: same host (socket) or distributed (mesh)

### 10. MCP Web Server (Chat Interface)

**Purpose:**
Provides a conversational web UI for interacting with operation-dbus through natural language instead of raw CLI commands or JSON APIs.

**Architecture:**
```
Browser (port 8080)
    ↓ WebSocket
Chat Server (Rust/Axum)
    ↓ D-Bus
operation-dbus → Introspection → Tools/Plugins
```

**Install script must:**

1. **Build MCP chat server:**
```bash
cargo build --features mcp --bin mcp-chat --release
cp target/release/mcp-chat /usr/local/bin/
```

2. **Create systemd service:**
```bash
cat > /etc/systemd/system/mcp-chat.service <<EOF
[Unit]
Description=operation-dbus MCP Chat Server
After=network.target opdbus.service
Requires=opdbus.service

[Service]
Type=simple
User=root
WorkingDirectory=/var/lib/op-dbus
ExecStart=/usr/local/bin/mcp-chat
Restart=on-failure
Environment="MCP_CHAT_PORT=8080"
Environment="RUST_LOG=info"

[Install]
WantedBy=multi-user.target
EOF
```

3. **Install web frontend:**
```bash
mkdir -p /var/lib/op-dbus/web
cp src/mcp/web/* /var/lib/op-dbus/web/
chmod 644 /var/lib/op-dbus/web/*
```

4. **Configure firewall (if enabled):**
```bash
# UFW
ufw allow 8080/tcp comment "MCP Chat Interface"

# firewalld
firewall-cmd --permanent --add-port=8080/tcp
firewall-cmd --reload
```

5. **Enable and start service:**
```bash
systemctl daemon-reload
systemctl enable mcp-chat
systemctl start mcp-chat
```

**User Experience:**
```
1. Admin opens browser: http://server-ip:8080/chat.html
2. Chat interface loads with WebSocket connection
3. Admin types: "create container web-01 with socket networking"
4. NLP parser converts to: op-dbus container create web-01 --network socket
5. Command executes, real-time output streams to chat
6. History saved, searchable via ML embeddings
```

**Natural Language Examples:**
```
User: "show me all containers"
→ Executes: op-dbus query containers

User: "create debian container named db-01 with netmaker mesh"
→ Executes: op-dbus container create db-01 --template debian-base --network mesh

User: "what containers were created yesterday?"
→ ML search: blockchain DB + embeddings → results

User: "status of systemd services"
→ D-Bus introspection: org.freedesktop.systemd1 → service list
```

**WebSocket Protocol:**
- Client sends: Plain text natural language
- Server sends: JSON responses with type, content, timestamp, tools_used
- Bidirectional: Server can push status updates

**Why this matters:**
- **Accessibility**: Non-technical users can manage infrastructure via chat
- **Audit trail**: All chat commands logged to blockchain
- **Discovery**: "help" and auto-suggestions guide users to available tools
- **Real-time**: WebSocket enables live progress updates
- **Mobile-friendly**: Responsive UI works on any device

**Testing:**
```bash
# After install, verify chat server
curl http://localhost:8080/chat.html
# Should return HTML

# Check WebSocket endpoint
wscat -c ws://localhost:8080/ws
# Should connect successfully

# Test command via WebSocket
> list tools
< {"type":"assistant","content":"Available tools: systemd, file, network, process, lxc, netmaker..."}
```

**Integration with operation-dbus:**
- Chat server uses operation-dbus D-Bus API
- All commands execute through same tool registry
- Permissions respect same D-Bus policies
- Audit trail unified (both CLI and chat commands logged)

## Install Script Flow

```bash
#!/bin/bash
# install-opdbus.sh - Complete infrastructure setup

1. Detect environment
   - Check if NixOS → guide to declarative path
   - Check if Proxmox → enable LXC features
   - Check for BTRFS → abort if missing

2. Create BTRFS subvolumes
   - @cache, @timing, @vectors, @state, @snapshots, @plugins, @templates
   - Set compression levels (zstd:1 or zstd:3)
   - Set proper permissions

3. Setup OVS networking
   - Create ovsbr0 bridge
   - Add uplink port (eth0 or specified)
   - Install OpenFlow rules (3 tables)
   - Create socket networking namespace

4. Install Netmaker
   - Download netclient binary
   - Install to /usr/local/bin/netclient
   - Create systemd services for auto-enrollment
   - Configure WireGuard kernel module

5. Create container templates
   - Download base OS (Debian, Ubuntu, Alpine)
   - Create BTRFS subvolume in @templates/
   - Install netclient in template
   - Embed join key
   - Configure socket networking
   - Set defaults (user, SSH, timezone)

6. Configure Proxmox (if detected)
   - Set LXC defaults in /etc/pve/lxc/
   - Create container creation hooks
   - Symlink templates to /var/lib/vz/template/cache/
   - Configure AppArmor profiles

7. Initialize blockchain database
   - Create SQLite schema in @timing/blockchain.db
   - Insert genesis block
   - Set up write-ahead logging (WAL)

8. Setup socket infrastructure
   - Create /var/lib/op-dbus/sockets/ hierarchy
   - Install systemd socket units
   - Configure permissions

9. Initialize plugin system
   - Create @plugins/ subvolumes for: lxc, netmaker, ovs, systemd
   - Generate default plugin.toml files
   - Create semantic-mapping.toml skeletons

10. Build and install operation-dbus
    - cargo build --release
    - Install binary to /usr/local/bin/op-dbus
    - Create systemd service
    - Enable and start service

11. Build and install MCP chat server
    - cargo build --features mcp --bin mcp-chat --release
    - Install binary to /usr/local/bin/mcp-chat
    - Copy web frontend to /var/lib/op-dbus/web/
    - Create systemd service for chat server
    - Configure firewall (port 8080)
    - Enable and start service

12. Run system introspection
    - op-dbus query → discover current state
    - Save to @state/current-state.json
    - Create initial snapshots

13. Enable upgrade mode
    - Create /etc/opdbus/installed marker
    - Record installation timestamp
    - Future runs = upgrade/update mode
```

## Idempotent Behavior (Upgrade/Update Mode)

**Install script must:**
1. Check for `/etc/opdbus/installed`
2. If exists → upgrade mode
3. Never destroy existing data
4. Add missing components
5. Update configurations
6. Preserve user customizations

**Example upgrade logic:**
```bash
if [ -f /etc/opdbus/installed ]; then
    echo "Existing installation detected, upgrading..."

    # Check each component
    [ -d /var/lib/op-dbus/@cache ] || create_cache_subvolume
    [ -f /etc/systemd/system/opdbus.service ] || install_systemd_service

    # Update OVS flows (non-destructive)
    update_ovs_flows

    # Refresh templates (preserve user templates)
    refresh_system_templates

    # Upgrade binary
    cargo build --release
    systemctl stop opdbus
    cp target/release/op-dbus /usr/local/bin/
    systemctl start opdbus
else
    echo "Fresh installation..."
    full_install
    touch /etc/opdbus/installed
fi
```

## Testing Requirements

**Install script must pass:**
1. Fresh Debian 12 install → full setup works
2. Fresh Proxmox VE 8 → full setup + LXC integration
3. Re-run script → idempotent (no errors, updates applied)
4. Create container with socket → local communication works
5. Create container with Netmaker → mesh enrollment works
6. Check BTRFS compression → ratios match expected (~60-70%)
7. Check blockchain DB → genesis block exists
8. Check plugin discovery → all plugins found
9. NixOS detection → guides to declarative config (doesn't proceed)

## What the User Gets

After install script completes:

**Infrastructure:**
- ✅ BTRFS subvolumes for all components
- ✅ OVS bridge with socket + mesh routing
- ✅ Netmaker mesh ready for enrollment
- ✅ Container templates ready to clone
- ✅ Blockchain audit trail initialized
- ✅ Plugin system configured
- ✅ MCP chat server running on port 8080
- ✅ Web UI accessible via browser

**Capabilities:**
- ✅ Create containers in ~50ms (BTRFS snapshot)
- ✅ Choose socket (local) or mesh (distributed) networking
- ✅ Automatic Netmaker enrollment for mesh containers
- ✅ All operations logged to blockchain
- ✅ ML search across operation history
- ✅ Plugin auto-discovery via D-Bus
- ✅ Declarative state management
- ✅ Upgrade script anytime without data loss

**Commands unlocked:**
```bash
# Via CLI:
op-dbus container create web-01 --template debian-base --network socket
op-dbus container create db-01 --template debian-base --network mesh
op-dbus query
op-dbus search "containers created last week"
op-dbus snapshot create backup-$(date +%Y%m%d)

# Via Web Chat (http://server-ip:8080/chat.html):
"create container web-01 with socket networking"
"show me all containers"
"what containers were created yesterday?"
"status of systemd services"

# Upgrade infrastructure
./install-opdbus.sh  # Detects existing install, upgrades
```

## Key Design Decisions

1. **BTRFS snapshots for templates** → 100x faster than traditional provisioning
2. **Dual networking (socket + mesh)** → Local speed when possible, distributed when needed
3. **Blockchain for audit** → Immutable, cryptographically verifiable
4. **Per-plugin subvolumes** → Isolation, snapshotting, distribution
5. **Socket activation** → Only runs when needed, lower resource usage
6. **Idempotent install** → Can run repeatedly, upgrades safely
7. **NixOS detection** → Guides to declarative path instead of imperative hacks

---

This document is the complete specification for the new install script. Every component, every subvolume, every network path, every user workflow - all captured here for implementation.

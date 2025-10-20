# op-dbus CLI Design

Complete CLI command specification for op-dbus.

## Current Status

**What we have (4 commands):**
- `op-dbus run` - Run daemon
- `op-dbus query [--plugin NAME]` - Query current state
- `op-dbus diff STATE_FILE` - Show diff between current and desired
- `op-dbus apply STATE_FILE` - Apply desired state

**What's missing:**
- Blockchain commands (list, verify, export, search)
- Rollback functionality
- Container lifecycle commands
- Network debugging helpers
- Plugin management
- Checkpoint creation

## Architecture Clarity

**op-dbus is the ONLY binary.** There is no:
- ❌ ovs-port-agent (old name, removed)
- ❌ Go code (pure Rust)
- ❌ gopyobject (not used)
- ❌ Separate agents

**Everything is op-dbus:**
```
op-dbus               # Main binary
├─ Native Rust        # No Go dependencies
├─ Native protocols   # OVSDB, Netlink, D-Bus
└─ Single CLI         # All commands in one binary
```

## Complete CLI Command Set

### Core State Management

```bash
# Query current system state
op-dbus query                      # All plugins
op-dbus query --plugin net         # Specific plugin
op-dbus query --plugin lxc
op-dbus query --plugin systemd
op-dbus query --format json        # JSON output (default)
op-dbus query --format yaml        # YAML output
op-dbus query --output state.json  # Save to file

# Show diff between current and desired
op-dbus diff state.json                    # Show all changes
op-dbus diff state.json --plugin net       # Show network changes only
op-dbus diff state.json --format text      # Human-readable
op-dbus diff state.json --format json      # Machine-readable (default)

# Apply desired state
op-dbus apply state.json                   # Apply all changes
op-dbus apply state.json --plugin net      # Apply network only
op-dbus apply state.json --dry-run         # Show what would happen
op-dbus apply state.json --checkpoint      # Create checkpoint before apply
op-dbus apply state.json --no-blockchain   # Skip blockchain footprint

# Verify current state matches footprint
op-dbus verify                             # Verify against last footprint
op-dbus verify --footprint HASH            # Verify against specific hash
op-dbus verify --full                      # Full blockchain integrity check
```

### Blockchain & Audit Trail

```bash
# List blockchain blocks
op-dbus blockchain list                    # Show all blocks
op-dbus blockchain list --limit 10         # Last 10 blocks
op-dbus blockchain list --since 2025-01-01 # Since date
op-dbus blockchain list --plugin net       # Network changes only

# Show specific block
op-dbus blockchain show BLOCK_ID           # Show block details
op-dbus blockchain show latest             # Show latest block
op-dbus blockchain show 0                  # Show genesis block

# Search blockchain
op-dbus blockchain search "nginx"          # Search for changes
op-dbus blockchain search "vmbr0"          # Search for interface
op-dbus blockchain search --semantic "database config"  # ML search (if enabled)

# Export blockchain
op-dbus blockchain export                  # Export all blocks (JSON)
op-dbus blockchain export --range 0-100    # Export block range
op-dbus blockchain export --since 2025-01-01
op-dbus blockchain export --output audit-2025-Q1.json

# Verify blockchain integrity
op-dbus blockchain verify                  # Verify all blocks
op-dbus blockchain verify --from 10        # Verify from block 10
op-dbus blockchain verify --full           # Full cryptographic check

# Compact/archive blockchain
op-dbus blockchain compact                 # Remove old full states (keep hashes)
op-dbus blockchain archive --before 2024-01-01  # Archive old blocks
```

### Rollback & Checkpoints

```bash
# Create checkpoint
op-dbus checkpoint create                  # Create checkpoint now
op-dbus checkpoint create --name "before-upgrade"
op-dbus checkpoint list                    # List all checkpoints
op-dbus checkpoint show CHECKPOINT_ID      # Show checkpoint details

# Rollback
op-dbus rollback                           # Rollback to previous state
op-dbus rollback --to-block 10             # Rollback to specific block
op-dbus rollback --to-checkpoint NAME      # Rollback to checkpoint
op-dbus rollback --dry-run                 # Show what would be rolled back
op-dbus rollback --plugin net              # Rollback network only
```

### Container Management (LXC)

```bash
# List containers
op-dbus container list                     # All containers
op-dbus container list --running           # Running only
op-dbus container list --stopped           # Stopped only
op-dbus container list --netmaker          # Netmaker-enabled only

# Show container details
op-dbus container show 100                 # Show container 100
op-dbus container show 100 --network       # Network config only

# Container lifecycle
op-dbus container create 100 --network-type netmaker  # Create container
op-dbus container start 100                           # Start container
op-dbus container stop 100                            # Stop container
op-dbus container destroy 100                         # Delete container
op-dbus container restart 100                         # Restart container

# Container networking
op-dbus container attach 100 --bridge mesh            # Attach to bridge
op-dbus container detach 100 --interface vi100        # Detach interface
op-dbus container netmaker-join 100                   # Join netmaker
op-dbus container netmaker-leave 100                  # Leave netmaker

# Batch operations
op-dbus container start-all                           # Start all containers
op-dbus container stop-all                            # Stop all containers
```

### Network Debugging & Helpers

```bash
# Show network state
op-dbus net bridges                        # List all bridges
op-dbus net bridge show vmbr0              # Show bridge details
op-dbus net bridge ports vmbr0             # List bridge ports
op-dbus net interfaces                     # List all interfaces
op-dbus net interface show eth0            # Show interface details
op-dbus net routes                         # Show routing table

# Test connectivity
op-dbus net ping TARGET                    # Ping test
op-dbus net trace TARGET                   # Traceroute
op-dbus net resolve HOSTNAME               # DNS resolution test

# OVS-specific
op-dbus ovs status                         # OVS daemon status
op-dbus ovs bridges                        # List OVS bridges
op-dbus ovs ports BRIDGE                   # List ports on bridge
op-dbus ovs flows BRIDGE                   # Show OpenFlow flows
op-dbus ovs dump                           # Dump full OVS config
```

### Daemon & Service Management

```bash
# Run daemon
op-dbus run                                # Run with /etc/op-dbus/state.json
op-dbus run --state-file custom.json       # Run with custom state
op-dbus run --enable-dhcp-server           # Run with DHCP server
op-dbus run --oneshot                      # Apply and exit (no daemon)
op-dbus run --watch                        # Watch for state file changes

# Daemon status
op-dbus status                             # Show daemon status
op-dbus status --verbose                   # Detailed status
op-dbus health                             # Health check (for monitoring)
```

### Plugin Management

```bash
# List plugins
op-dbus plugin list                        # All registered plugins
op-dbus plugin show net                    # Show plugin details
op-dbus plugin capabilities net            # Show plugin capabilities

# Plugin state
op-dbus plugin query net                   # Query plugin state (alias for `query --plugin`)
op-dbus plugin apply net state.json        # Apply to specific plugin
op-dbus plugin verify net                  # Verify plugin state

# Plugin diagnostics
op-dbus plugin test net                    # Test plugin connectivity
op-dbus plugin debug net                   # Debug plugin (verbose logging)
```

### Configuration & Setup

```bash
# Generate example config
op-dbus init                               # Generate /etc/op-dbus/state.json
op-dbus init --introspect                  # Auto-detect current state
op-dbus init --template minimal            # Minimal config template
op-dbus init --template full               # Full config with examples
op-dbus init --output custom.json          # Custom output path

# Validate config
op-dbus validate state.json                # Validate state file syntax
op-dbus validate state.json --strict       # Strict validation
op-dbus validate state.json --plugin net   # Validate plugin section only

# Show current config
op-dbus config show                        # Show current config
op-dbus config paths                       # Show config file paths
```

### System Information

```bash
# Version and info
op-dbus version                            # Show version
op-dbus version --verbose                  # Version + build info
op-dbus info                               # System info

# Diagnostics
op-dbus doctor                             # Check system prerequisites
op-dbus doctor --fix                       # Attempt to fix issues
op-dbus logs                               # Show recent logs
op-dbus logs --follow                      # Follow logs
op-dbus logs --since 1h                    # Logs since 1 hour ago
```

### Advanced / Debug

```bash
# Low-level debugging
op-dbus debug ovsdb                        # Test OVSDB connection
op-dbus debug netlink                      # Test Netlink
op-dbus debug dbus                         # Test D-Bus connection
op-dbus debug all                          # Test all protocols

# Footprint debugging
op-dbus footprint generate state.json      # Generate footprint for state
op-dbus footprint compare HASH1 HASH2      # Compare two footprints
op-dbus footprint show HASH                # Show footprint details

# Raw protocol access (debugging)
op-dbus raw ovsdb LIST_BRIDGES             # Raw OVSDB JSON-RPC
op-dbus raw netlink GET_LINK eth0          # Raw Netlink command
```

## Command Aliases

Common shortcuts:

```bash
op-dbus q          # alias for query
op-dbus a          # alias for apply
op-dbus d          # alias for diff
op-dbus r          # alias for run
op-dbus v          # alias for verify
op-dbus bc         # alias for blockchain
op-dbus ct         # alias for container
```

## Output Formats

All commands support:
- `--format json` (default for machine-readable)
- `--format yaml` (human-friendly structured)
- `--format text` (human-friendly plain text)
- `--format table` (tabular display)
- `--quiet` (minimal output)
- `--verbose` (detailed output)
- `--output FILE` (save to file)

## Examples

### Common Workflows

**Initial setup:**
```bash
# Auto-detect current network config
op-dbus init --introspect > /etc/op-dbus/state.json

# Verify it looks right
op-dbus query

# Apply it (creates genesis block)
op-dbus apply /etc/op-dbus/state.json
```

**Make a change:**
```bash
# Edit state file
vim /etc/op-dbus/state.json

# Preview changes
op-dbus diff /etc/op-dbus/state.json

# Create checkpoint before applying
op-dbus checkpoint create --name "before-network-change"

# Apply with blockchain record
op-dbus apply /etc/op-dbus/state.json

# Verify it worked
op-dbus verify

# If something broke, rollback
op-dbus rollback --to-checkpoint before-network-change
```

**Create netmaker container:**
```bash
# Add to state.json, then:
op-dbus apply /etc/op-dbus/state.json

# Or directly:
op-dbus container create 100 --network-type netmaker

# Check status
op-dbus container show 100

# View blockchain record
op-dbus blockchain show latest
```

**Audit changes:**
```bash
# Show all network changes
op-dbus blockchain list --plugin net

# Search for specific interface
op-dbus blockchain search "vmbr0"

# Export for compliance audit
op-dbus blockchain export --since 2025-01-01 --output audit-Q1-2025.json

# Verify blockchain integrity
op-dbus blockchain verify --full
```

**Debug issues:**
```bash
# Check system health
op-dbus doctor

# Test protocols
op-dbus debug all

# Show detailed status
op-dbus status --verbose

# Check logs
op-dbus logs --follow
```

## Implementation Priority

### Phase 1: Core (Partially Done)
- [x] `run`
- [x] `query`
- [x] `diff`
- [x] `apply`
- [ ] `verify`

### Phase 2: Blockchain
- [ ] `blockchain list`
- [ ] `blockchain show`
- [ ] `blockchain export`
- [ ] `blockchain verify`
- [ ] `blockchain search`

### Phase 3: Rollback
- [ ] `checkpoint create`
- [ ] `checkpoint list`
- [ ] `rollback`

### Phase 4: Container Management
- [ ] `container list`
- [ ] `container show`
- [ ] `container create/start/stop/destroy`
- [ ] `container netmaker-join/leave`

### Phase 5: Network Helpers
- [ ] `net bridges`
- [ ] `net interfaces`
- [ ] `net routes`
- [ ] `ovs status`

### Phase 6: Convenience
- [ ] `init --introspect`
- [ ] `validate`
- [ ] `doctor`
- [ ] `logs`
- [ ] `status`

### Phase 7: Advanced
- [ ] `plugin list`
- [ ] `debug` commands
- [ ] `footprint` commands
- [ ] Raw protocol access

## CLI Structure (Implementation)

```rust
#[derive(Parser)]
#[command(name = "op-dbus", version, about = "Declarative system state via native protocols")]
struct Cli {
    #[arg(short, long, global = true)]
    state_file: Option<PathBuf>,

    #[arg(long, global = true)]
    format: Option<OutputFormat>,

    #[arg(long, global = true)]
    verbose: bool,

    #[arg(long, global = true)]
    quiet: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the daemon
    Run {
        #[arg(long)]
        enable_dhcp_server: bool,
        #[arg(long)]
        oneshot: bool,
        #[arg(long)]
        watch: bool,
    },

    /// Query current state
    Query {
        #[arg(short, long)]
        plugin: Option<String>,
    },

    /// Show diff
    Diff {
        state_file: PathBuf,
        #[arg(short, long)]
        plugin: Option<String>,
    },

    /// Apply state
    Apply {
        state_file: PathBuf,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        checkpoint: bool,
        #[arg(short, long)]
        plugin: Option<String>,
    },

    /// Verify state
    Verify {
        #[arg(long)]
        footprint: Option<String>,
        #[arg(long)]
        full: bool,
    },

    /// Blockchain commands
    Blockchain {
        #[command(subcommand)]
        command: BlockchainCommands,
    },

    /// Checkpoint commands
    Checkpoint {
        #[command(subcommand)]
        command: CheckpointCommands,
    },

    /// Rollback state
    Rollback {
        #[arg(long)]
        to_block: Option<u64>,
        #[arg(long)]
        to_checkpoint: Option<String>,
        #[arg(long)]
        dry_run: bool,
        #[arg(short, long)]
        plugin: Option<String>,
    },

    /// Container management
    Container {
        #[command(subcommand)]
        command: ContainerCommands,
    },

    /// Network helpers
    Net {
        #[command(subcommand)]
        command: NetCommands,
    },

    /// OVS helpers
    Ovs {
        #[command(subcommand)]
        command: OvsCommands,
    },

    /// Plugin management
    Plugin {
        #[command(subcommand)]
        command: PluginCommands,
    },

    /// Initialize config
    Init {
        #[arg(long)]
        introspect: bool,
        #[arg(long)]
        template: Option<String>,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Validate config
    Validate {
        state_file: PathBuf,
        #[arg(long)]
        strict: bool,
        #[arg(short, long)]
        plugin: Option<String>,
    },

    /// Show configuration
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },

    /// System diagnostics
    Doctor {
        #[arg(long)]
        fix: bool,
    },

    /// Show logs
    Logs {
        #[arg(long)]
        follow: bool,
        #[arg(long)]
        since: Option<String>,
    },

    /// Daemon status
    Status {
        #[arg(long)]
        verbose: bool,
    },

    /// Health check
    Health,

    /// Debug commands
    Debug {
        #[command(subcommand)]
        command: DebugCommands,
    },
}
```

## Notes

- All Rust, no Go code
- Single binary: `op-dbus`
- Native protocols: OVSDB JSON-RPC, Netlink, D-Bus
- No external agents or helpers
- Clean, consistent CLI interface

# Example NixOS configuration for op-dbus
# This demonstrates a complete deployment with all enterprise features enabled
#
# Usage:
# 1. Copy this to /etc/nixos/configuration.nix or import it as a module
# 2. Adjust settings based on your hardware (NUMA, BTRFS, etc.)
# 3. Run: sudo nixos-rebuild switch

{ config, pkgs, ... }:

{
  # Import the op-dbus module
  # In production, you would import from the flake or nixpkgs
  imports = [
    ./nixos-module.nix
  ];

  # ===========================================================================
  # FILESYSTEM REQUIREMENTS
  # ===========================================================================
  # op-dbus blockchain requires BTRFS for optimal performance (subvolumes, snapshots)
  fileSystems."/var/lib/op-dbus" = {
    device = "/dev/disk/by-label/opdbus";
    fsType = "btrfs";
    options = [
      "compress=zstd"           # 3-5x compression for blockchain data
      "noatime"                 # Reduce write amplification
      "space_cache=v2"          # Faster free space tracking
    ];
  };

  # ===========================================================================
  # OP-DBUS SERVICE CONFIGURATION
  # ===========================================================================
  services.op-dbus = {
    enable = true;

    # Use the op-dbus package from the flake
    # package = pkgs.op-dbus;  # Uncomment when available in nixpkgs

    # ---------------------------------------------------------------------------
    # BLOCKCHAIN CONFIGURATION
    # ---------------------------------------------------------------------------
    # Cryptographic audit trail with disaster recovery snapshots
    blockchain = {
      enable = true;

      # Blockchain storage path (must be on BTRFS)
      path = "/var/lib/op-dbus/blockchain";

      # Snapshot frequency - balance between recovery granularity and disk usage
      # Options: per-operation, every-minute, every-5-minutes, every-15-minutes,
      #          every-30-minutes, hourly, daily, weekly
      snapshotInterval = "every-15-minutes";

      # Retention policy - rolling windows with automatic pruning
      retention = {
        # Keep last 10 hourly snapshots (last ~10 hours)
        hourly = 10;

        # Keep last 7 daily snapshots (last week)
        daily = 7;

        # Keep last 4 weekly snapshots (last month)
        weekly = 4;

        # Keep last 4 quarterly snapshots (last year)
        quarterly = 4;
      };
    };

    # ---------------------------------------------------------------------------
    # NUMA OPTIMIZATION (Enterprise DGX/Multi-Socket Systems)
    # ---------------------------------------------------------------------------
    # Provides 2-3x performance improvement on multi-socket servers
    numa = {
      enable = true;

      # NUMA placement strategy:
      # - local-node:  Pin to current NUMA node (best for DGX)
      # - round-robin: Distribute across all nodes (load balancing)
      # - most-memory: Use node with most available memory
      # - disabled:    No NUMA optimization
      strategy = "local-node";

      # Optional: Force specific NUMA node (0-N)
      # null = auto-detect optimal node
      nodePreference = null;
    };

    # ---------------------------------------------------------------------------
    # BTRFS CACHE CONFIGURATION
    # ---------------------------------------------------------------------------
    # ML embedding cache with NUMA-aware memory placement
    cache = {
      # BTRFS compression algorithm
      # Options: zstd (best compression), lzo (fast), zlib (compatible), none
      compression = "zstd";

      # Maximum number of cache snapshots to retain
      maxSnapshots = 24;
    };

    # ---------------------------------------------------------------------------
    # STATE MANAGEMENT
    # ---------------------------------------------------------------------------
    # Declarative state configuration
    state = {
      version = 1;

      plugins = {
        # Systemd service management
        systemd = {
          units = {
            # Example: Ensure nginx is running and enabled
            "nginx.service" = {
              active_state = "active";
              enabled = true;
            };

            # Example: Ensure unnecessary services are stopped
            "cups.service" = {
              active_state = "inactive";
              enabled = false;
            };
          };
        };

        # Network configuration (requires OpenVSwitch)
        net = {
          interfaces = [
            {
              name = "br0";
              type = "ovs-bridge";
              ports = [ "eth0" "eth1" ];
              ipv4 = {
                enabled = true;
                dhcp = false;
                address = [
                  { ip = "192.168.1.10"; prefix = 24; }
                ];
                gateway = "192.168.1.1";
              };
            }
          ];
        };
      };
    };

    # ---------------------------------------------------------------------------
    # PLUGIN CONFIGURATION
    # ---------------------------------------------------------------------------
    # Built-in plugins to enable
    # Plugins gracefully skip if dependencies unavailable
    enabledPlugins = [
      "systemd"   # Always available
      "login1"    # Session management
      "net"       # Requires OpenVSwitch
      # "lxc"     # Uncomment if using LXC containers
    ];

    # Automatic D-Bus service discovery
    # Generates plugins for all discoverable D-Bus services
    autoDiscovery = true;

    # ---------------------------------------------------------------------------
    # PATHS
    # ---------------------------------------------------------------------------
    stateFile = "/etc/op-dbus/state.json";
    dataDir = "/var/lib/op-dbus";

    # ---------------------------------------------------------------------------
    # SECRETS (Optional)
    # ---------------------------------------------------------------------------
    # Load additional environment variables from files
    # Useful for Netmaker tokens, API keys, etc.
    environmentFiles = [
      # "/etc/op-dbus/secrets.env"
    ];
  };

  # ===========================================================================
  # REQUIRED SYSTEM SERVICES
  # ===========================================================================

  # D-Bus is required for op-dbus operation
  services.dbus.enable = true;

  # OpenVSwitch (optional, required for 'net' plugin)
  # Automatically enabled if 'net' is in enabledPlugins
  # virtualisation.openvswitch.enable = true;

  # ===========================================================================
  # SYSTEM PACKAGES
  # ===========================================================================
  # The op-dbus CLI is automatically added to systemPackages

  # ===========================================================================
  # MONITORING & OBSERVABILITY
  # ===========================================================================
  # Example: Export op-dbus metrics to Prometheus
  # services.prometheus.exporters.node = {
  #   enable = true;
  #   enabledCollectors = [ "systemd" "btrfs" ];
  # };

  # ===========================================================================
  # FIREWALL (Optional)
  # ===========================================================================
  # networking.firewall.allowedTCPPorts = [ 8080 ];  # If exposing API

  # ===========================================================================
  # HARDWARE OPTIMIZATION
  # ===========================================================================
  # For multi-socket systems: ensure NUMA is enabled in BIOS
  # For DGX systems: use 'local-node' strategy
  # For high-throughput workloads: increase inotify limits
  boot.kernel.sysctl = {
    "fs.inotify.max_user_watches" = 524288;
    "fs.inotify.max_user_instances" = 512;
  };
}

{ config, lib, pkgs, ... }:

with lib;

let
  cfg = config.services.op-dbus;

  # Generate state file from NixOS configuration
  stateFile = pkgs.writeText "op-dbus-state.json" (builtins.toJSON cfg.state);

in {
  options.services.op-dbus = {
    enable = mkEnableOption "op-dbus declarative state management";

    package = mkOption {
      type = types.package;
      default = pkgs.op-dbus or (throw "op-dbus package not available. Add overlay or build from source.");
      defaultText = "pkgs.op-dbus";
      description = "The op-dbus package to use";
    };

    state = mkOption {
      type = types.attrs;
      default = {
        version = 1;
        plugins = {
          systemd = {
            units = {};
          };
        };
      };
      example = literalExpression ''
        {
          version = 1;
          plugins = {
            net = {
              interfaces = [{
                name = "br0";
                type = "ovs-bridge";
                ports = [ "eth0" ];
                ipv4 = {
                  enabled = true;
                  dhcp = false;
                  address = [{ ip = "192.168.1.10"; prefix = 24; }];
                  gateway = "192.168.1.1";
                };
              }];
            };
            systemd = {
              units = {
                "nginx.service" = {
                  active_state = "active";
                  enabled = true;
                };
              };
            };
          };
        }
      '';
      description = ''
        Declarative state configuration for op-dbus.
        This will be written to /etc/op-dbus/state.json
      '';
    };

    autoDiscovery = mkOption {
      type = types.bool;
      default = true;
      description = ''
        Enable automatic D-Bus service discovery.
        When enabled, op-dbus will auto-generate plugins for all
        discoverable D-Bus services on the system.
      '';
    };

    enabledPlugins = mkOption {
      type = types.listOf types.str;
      default = [ "systemd" "login1" ];
      example = [ "systemd" "login1" "net" "lxc" ];
      description = ''
        List of built-in plugins to enable.
        Plugins will be automatically skipped if their dependencies
        are not available (e.g., 'net' requires OpenVSwitch).
      '';
    };

    stateFile = mkOption {
      type = types.path;
      default = "/etc/op-dbus/state.json";
      description = "Path to the state file";
    };

    dataDir = mkOption {
      type = types.path;
      default = "/var/lib/op-dbus";
      description = "Directory for blockchain storage";
    };

    environmentFiles = mkOption {
      type = types.listOf types.path;
      default = [];
      example = [ "/etc/op-dbus/secrets.env" ];
      description = ''
        Additional environment files to load.
        Useful for secrets like Netmaker tokens.
      '';
    };

    blockchain = {
      enable = mkOption {
        type = types.bool;
        default = true;
        description = "Enable blockchain audit trail and state snapshots";
      };

      path = mkOption {
        type = types.path;
        default = "${cfg.dataDir}/blockchain";
        description = "Path to blockchain storage (requires BTRFS for optimal performance)";
      };

      snapshotInterval = mkOption {
        type = types.enum [
          "per-operation"
          "every-minute"
          "every-5-minutes"
          "every-15-minutes"
          "every-30-minutes"
          "hourly"
          "daily"
          "weekly"
        ];
        default = "every-15-minutes";
        description = "How often to create BTRFS snapshots";
      };

      retention = {
        hourly = mkOption {
          type = types.int;
          default = 5;
          description = "Number of hourly snapshots to keep (last 24 hours)";
        };

        daily = mkOption {
          type = types.int;
          default = 5;
          description = "Number of daily snapshots to keep (last 30 days)";
        };

        weekly = mkOption {
          type = types.int;
          default = 5;
          description = "Number of weekly snapshots to keep (last 12 weeks)";
        };

        quarterly = mkOption {
          type = types.int;
          default = 5;
          description = "Number of quarterly snapshots to keep (long-term)";
        };
      };
    };

    cache = {
      compression = mkOption {
        type = types.enum [ "zstd" "lzo" "zlib" "none" ];
        default = "zstd";
        description = ''
          BTRFS compression algorithm for cache.
          zstd provides 3-5x compression with good performance.
        '';
      };

      maxSnapshots = mkOption {
        type = types.int;
        default = 24;
        description = "Maximum number of cache snapshots to keep";
      };
    };

    numa = {
      enable = mkOption {
        type = types.bool;
        default = true;
        description = ''
          Enable NUMA optimization for multi-socket systems.
          Automatically detects topology and applies CPU affinity.
          Provides 2-3x performance improvement on DGX/multi-socket servers.
        '';
      };

      strategy = mkOption {
        type = types.enum [ "local-node" "round-robin" "most-memory" "disabled" ];
        default = "local-node";
        description = ''
          NUMA placement strategy:
          - local-node: Pin to current NUMA node (best for DGX)
          - round-robin: Distribute across all nodes
          - most-memory: Use node with most available memory
          - disabled: No NUMA optimization
        '';
      };

      nodePreference = mkOption {
        type = types.nullOr types.int;
        default = null;
        example = 0;
        description = "Preferred NUMA node (0-N). null = auto-detect";
      };
    };
  };

  config = mkIf cfg.enable {
    # Ensure systemd and D-Bus are available
    assertions = [
      {
        assertion = config.systemd.package != null;
        message = "op-dbus requires systemd";
      }
      {
        assertion = config.services.dbus.enable;
        message = "op-dbus requires D-Bus (services.dbus.enable = true)";
      }
    ];

    # Install the package
    environment.systemPackages = [ cfg.package ];

    # Create state file
    environment.etc."op-dbus/state.json" = {
      text = builtins.toJSON cfg.state;
      mode = "0644";
    };

    # Create data directory
    systemd.tmpfiles.rules = [
      "d ${cfg.dataDir} 0700 root root -"
      "d ${cfg.dataDir}/blockchain 0700 root root -"
      "d ${cfg.dataDir}/blockchain/timing 0700 root root -"
      "d ${cfg.dataDir}/blockchain/vectors 0700 root root -"
      "d ${cfg.dataDir}/blockchain/state 0700 root root -"
      "d ${cfg.dataDir}/blockchain/snapshots 0700 root root -"
      "d /run/op-dbus 0755 root root -"
    ];

    # Systemd service
    systemd.services.op-dbus = {
      description = "op-dbus - Declarative system state management";
      documentation = [ "https://github.com/repr0bated/operation-dbus" ];

      after = [ "network-online.target" "dbus.service" ]
        ++ optional config.virtualisation.openvswitch.enable "openvswitch.service";

      wants = [ "network-online.target" ];

      requires = [ "dbus.service" ]
        ++ optional config.virtualisation.openvswitch.enable "openvswitch.service";

      wantedBy = [ "multi-user.target" ];

      serviceConfig = {
        Type = "simple";
        ExecStart = "${cfg.package}/bin/op-dbus run --state-file ${cfg.stateFile}";
        Restart = "on-failure";
        RestartSec = "5s";

        # Security hardening
        NoNewPrivileges = false;  # Needs to be false for CAP_NET_ADMIN
        PrivateTmp = true;
        ProtectSystem = "strict";
        ProtectHome = true;
        ReadWritePaths = [
          cfg.dataDir
          "/run/op-dbus"
        ];

        # Network capabilities for OVS and netlink
        AmbientCapabilities = [ "CAP_NET_ADMIN" "CAP_NET_RAW" ];
        CapabilityBoundingSet = [ "CAP_NET_ADMIN" "CAP_NET_RAW" ];

        # Environment
        Environment = [
          # Blockchain configuration
          "OPDBUS_BLOCKCHAIN_ENABLED=${if cfg.blockchain.enable then "true" else "false"}"
          "OPDBUS_BLOCKCHAIN_PATH=${cfg.blockchain.path}"
          "OPDBUS_SNAPSHOT_INTERVAL=${cfg.blockchain.snapshotInterval}"
          "OPDBUS_RETAIN_HOURLY=${toString cfg.blockchain.retention.hourly}"
          "OPDBUS_RETAIN_DAILY=${toString cfg.blockchain.retention.daily}"
          "OPDBUS_RETAIN_WEEKLY=${toString cfg.blockchain.retention.weekly}"
          "OPDBUS_RETAIN_QUARTERLY=${toString cfg.blockchain.retention.quarterly}"

          # NUMA optimization
          "OPDBUS_NUMA_ENABLED=${if cfg.numa.enable then "true" else "false"}"
          "OPDBUS_NUMA_STRATEGY=${cfg.numa.strategy}"

          # Cache configuration
          "OPDBUS_CACHE_COMPRESSION=${cfg.cache.compression}"
          "OPDBUS_CACHE_MAX_SNAPSHOTS=${toString cfg.cache.maxSnapshots}"

          # Data directory
          "OPDBUS_DATA_DIR=${cfg.dataDir}"
        ] ++ optional (cfg.numa.nodePreference != null)
          "OPDBUS_NUMA_NODE_PREFERENCE=${toString cfg.numa.nodePreference}";

        EnvironmentFile = cfg.environmentFiles;
      };
    };

    # Optional: Enable OpenVSwitch if net plugin is requested
    virtualisation.openvswitch.enable =
      mkIf (builtins.elem "net" cfg.enabledPlugins)
        (mkDefault true);
  };
}

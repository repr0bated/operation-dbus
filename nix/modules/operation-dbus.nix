{ config, lib, pkgs, ... }:

with lib;

let
  cfg = config.services.operation-dbus;

  # State file format
  stateFileType = types.submodule {
    options = {
      version = mkOption {
        type = types.int;
        default = 1;
        description = "State file format version";
      };

      plugins = mkOption {
        type = types.attrsOf types.attrs;
        default = {};
        description = "Plugin-specific state configuration";
      };
    };
  };

  # Generate state file from configuration
  generateStateFile = cfg: pkgs.writeText "op-dbus-state.json" (builtins.toJSON {
    version = 1;
    plugins = mkMerge [
      (optionalAttrs cfg.plugins.systemd.enable {
        systemd = cfg.plugins.systemd.config;
      })
      (optionalAttrs cfg.plugins.network.enable {
        net = cfg.plugins.network.config;
      })
      (optionalAttrs cfg.plugins.lxc.enable {
        lxc = cfg.plugins.lxc.config;
      })
      (optionalAttrs cfg.plugins.login1.enable {
        login1 = cfg.plugins.login1.config;
      })
      (optionalAttrs cfg.plugins.dnsresolver.enable {
        dnsresolver = cfg.plugins.dnsresolver.config;
      })
      (optionalAttrs cfg.plugins.openflow.enable {
        openflow = cfg.plugins.openflow.config;
      })
    ];
  });

in
{
  options.services.operation-dbus = {
    enable = mkEnableOption "Operation D-Bus declarative system state management";

    package = mkOption {
      type = types.package;
      default = pkgs.operation-dbus;
      defaultText = literalExpression "pkgs.operation-dbus";
      description = "The operation-dbus package to use";
    };

    stateFile = mkOption {
      type = types.nullOr types.path;
      default = null;
      description = ''
        Path to state file for declarative configuration.
        If null, will be generated from plugin configurations.
      '';
    };

    dataDir = mkOption {
      type = types.path;
      default = "/var/lib/op-dbus";
      description = "Data directory for operation-dbus";
    };

    cacheDir = mkOption {
      type = types.path;
      default = "/var/cache/op-dbus";
      description = "Cache directory for BTRFS snapshots and SQLite cache";
    };

    logLevel = mkOption {
      type = types.enum [ "error" "warn" "info" "debug" "trace" ];
      default = "info";
      description = "Logging level";
    };

    oneshot = mkOption {
      type = types.bool;
      default = false;
      description = ''
        Run in oneshot mode (apply state and exit) instead of daemon mode.
        Useful for testing or boot-time configuration.
      '';
    };

    # Plugin configurations
    plugins = {
      systemd = {
        enable = mkEnableOption "Systemd service management plugin";

        config = mkOption {
          type = types.attrs;
          default = { units = {}; };
          description = "Systemd plugin configuration";
          example = literalExpression ''
            {
              units = {
                "nginx.service" = {
                  active_state = "active";
                  enabled = true;
                };
              };
            }
          '';
        };
      };

      network = {
        enable = mkEnableOption "Network and OVS management plugin";

        config = mkOption {
          type = types.attrs;
          default = { interfaces = []; };
          description = "Network plugin configuration";
          example = literalExpression ''
            {
              interfaces = [
                {
                  name = "ovsbr0";
                  type = "ovs-bridge";
                  ports = [ "eth0" ];
                  ipv4 = {
                    enabled = true;
                    dhcp = false;
                    address = [
                      { ip = "192.168.1.10"; prefix = 24; }
                    ];
                  };
                }
              ];
            }
          '';
        };
      };

      lxc = {
        enable = mkEnableOption "LXC container management plugin";

        config = mkOption {
          type = types.attrs;
          default = { containers = []; };
          description = "LXC plugin configuration";
          example = literalExpression ''
            {
              containers = [
                {
                  name = "web-server";
                  state = "running";
                  config = {
                    image = "ubuntu/22.04";
                    network = { type = "veth"; };
                  };
                }
              ];
            }
          '';
        };
      };

      login1 = {
        enable = mkEnableOption "User session management plugin (login1)";

        config = mkOption {
          type = types.attrs;
          default = {};
          description = "Login1 plugin configuration";
        };
      };

      dnsresolver = {
        enable = mkEnableOption "DNS resolver configuration plugin";

        config = mkOption {
          type = types.attrs;
          default = {};
          description = "DNS resolver plugin configuration";
        };
      };

      openflow = {
        enable = mkEnableOption "OpenFlow controller plugin";

        config = mkOption {
          type = types.attrs;
          default = { bridges = []; flows = []; };
          description = "OpenFlow plugin configuration";
          example = literalExpression ''
            {
              bridges = [
                {
                  name = "mesh";
                  controller = "tcp:127.0.0.1:6653";
                }
              ];
              flows = [];
            }
          '';
        };
      };
    };

    # Blockchain/audit configuration
    blockchain = {
      enable = mkOption {
        type = types.bool;
        default = true;
        description = "Enable blockchain audit trail";
      };

      useML = mkOption {
        type = types.bool;
        default = false;
        description = "Enable ML-based vectorization (requires additional dependencies)";
      };
    };

    # Cache configuration
    cache = {
      enable = mkOption {
        type = types.bool;
        default = true;
        description = "Enable BTRFS-based caching";
      };

      maxSize = mkOption {
        type = types.nullOr types.str;
        default = "10G";
        description = "Maximum cache size (null for unlimited)";
      };
    };
  };

  config = mkIf cfg.enable {
    # Ensure required system packages are available
    environment.systemPackages = [ cfg.package ];

    # Create system user
    users.users.op-dbus = {
      description = "Operation D-Bus daemon user";
      isSystemUser = true;
      group = "op-dbus";
      home = cfg.dataDir;
    };

    users.groups.op-dbus = {};

    # Create data directories
    systemd.tmpfiles.rules = [
      "d ${cfg.dataDir} 0750 op-dbus op-dbus -"
      "d ${cfg.cacheDir} 0750 op-dbus op-dbus -"
      "d ${cfg.dataDir}/blockchain 0750 op-dbus op-dbus -"
      "d ${cfg.dataDir}/checkpoints 0750 op-dbus op-dbus -"
    ];

    # Systemd service
    systemd.services.operation-dbus = {
      description = "Operation D-Bus - Declarative System State Management";
      documentation = [ "https://github.com/repr0bated/operation-dbus" ];
      after = [ "network.target" "dbus.service" "systemd-logind.service" ];
      wants = [ "network.target" ];
      wantedBy = [ "multi-user.target" ];

      # Service configuration
      serviceConfig = {
        Type = if cfg.oneshot then "oneshot" else "simple";
        User = "root";  # Required for system management
        Group = "root";
        Restart = if cfg.oneshot then "no" else "on-failure";
        RestartSec = "10s";

        # Security hardening
        NoNewPrivileges = false;  # Need privileges for system management
        PrivateTmp = true;
        ProtectSystem = "strict";
        ProtectHome = true;
        ReadWritePaths = [ cfg.dataDir cfg.cacheDir "/var/lib/lxc" "/run" "/sys" ];

        # Resource limits
        LimitNOFILE = 65536;
        LimitNPROC = 512;

        # Capabilities needed for system management
        AmbientCapabilities = [
          "CAP_NET_ADMIN"
          "CAP_SYS_ADMIN"
          "CAP_DAC_OVERRIDE"
        ];
        CapabilityBoundingSet = [
          "CAP_NET_ADMIN"
          "CAP_SYS_ADMIN"
          "CAP_DAC_OVERRIDE"
          "CAP_SETUID"
          "CAP_SETGID"
        ];

        # Environment
        Environment = [
          "RUST_LOG=${cfg.logLevel}"
          "OP_DBUS_DATA_DIR=${cfg.dataDir}"
          "OP_DBUS_CACHE_DIR=${cfg.cacheDir}"
        ];

        # Execute command
        ExecStart = let
          stateArg = if cfg.stateFile != null
                     then cfg.stateFile
                     else generateStateFile cfg;
          runCmd = if cfg.oneshot
                   then "${cfg.package}/bin/op-dbus run --oneshot"
                   else "${cfg.package}/bin/op-dbus run";
        in "${runCmd}";
      };
    };

    # D-Bus service file
    services.dbus.packages = [ cfg.package ];

    # Enable required system services based on plugins
    systemd.services = mkMerge [
      (mkIf cfg.plugins.network.enable {
        openvswitch = {
          enable = true;
          wantedBy = [ "operation-dbus.service" ];
          before = [ "operation-dbus.service" ];
        };
      })
    ];

    # Enable LXC if plugin is enabled
    virtualisation.lxc.enable = mkIf cfg.plugins.lxc.enable true;

    # Assertions
    assertions = [
      {
        assertion = cfg.plugins.network.enable -> config.virtualisation.openvswitch.enable or false;
        message = "Network plugin requires Open vSwitch to be enabled";
      }
      {
        assertion = cfg.plugins.lxc.enable -> config.virtualisation.lxc.enable;
        message = "LXC plugin requires LXC to be enabled";
      }
    ];
  };
}

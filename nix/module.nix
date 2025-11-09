# NixOS module for op-dbus
{ config, lib, pkgs, ... }:

with lib;

let
  cfg = config.services.op-dbus;

  # Generate state.json based on configuration
  stateFile = pkgs.writeText "op-dbus-state.json" (builtins.toJSON {
    version = 1;
    plugins = cfg.stateConfig;
  });

in {
  options.services.op-dbus = {
    enable = mkEnableOption "op-dbus declarative system state management";

    package = mkOption {
      type = types.package;
      default = pkgs.callPackage ./package.nix { };
      description = "The op-dbus package to use";
    };

    mode = mkOption {
      type = types.enum [ "full" "standalone" "agent" ];
      default = "standalone";
      description = ''
        Deployment mode:
        - full: D-Bus + Blockchain + LXC + Netmaker (requires Proxmox)
        - standalone: D-Bus + Blockchain (no containers)
        - agent: D-Bus plugins only (minimal)
      '';
    };

    stateConfig = mkOption {
      type = types.attrs;
      default = { };
      description = ''
        The declarative state configuration.
        This will be converted to /etc/op-dbus/state.json

        Example:
        {
          net = {
            interfaces = [{
              name = "ovsbr0";
              type = "ovs-bridge";
              ports = [];
              ipv4 = {
                enabled = true;
                dhcp = false;
                address = [];
                gateway = null;
              };
            }];
          };
          systemd = {
            units = {
              "openvswitch-switch.service" = {
                enabled = true;
                active_state = "active";
              };
            };
          };
        }
      '';
    };

    dataDir = mkOption {
      type = types.path;
      default = "/var/lib/op-dbus";
      description = "Data directory for op-dbus";
    };

    enableBlockchain = mkOption {
      type = types.bool;
      default = cfg.mode != "agent";
      description = "Enable blockchain audit logging";
    };

    enableCache = mkOption {
      type = types.bool;
      default = true;
      description = "Enable BTRFS-based caching";
    };

    # TODO: NUMA configuration options
    # numaCpuAffinity = mkOption { ... };
    # numaPolicy = mkOption { ... };
  };

  config = mkIf cfg.enable {
    # Install the package
    environment.systemPackages = [ cfg.package ];

    # Enable OpenVSwitch if not agent-only mode
    services.openvswitch.enable = mkIf (cfg.mode != "agent") true;

    # Create state file in /etc
    environment.etc."op-dbus/state.json" = {
      text = builtins.toJSON {
        version = 1;
        plugins = cfg.stateConfig;
      };
    };

    # Create directory structure via tmpfiles
    systemd.tmpfiles.rules = [
      # Config directory
      "d /etc/op-dbus 0755 root root -"

      # Data directory
      "d ${cfg.dataDir} 0755 root root -"

      # Blockchain directories (always created)
      "d ${cfg.dataDir}/blockchain 0755 root root -"
      "d ${cfg.dataDir}/blockchain/timing 0755 root root -"
      "d ${cfg.dataDir}/blockchain/vectors 0755 root root -"
      "d ${cfg.dataDir}/blockchain/snapshots 0755 root root -"

      # Cache directory (TODO: BTRFS subvolume)
      "d ${cfg.dataDir}/@cache 0755 root root -"

      # Runtime directory
      "d /run/op-dbus 0755 root root -"
    ];

    # Systemd service definition
    systemd.services.op-dbus = {
      description = "op-dbus - Declarative system state management";
      documentation = [ "https://github.com/ghostbridge/op-dbus" ];

      # Dependencies based on mode
      after = [ "network-online.target" ]
        ++ optional (cfg.mode != "agent") "openvswitch.service";
      wants = [ "network-online.target" ];
      requires = optional (cfg.mode != "agent") "openvswitch.service";

      wantedBy = [ "multi-user.target" ];

      serviceConfig = {
        Type = "simple";
        ExecStart = "${cfg.package}/bin/op-dbus run --state-file /etc/op-dbus/state.json";
        Restart = "on-failure";
        RestartSec = "5s";

        # Security hardening
        NoNewPrivileges = false;
        PrivateTmp = true;
        ProtectSystem = "strict";
        ProtectHome = true;
        ReadWritePaths = [
          "/etc/network/interfaces"
          "/run"
          "/var/run"
          "/etc/dnsmasq.d"
          cfg.dataDir
        ];

        # Network capabilities
        AmbientCapabilities = [ "CAP_NET_ADMIN" "CAP_NET_RAW" ];
        CapabilityBoundingSet = [ "CAP_NET_ADMIN" "CAP_NET_RAW" ];

        # TODO: NUMA configuration
        # CPUAffinity = cfg.numaCpuAffinity;
        # NUMAPolicy = cfg.numaPolicy;
      };
    };

    # Default state configuration based on mode
    services.op-dbus.stateConfig = mkDefault (
      if cfg.mode == "full" then {
        net = {
          interfaces = [
            {
              name = "ovsbr0";
              type = "ovs-bridge";
              ports = [];
              ipv4 = {
                enabled = true;
                dhcp = false;
                address = [];
                gateway = null;
              };
            }
            {
              name = "mesh";
              type = "ovs-bridge";
              ports = [];
              ipv4 = {
                enabled = true;
                dhcp = false;
                address = [];
                gateway = null;
              };
            }
          ];
        };
        systemd = {
          units = {
            "openvswitch.service" = {
              enabled = true;
              active_state = "active";
            };
          };
        };
        lxc = {
          containers = [];
        };
      }
      else if cfg.mode == "standalone" then {
        net = {
          interfaces = [
            {
              name = "ovsbr0";
              type = "ovs-bridge";
              ports = [];
              ipv4 = {
                enabled = true;
                dhcp = false;
                address = [];
                gateway = null;
              };
            }
          ];
        };
        systemd = {
          units = {
            "openvswitch.service" = {
              enabled = true;
              active_state = "active";
            };
          };
        };
      }
      else {  # agent mode
        systemd = {
          units = {};
        };
      }
    );
  };
}

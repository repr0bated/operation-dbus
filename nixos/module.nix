# NixOS Module for op-dbus
# Declarative system state management via D-Bus with MCP integration

{ config, lib, pkgs, ... }:

with lib;

let
  cfg = config.services.op-dbus;

  # Build op-dbus from source
  op-dbus-package = pkgs.rustPlatform.buildRustPackage rec {
    pname = "op-dbus";
    version = "0.1.0";

    src = ./..;

    cargoLock = {
      lockFile = ../Cargo.lock;
    };

    nativeBuildInputs = with pkgs; [
      pkg-config
      rustPlatform.bindgenHook
    ];

    buildInputs = with pkgs; [
      dbus
      systemd
      openssl
      openvswitch
    ];

    # Build with MCP features
    buildFeatures = [ "mcp" ];

    # Run tests
    checkPhase = ''
      cargo test --release
    '';

    meta = with lib; {
      description = "Declarative system state management via D-Bus";
      homepage = "https://github.com/repr0bated/operation-dbus";
      license = licenses.mit;
      maintainers = [];
      platforms = platforms.linux;
    };
  };

  # Generate state.json from NixOS configuration
  stateFile = pkgs.writeText "op-dbus-state.json" (builtins.toJSON {
    version = 1;
    plugins = {
      net = {
        interfaces = cfg.network.interfaces;
      };
      systemd = {
        units = cfg.systemd.units;
      };
      packagekit = if cfg.packages.enable then {
        installed = cfg.packages.installed;
        removed = cfg.packages.removed;
        auto_update = cfg.packages.autoUpdate;
      } else null;
    };
  });

in {
  options.services.op-dbus = {
    enable = mkEnableOption "op-dbus declarative system state management";

    package = mkOption {
      type = types.package;
      default = op-dbus-package;
      description = "The op-dbus package to use";
    };

    mode = mkOption {
      type = types.enum [ "full" "standalone" "agent-only" ];
      default = "standalone";
      description = ''
        Deployment mode:
        - full: D-Bus + OVS + LXC/Proxmox + Netmaker
        - standalone: D-Bus + OVS (no containers)
        - agent-only: D-Bus plugins only (minimal)
      '';
    };

    stateFile = mkOption {
      type = types.path;
      default = stateFile;
      description = "Path to the state.json configuration file";
    };

    mcp = {
      enable = mkEnableOption "MCP (Model Context Protocol) integration";

      introspection = mkOption {
        type = types.bool;
        default = true;
        description = "Enable automatic D-Bus introspection and MCP tool generation";
      };

      hybridScanner = mkOption {
        type = types.bool;
        default = true;
        description = "Enable hybrid scanner (D-Bus + filesystem + processes + hardware)";
      };

      agents = {
        systemd = mkEnableOption "systemd MCP agent" // { default = true; };
        packagekit = mkEnableOption "PackageKit MCP agent" // { default = true; };
        network = mkEnableOption "network MCP agent" // { default = true; };
        file = mkEnableOption "file MCP agent" // { default = true; };
      };
    };

    network = {
      interfaces = mkOption {
        type = types.listOf (types.submodule {
          options = {
            name = mkOption {
              type = types.str;
              description = "Interface name";
            };
            type = mkOption {
              type = types.enum [ "ovs-bridge" "ethernet" "wifi" "bond" ];
              default = "ethernet";
              description = "Interface type";
            };
            ports = mkOption {
              type = types.listOf types.str;
              default = [];
              description = "Ports to add to bridge (for ovs-bridge type)";
            };
            ipv4 = mkOption {
              type = types.nullOr (types.submodule {
                options = {
                  enabled = mkOption {
                    type = types.bool;
                    default = true;
                  };
                  dhcp = mkOption {
                    type = types.bool;
                    default = false;
                  };
                  addresses = mkOption {
                    type = types.listOf (types.submodule {
                      options = {
                        ip = mkOption { type = types.str; };
                        prefix = mkOption { type = types.int; };
                      };
                    });
                    default = [];
                  };
                  gateway = mkOption {
                    type = types.nullOr types.str;
                    default = null;
                  };
                };
              });
              default = null;
            };
          };
        });
        default = [];
        description = "Network interfaces configuration";
      };

      ovs = {
        enable = mkOption {
          type = types.bool;
          default = true;
          description = "Enable Open vSwitch";
        };

        bridges = mkOption {
          type = types.listOf types.str;
          default = [ "ovsbr0" ];
          description = "OVS bridges to create";
        };
      };
    };

    systemd = {
      units = mkOption {
        type = types.attrsOf (types.submodule {
          options = {
            active_state = mkOption {
              type = types.nullOr (types.enum [ "active" "inactive" ]);
              default = null;
              description = "Desired active state";
            };
            enabled = mkOption {
              type = types.nullOr types.bool;
              default = null;
              description = "Should be enabled at boot";
            };
          };
        });
        default = {};
        description = "Systemd units configuration";
      };
    };

    packages = {
      enable = mkEnableOption "PackageKit plugin for package management";

      installed = mkOption {
        type = types.listOf types.str;
        default = [];
        description = "Packages that must be installed";
      };

      removed = mkOption {
        type = types.listOf types.str;
        default = [];
        description = "Packages that must be removed";
      };

      autoUpdate = mkOption {
        type = types.bool;
        default = false;
        description = "Automatically update packages";
      };
    };
  };

  config = mkIf cfg.enable {
    # Install op-dbus package
    environment.systemPackages = [ cfg.package ];

    # Enable D-Bus
    services.dbus.enable = true;

    # Enable PackageKit if needed
    services.packagekit.enable = mkIf cfg.packages.enable true;

    # Enable Open vSwitch if needed
    services.openvswitch.enable = mkIf cfg.network.ovs.enable true;

    # Create op-dbus configuration directory
    system.activationScripts.op-dbus = ''
      mkdir -p /etc/op-dbus
      ln -sf ${cfg.stateFile} /etc/op-dbus/state.json
    '';

    # Main op-dbus systemd service
    systemd.services.op-dbus = {
      description = "op-dbus declarative system state management";
      wantedBy = [ "multi-user.target" ];
      after = [
        "network.target"
        "dbus.service"
      ] ++ optional cfg.network.ovs.enable "openvswitch.service";

      serviceConfig = {
        Type = "simple";
        ExecStart = "${cfg.package}/bin/op-dbus run --state-file /etc/op-dbus/state.json";
        Restart = "always";
        RestartSec = "10s";

        # Security hardening
        PrivateTmp = true;
        ProtectSystem = "strict";
        ProtectHome = true;
        ReadWritePaths = [ "/etc/op-dbus" "/var/lib/op-dbus" ];
        NoNewPrivileges = false; # Need privileges for system management

        # Capabilities needed
        AmbientCapabilities = [
          "CAP_NET_ADMIN"   # Network management
          "CAP_SYS_ADMIN"   # System administration
        ];
      };

      environment = {
        RUST_LOG = "info";
      };
    };

    # MCP Introspection Service
    systemd.services.op-dbus-introspection = mkIf cfg.mcp.introspection {
      description = "op-dbus system introspection service";
      wantedBy = [ "multi-user.target" ];
      after = [ "dbus.service" ];

      serviceConfig = {
        Type = "oneshot";
        ExecStart = "${cfg.package}/bin/op-dbus introspect-all";
        StandardOutput = "file:/var/lib/op-dbus/introspection.json";
      };
    };

    # Hybrid Scanner Service
    systemd.services.op-dbus-hybrid-scanner = mkIf cfg.mcp.hybridScanner {
      description = "op-dbus hybrid system scanner (D-Bus + filesystem + processes)";
      wantedBy = [ "multi-user.target" ];
      after = [ "dbus.service" "op-dbus.service" ];

      serviceConfig = {
        Type = "dbus";
        BusName = "org.opdbus.HybridSystem";
        ExecStart = "${cfg.package}/bin/op-dbus hybrid-bridge start";
        Restart = "always";
      };
    };

    # MCP Agents
    systemd.services.op-dbus-agent-systemd = mkIf cfg.mcp.agents.systemd {
      description = "op-dbus systemd MCP agent";
      wantedBy = [ "multi-user.target" ];
      after = [ "dbus.service" ];

      serviceConfig = {
        Type = "dbus";
        BusName = "org.dbusmcp.Agent.Systemd";
        ExecStart = "${cfg.package}/bin/op-dbus-agent-systemd";
        Restart = "always";
      };
    };

    systemd.services.op-dbus-agent-packagekit = mkIf cfg.mcp.agents.packagekit {
      description = "op-dbus PackageKit MCP agent";
      wantedBy = [ "multi-user.target" ];
      after = [ "dbus.service" "packagekit.service" ];

      serviceConfig = {
        Type = "dbus";
        BusName = "org.dbusmcp.Agent.PackageKit";
        ExecStart = "${cfg.package}/bin/op-dbus-agent-packagekit";
        Restart = "always";
      };
    };

    # D-Bus policy for op-dbus
    services.dbus.packages = [ cfg.package ];

    # Create data directory
    systemd.tmpfiles.rules = [
      "d /var/lib/op-dbus 0755 root root -"
    ];
  };
}

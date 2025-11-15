# Example NixOS Configuration using Operation D-Bus
# This demonstrates a basic setup with systemd and network plugins

{ config, pkgs, ... }:

{
  imports = [
    # Import the operation-dbus flake modules
    # In a real system, you would add the flake as an input
    # inputs.operation-dbus.nixosModules.default
  ];

  # Enable operation-dbus with basic plugins
  services.operation-dbus = {
    enable = true;

    # Enable specific plugins
    plugins = {
      # Manage systemd services
      systemd = {
        enable = true;
        config = {
          units = {
            "nginx.service" = {
              active_state = "active";
              enabled = true;
            };
            "postgresql.service" = {
              active_state = "active";
              enabled = true;
            };
          };
        };
      };

      # Manage network configuration
      network = {
        enable = true;
        config = {
          interfaces = [
            {
              name = "ovsbr0";
              type = "ovs-bridge";
              ports = [ "eth0" ];
              ipv4 = {
                enabled = true;
                dhcp = false;
                address = [
                  {
                    ip = "192.168.1.100";
                    prefix = 24;
                  }
                ];
                gateway = "192.168.1.1";
              };
            }
          ];
        };
      };

      # DNS resolver
      dnsresolver.enable = true;
    };

    # Blockchain audit trail
    blockchain = {
      enable = true;
      useML = false;
    };

    # Cache configuration
    cache = {
      enable = true;
      maxSize = "10G";
    };

    # Logging
    logLevel = "info";
  };

  # Enable MCP server
  services.operation-dbus.mcp = {
    enable = true;

    # Enable agents
    agents = [
      "executor"   # Command execution
      "systemd"    # Service management
      "file"       # File operations
    ];

    # Enable D-Bus service discovery
    enableDiscovery = true;

    # Orchestrator for multi-agent coordination
    orchestrator = {
      enable = true;
      maxConcurrentTasks = 10;
    };

    logLevel = "info";
  };
}

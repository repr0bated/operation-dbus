# Advanced NixOS Configuration with Operation D-Bus
# This demonstrates a full setup with containers, OpenFlow, and all MCP agents

{ config, pkgs, ... }:

{
  imports = [
    # Import the operation-dbus flake modules
  ];

  # Full operation-dbus setup
  services.operation-dbus = {
    enable = true;

    plugins = {
      # Systemd service management
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
            "redis.service" = {
              active_state = "active";
              enabled = true;
            };
          };
        };
      };

      # Network and OVS management
      network = {
        enable = true;
        config = {
          interfaces = [
            {
              name = "mesh";
              type = "ovs-bridge";
              ports = [ "eth0" "eth1" ];
              ipv4 = {
                enabled = true;
                dhcp = false;
                address = [
                  {
                    ip = "10.0.0.1";
                    prefix = 24;
                  }
                ];
              };
            }
          ];
        };
      };

      # LXC container management
      lxc = {
        enable = true;
        config = {
          containers = [
            {
              name = "web-frontend";
              state = "running";
              config = {
                image = "ubuntu/22.04";
                network = {
                  type = "veth";
                  bridge = "mesh";
                  ipv4 = "10.0.0.10/24";
                };
                resources = {
                  memory = "2G";
                  cpus = "2";
                };
              };
            }
            {
              name = "api-backend";
              state = "running";
              config = {
                image = "ubuntu/22.04";
                network = {
                  type = "veth";
                  bridge = "mesh";
                  ipv4 = "10.0.0.20/24";
                };
                resources = {
                  memory = "4G";
                  cpus = "4";
                };
              };
            }
          ];
        };
      };

      # OpenFlow controller
      openflow = {
        enable = true;
        config = {
          bridges = [
            {
              name = "mesh";
              controller = "tcp:127.0.0.1:6653";
              protocols = [ "OpenFlow13" "OpenFlow14" ];
            }
          ];
          flows = [
            {
              bridge = "mesh";
              table = 0;
              priority = 1000;
              match = {
                in_port = 1;
                dl_type = "0x0800";
              };
              actions = [ "output:2" ];
            }
          ];
          policies = {
            container_isolation = {
              enabled = true;
              default_action = "allow";
            };
            rate_limiting = {
              enabled = true;
              max_rate = "100mbps";
            };
          };
        };
      };

      # User session management
      login1.enable = true;

      # DNS resolver
      dnsresolver = {
        enable = true;
        config = {
          dns_servers = [ "1.1.1.1" "8.8.8.8" ];
          domains = [ "internal.example.com" ];
        };
      };
    };

    # Advanced blockchain configuration
    blockchain = {
      enable = true;
      useML = true;  # Enable ML-based vectorization
    };

    # Cache configuration with larger size
    cache = {
      enable = true;
      maxSize = "50G";
    };

    # Data directories
    dataDir = "/var/lib/op-dbus";
    cacheDir = "/var/cache/op-dbus";

    # Detailed logging
    logLevel = "debug";
  };

  # Full MCP server configuration
  services.operation-dbus.mcp = {
    enable = true;

    # Enable all agents
    agents = [
      "executor"
      "systemd"
      "file"
      "network"
      "monitor"
    ];

    # Enable web bridge for HTTP/WebSocket access
    enableWebBridge = true;
    serverPort = 3000;

    # Enable D-Bus discovery
    enableDiscovery = true;
    discoveryConfig = {
      scan_interval = 300;  # 5 minutes
      default_format = "xml";
      output_dir = "/var/lib/op-dbus/mcp-discovery";
    };

    # Orchestrator configuration
    orchestrator = {
      enable = true;
      maxConcurrentTasks = 50;
    };

    # Tool configuration
    tools = {
      maxToolCount = 200;
      blocklist = [
        "org.freedesktop.DBus"
        "org.freedesktop.secrets"
        "org.freedesktop.impl.portal.*"
      ];
    };

    # Interactive chat interface
    chat = {
      enable = true;
      port = 8080;
    };

    logLevel = "info";
  };

  # Open necessary firewall ports
  networking.firewall = {
    allowedTCPPorts = [
      3000  # MCP web bridge
      6653  # OpenFlow controller
      8080  # Chat interface
    ];
  };

  # Additional system packages
  environment.systemPackages = with pkgs; [
    # Network tools
    iproute2
    bridge-utils

    # Container tools
    lxc

    # Debugging
    tcpdump
    wireshark-cli
  ];

  # Enable required virtualizations
  virtualisation = {
    openvswitch.enable = true;
    lxc.enable = true;
  };
}

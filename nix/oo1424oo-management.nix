# oo1424oo Management Configuration
# Powerful: 32GB RAM, Multi-core
# op-dbus management + distributed services

{ config, pkgs, ... }:

{
  imports = [
    ./hardware-configuration.nix
    ./module.nix
  ];

  # Boot
  boot.loader.grub = {
    enable = true;
    device = "/dev/sda";
  };

  # Network
  networking = {
    hostName = "oo1424oo";
    firewall = {
      enable = true;
      allowedUDPPorts = [ 51821 ];  # Netmaker
      allowedTCPPorts = [ 22 8006 ];  # SSH, Proxmox
      trustedInterfaces = [ "socket" "nm-privacy" ];
    };
  };

  # Proxmox/LXC support
  virtualisation.lxc = {
    enable = true;
    lxcfs.enable = true;
  };

  # op-dbus management plane
  services.op-dbus = {
    enable = true;
    mode = "full";

    stateConfig = {
      # Netmaker client - connects to VPS
      netmaker = {
        mode = "client";
        network = "privacy-mesh";
        interface = "nm-privacy";
        bridge = "socket";  # ONE bridge
        server = "https://<vps-public-ip>:8081";  # Replace with actual VPS IP
      };

      # OpenFlow rules for management
      openflow = {
        # Enable dynamic routing for distributed services
        dynamic_routing = {
          enabled = true;
          service_discovery = true;
          auto_flows = true;
        };

        bridges = {
          socket = {
            flows = [
              # Traffic from netmaker (VPS or other nodes)
              "priority=100,in_port=nm-privacy,actions=normal"

              # MCP server traffic
              "priority=90,tcp,tp_dst=9573,actions=output:veth200"

              # op-dbus API traffic
              "priority=90,tcp,tp_dst=9574,actions=output:veth201"

              # Vector DB traffic
              "priority=90,tcp,tp_dst=6333,actions=output:veth202"

              # Redis traffic
              "priority=90,tcp,tp_dst=6379,actions=output:veth203"

              # Container responses
              "priority=80,in_port=veth200,actions=normal"
              "priority=80,in_port=veth201,actions=normal"
              "priority=80,in_port=veth202,actions=normal"
              "priority=80,in_port=veth203,actions=normal"

              # Default: normal switching
              "priority=10,actions=normal"
            ];
          };
        };
      };

      # Management containers (scalable on powerful hardware)
      lxc = {
        containers = [
          # MCP Server - Model Context Protocol
          {
            id = "200";
            veth = "veth200";
            bridge = "socket";
            running = true;
            properties = {
              name = "mcp-server";
              network_type = "veth";
              ipv4_address = "10.0.0.200/24";
              gateway = "10.0.0.1";
              template = "alpine-3.19";
              startup = "order=1";
              memory = 1024;  # More resources available on oo1424oo
              swap = 512;
              services = [
                { name = "mcp"; protocol = "tcp"; port = 9573; exposed = true; }
              ];
            };
          }

          # op-dbus API - Control plane
          {
            id = "201";
            veth = "veth201";
            bridge = "socket";
            running = true;
            properties = {
              name = "op-dbus-api";
              network_type = "veth";
              ipv4_address = "10.0.0.201/24";
              gateway = "10.0.0.1";
              template = "alpine-3.19";
              startup = "order=1";
              memory = 1024;
              swap = 512;
              services = [
                { name = "api"; protocol = "tcp"; port = 9574; exposed = true; }
              ];
            };
          }

          # Vector DB - Heavy computation
          {
            id = "202";
            veth = "veth202";
            bridge = "socket";
            running = true;
            properties = {
              name = "vector-db";
              network_type = "veth";
              ipv4_address = "10.0.0.202/24";
              gateway = "10.0.0.1";
              template = "debian-12";
              startup = "order=2";
              memory = 4096;  # Vector DB needs significant memory
              swap = 2048;
              services = [
                { name = "vector-db"; protocol = "tcp"; port = 6333; exposed = true; }
              ];
            };
          }

          # Redis - Cache/state
          {
            id = "203";
            veth = "veth203";
            bridge = "socket";
            running = true;
            properties = {
              name = "redis";
              network_type = "veth";
              ipv4_address = "10.0.0.203/24";
              gateway = "10.0.0.1";
              template = "alpine-3.19";
              startup = "order=2";
              memory = 2048;
              swap = 1024;
              services = [
                { name = "redis"; protocol = "tcp"; port = 6379; exposed = true; }
              ];
            };
          }

          # Additional distributed services can be added here
          # oo1424oo has 32GB RAM and multi-core - plenty of resources
        ];
      };

      # System services
      systemd = {
        units = {
          "openvswitch.service" = {
            enabled = true;
            active_state = "active";
          };
          "netclient.service" = {
            enabled = true;
            active_state = "active";
          };
        };
      };

      # Required packages
      packagekit = {
        packages = {
          "lxc" = { ensure = "installed"; };
          "wireguard-tools" = { ensure = "installed"; };
          "netclient" = { ensure = "installed"; };
          "curl" = { ensure = "installed"; };
        };
      };
    };
  };

  # SSH
  services.openssh = {
    enable = true;
    settings = {
      PermitRootLogin = "prohibit-password";
      PasswordAuthentication = false;
    };
  };

  # Essential packages
  environment.systemPackages = with pkgs; [
    vim
    git
    htop
    tmux
    curl
    wget
    wireguard-tools
    iotop
    ncdu
  ];

  system.stateVersion = "25.05";
}

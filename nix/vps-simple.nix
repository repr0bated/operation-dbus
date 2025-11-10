# VPS Simple Socket Network Configuration
# ONE socket bridge with management + ingress containers
# Netmaker server provides L3 routing to oo1424oo

{ config, pkgs, ... }:

{
  imports = [
    ./hardware-configuration.nix
    ./module.nix
  ];

  # Boot
  boot.loader.grub = {
    enable = true;
    device = "/dev/vda";  # DigitalOcean virtual disk
  };

  # Network
  networking = {
    hostName = "vps-gateway";
    firewall = {
      enable = true;
      allowedTCPPorts = [
        22        # SSH
        443       # Xray VLESS
        8443      # Xray VMess
        8081      # Netmaker API
        9573      # MCP
        9574      # op-dbus API
      ];
      allowedUDPPorts = [
        51821     # Netmaker WireGuard
      ];
      trustedInterfaces = [ "socket" "nm-server" ];
    };
  };

  # op-dbus with socket network
  services.op-dbus = {
    enable = true;
    mode = "standalone";

    stateConfig = {
      # Netmaker server - provides L3 routing to oo1424oo
      netmaker = {
        mode = "server";
        network = "privacy-mesh";
        interface = "nm-server";
        bridge = "socket";  # ONE bridge (arbitrary name)
        listen_port = 51821;
        api_endpoint = "https://<vps-public-ip>:8081";  # Replace with actual IP
      };

      # OpenFlow rules for socket network
      openflow = {
        bridges = {
          socket = {
            flows = [
              # Ingress traffic to Xray server (privacy tunnel ingress)
              "priority=100,tcp,tp_dst=443,actions=output:veth202"
              "priority=100,tcp,tp_dst=8443,actions=output:veth202"

              # Xray server → netmaker (forward to oo1424oo)
              "priority=100,in_port=veth202,actions=output:nm-server"

              # MCP traffic
              "priority=90,tcp,tp_dst=9573,actions=output:veth200"

              # op-dbus API traffic
              "priority=90,tcp,tp_dst=9574,actions=output:veth201"

              # Container responses
              "priority=80,in_port=veth200,actions=normal"
              "priority=80,in_port=veth201,actions=normal"
              "priority=80,in_port=veth202,actions=normal"

              # Traffic from netmaker (from oo1424oo or other nodes)
              "priority=80,in_port=nm-server,actions=normal"

              # Default: normal switching for socket network
              "priority=10,actions=normal"
            ];
          };
        };
      };

      # Management + ingress containers on socket network
      lxc = {
        containers = [
          # MCP Server
          {
            id = "200";
            veth = "veth200";
            bridge = "socket";
            running = true;
            properties = {
              name = "mcp-server";
              network_type = "veth";
              ipv4_address = "10.1.0.200/24";
              gateway = "10.1.0.1";
              template = "alpine-3.19";
              startup = "order=1";
              memory = 512;
              swap = 256;
              services = [
                { name = "mcp"; protocol = "tcp"; port = 9573; exposed = true; }
              ];
            };
          }

          # op-dbus API
          {
            id = "201";
            veth = "veth201";
            bridge = "socket";
            running = true;
            properties = {
              name = "op-dbus-api";
              network_type = "veth";
              ipv4_address = "10.1.0.201/24";
              gateway = "10.1.0.1";
              template = "alpine-3.19";
              startup = "order=1";
              memory = 512;
              swap = 256;
              services = [
                { name = "api"; protocol = "tcp"; port = 9574; exposed = true; }
              ];
            };
          }

          # Xray Server - Privacy tunnel ingress
          {
            id = "202";
            veth = "veth202";
            bridge = "socket";
            running = true;
            properties = {
              name = "xray-server";
              network_type = "veth";
              ipv4_address = "10.1.0.202/24";
              gateway = "10.1.0.1";
              template = "alpine-3.19";
              startup = "order=1";
              memory = 512;
              swap = 256;
              services = [
                {
                  name = "xray-vless";
                  protocol = "tcp";
                  port = 443;
                  exposed = true;
                  description = "Privacy tunnel ingress, forwards to oo1424oo";
                }
                {
                  name = "xray-vmess";
                  protocol = "tcp";
                  port = 8443;
                  exposed = true;
                }
              ];
            };
          }
        ];
      };

      # System services
      systemd = {
        units = {
          "openvswitch.service" = {
            enabled = true;
            active_state = "active";
          };
          "netmaker.service" = {
            enabled = true;
            active_state = "active";
          };
        };
      };

      # Required packages
      packagekit = {
        packages = {
          "wireguard-tools" = { ensure = "installed"; };
          "netmaker" = { ensure = "installed"; };
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
    curl
    wget
    wireguard-tools
    netmaker
  ];

  system.stateVersion = "25.05";
}

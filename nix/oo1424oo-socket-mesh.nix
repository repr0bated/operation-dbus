# oo1424oo Socket Network Configuration (Correct Architecture)
# Two OVS bridges:
# 1. mesh bridge - Isolation layer with nm-privacy (Netmaker) enslaved
# 2. vmbr0 - Socket network for privacy router + distributed services

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
      allowedUDPPorts = [ 51820 51821 ];  # Gateway VPN, Netmaker
      allowedTCPPorts = [ 22 8006 9573 9574 ];  # SSH, Proxmox, MCP, API
      trustedInterfaces = [ "vmbr0" "mesh" "nm-privacy" ];
    };
  };

  # Proxmox/LXC support
  virtualisation.lxc = {
    enable = true;
    lxcfs.enable = true;
  };

  # op-dbus socket network configuration
  services.op-dbus = {
    enable = true;
    mode = "full";

    stateConfig = {
      # Netmaker client on HOST (tun device enslaved by mesh bridge)
      netmaker = {
        mode = "client";
        network = "privacy-mesh";
        interface = "nm-privacy";  # tun device, NOT veth
        bridge = "mesh";  # Enslaved by mesh bridge
        server = "https://netmaker-gateway:8081";
      };

      # Inter-bridge connection (veth pair)
      # Connects mesh isolation layer to socket network
      networking = {
        veth_pairs = {
          mesh_to_socket = {
            peer1 = {
              name = "to-socket";
              bridge = "mesh";
            };
            peer2 = {
              name = "from-mesh";
              bridge = "vmbr0";
            };
          };
        };
      };

      # WARP tunnel on host (privacy exit)
      wireguard = {
        warp_tunnel = {
          interface = "warp0";
          enabled = true;
          # PostUp hook adds warp0 to socket network bridge
          post_up = "ovs-vsctl add-port vmbr0 warp0";
          pre_down = "ovs-vsctl del-port vmbr0 warp0";
        };

        # VPN server for authenticated clients
        vpn_server = {
          interface = "wg0";
          listen_port = 51820;
          ip_pool = "10.8.0.0/24";
          auto_provision = true;
          peers = [];
        };
      };

      # OpenFlow rules - TWO bridges
      openflow = {
        bridges = {
          # Mesh bridge - Isolation layer
          # nm-privacy (Netmaker tun) enslaved as port
          mesh = {
            flows = [
              # Netmaker traffic → socket network
              "priority=100,in_port=nm-privacy,actions=output:to-socket"

              # Socket network → Netmaker
              "priority=100,in_port=to-socket,actions=output:nm-privacy"

              # Default: DROP (strict isolation)
              "priority=1,actions=drop"
            ];
          };

          # Socket network bridge (vmbr0)
          # Handles BOTH privacy router AND distributed services
          vmbr0 = {
            flows = [
              # Privacy router traffic from mesh → WARP exit
              "priority=100,in_port=from-mesh,actions=output:warp0"

              # Gateway container → WARP
              "priority=100,in_port=veth100,actions=output:warp0"

              # Xray proxy → WARP
              "priority=100,in_port=veth102,actions=output:warp0"

              # WARP return traffic (MAC learning)
              "priority=100,in_port=warp0,actions=learn(table=1,hard_timeout=300,priority=110,NXM_OF_ETH_DST[]=NXM_OF_ETH_SRC[],output:NXM_OF_IN_PORT[]),output:normal"

              # Distributed services (MCP, API, etc.)
              # Can communicate locally or cross-host via mesh
              "priority=80,in_port=veth200,actions=normal"
              "priority=80,in_port=veth201,actions=normal"

              # Default: normal switching
              "priority=10,actions=normal"
            ];
          };
        };
      };

      # Containers on socket network (vmbr0)
      lxc = {
        containers = [
          # ══════════════════════════════════════════════════════
          # Privacy Router Containers (100-199)
          # ══════════════════════════════════════════════════════

          # Gateway - WireGuard VPN server
          {
            id = "100";
            veth = "veth100";
            bridge = "vmbr0";
            running = true;
            properties = {
              name = "gateway";
              network_type = "veth";
              ipv4_address = "10.0.0.100/24";
              gateway = "10.0.0.1";
              template = "ubuntu-22.04";
              startup = "order=1";
              memory = 512;
              swap = 512;
              features = {
                nesting = true;
              };
              wireguard = {
                enabled = true;
                interface = "wg0";
                listen_port = 51820;
                ip_pool = "10.8.0.0/24";
                dns = "1.1.1.1";
                auto_provision = true;
              };
            };
          }

          # Xray - V2Ray/Xray proxy server
          {
            id = "102";
            veth = "veth102";
            bridge = "vmbr0";
            running = true;
            properties = {
              name = "xray";
              network_type = "veth";
              ipv4_address = "10.0.0.102/24";
              gateway = "10.0.0.1";
              template = "alpine-3.19";
              startup = "order=2,up=30";
              memory = 512;
              swap = 256;
              services = [
                { name = "xray-vless"; protocol = "tcp"; port = 443; exposed = true; }
                { name = "xray-vmess"; protocol = "tcp"; port = 8443; exposed = true; }
              ];
            };
          }

          # ══════════════════════════════════════════════════════
          # Distributed Service Containers (200-299)
          # Part of multi-server socket network
          # ══════════════════════════════════════════════════════

          # MCP Server - Model Context Protocol
          {
            id = "200";
            veth = "veth200";
            bridge = "vmbr0";
            running = true;
            properties = {
              name = "mcp-server";
              network_type = "veth";
              ipv4_address = "10.0.0.200/24";
              gateway = "10.0.0.1";
              template = "alpine-3.19";
              startup = "order=1";
              memory = 512;
              swap = 256;
              services = [
                { name = "mcp"; protocol = "tcp"; port = 9573; exposed = true; }
              ];
            };
          }

          # op-dbus API - Control plane
          {
            id = "201";
            veth = "veth201";
            bridge = "vmbr0";
            running = true;
            properties = {
              name = "op-dbus-api";
              network_type = "veth";
              ipv4_address = "10.0.0.201/24";
              gateway = "10.0.0.1";
              template = "alpine-3.19";
              startup = "order=1";
              memory = 512;
              swap = 256;
              services = [
                { name = "api"; protocol = "tcp"; port = 9573; exposed = true; }
                { name = "api-alt"; protocol = "tcp"; port = 9574; exposed = true; }
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
          "wg-quick@wg0.service" = {
            enabled = true;
            active_state = "active";
          };
          "wg-quick@warp0.service" = {
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
  ];

  system.stateVersion = "25.05";
}

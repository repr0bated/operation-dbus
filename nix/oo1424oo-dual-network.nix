# oo1424oo Dual Socket Network Configuration
# Two isolated socket networks:
# 1. Privacy network (vmbr0) - User traffic through WARP
# 2. Management network (vmbr1) - MCP, op-dbus API, monitoring

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
      allowedUDPPorts = [ 51820 51821 51822 ];  # Gateway, Netmaker privacy, Netmaker mgmt
      allowedTCPPorts = [ 22 8006 9573 9574 ];  # SSH, Proxmox, MCP, API

      # Trust management network only
      trustedInterfaces = [ "vmbr1" "mesh-mgmt" "nm-mgmt" ];

      # Privacy network is NOT trusted by host
      # Traffic must go through OpenFlow rules
    };
  };

  # Proxmox/LXC support
  virtualisation.lxc = {
    enable = true;
    lxcfs.enable = true;
  };

  # op-dbus with dual socket networks
  services.op-dbus = {
    enable = true;
    mode = "full";

    stateConfig = {
      # Netmaker - TWO separate mesh networks
      netmaker = {
        networks = [
          {
            name = "privacy-mesh";
            mode = "client";
            interface = "nm-privacy";
            bridge = "mesh-privacy";
            server = "https://netmaker-gateway:8081";
            # Privacy network for user traffic routing
          }
          {
            name = "management-mesh";
            mode = "client";
            interface = "nm-mgmt";
            bridge = "mesh-mgmt";
            server = "https://netmaker-gateway:8082";
            # Management network for control plane
          }
        ];
      };

      # Inter-bridge veth pairs (connects mesh to socket networks)
      networking = {
        veth_pairs = {
          privacy_mesh_to_socket = {
            peer1 = { name = "to-privacy"; bridge = "mesh-privacy"; };
            peer2 = { name = "from-privacy"; bridge = "vmbr0"; };
          };
          mgmt_mesh_to_socket = {
            peer1 = { name = "to-mgmt"; bridge = "mesh-mgmt"; };
            peer2 = { name = "from-mgmt"; bridge = "vmbr1"; };
          };
        };
      };

      # WARP tunnel on host (privacy exit)
      wireguard = {
        warp_tunnel = {
          interface = "warp0";
          enabled = true;
          # PostUp hook adds warp0 to privacy network bridge
          post_up = "ovs-vsctl add-port vmbr0 warp0";
          pre_down = "ovs-vsctl del-port vmbr0 warp0";
        };

        # VPN server for authenticated clients
        vpn_server = {
          interface = "wg0";
          listen_port = 51820;
          ip_pool = "10.8.0.0/24";
          auto_provision = true;
          peers = [];  # Dynamically added
        };
      };

      # OpenFlow rules - Network isolation and routing
      openflow = {
        # Enable dynamic routing for both networks
        dynamic_routing = {
          enabled = true;
          service_discovery = true;
          auto_flows = true;
        };

        bridges = {
          # Privacy mesh bridge - Isolation layer
          "mesh-privacy" = {
            flows = [
              # Netmaker privacy → socket network
              "priority=100,in_port=nm-privacy,actions=output:to-privacy"

              # Socket network → Netmaker privacy
              "priority=100,in_port=to-privacy,actions=output:nm-privacy"

              # Default: DROP (strict isolation)
              "priority=1,actions=drop"
            ];
          };

          # Management mesh bridge - Isolation layer
          "mesh-mgmt" = {
            flows = [
              # Netmaker mgmt → socket network
              "priority=100,in_port=nm-mgmt,actions=output:to-mgmt"

              # Socket network → Netmaker mgmt
              "priority=100,in_port=to-mgmt,actions=output:nm-mgmt"

              # Default: DROP (strict isolation)
              "priority=1,actions=drop"
            ];
          };

          # Privacy socket network (vmbr0)
          vmbr0 = {
            flows = [
              # VPN clients → Gateway container
              "priority=100,udp,tp_dst=51820,actions=output:veth100"

              # Gateway → WARP tunnel
              "priority=100,in_port=veth100,actions=output:warp0"

              # Xray proxy → WARP tunnel
              "priority=100,in_port=veth102,actions=output:warp0"

              # WARP → return traffic (MAC learning)
              "priority=100,in_port=warp0,actions=learn(table=1,hard_timeout=300,priority=110,NXM_OF_ETH_DST[]=NXM_OF_ETH_SRC[],output:NXM_OF_IN_PORT[]),output:normal"

              # Cross-node privacy traffic via mesh
              "priority=90,in_port=from-privacy,actions=output:warp0"

              # NO access to management network (enforced by lack of flows)

              # Default: normal switching for local traffic
              "priority=10,actions=normal"
            ];
          };

          # Management socket network (vmbr1)
          vmbr1 = {
            flows = [
              # MCP → op-dbus API
              "priority=100,tcp,tp_dst=9573,actions=output:veth201"
              "priority=100,tcp,tp_dst=9574,actions=output:veth201"

              # Cross-node management traffic via mesh
              "priority=90,in_port=from-mgmt,actions=normal"

              # NO access to privacy network (enforced by lack of flows)

              # Default: normal switching for local traffic
              "priority=10,actions=normal"
            ];
          };
        };
      };

      # Containers assigned to networks
      lxc = {
        # Container template for dynamic creation
        container_template = {
          template = "alpine-3.19";
          memory = 512;
          swap = 256;
          features = {
            nesting = false;
          };
        };

        containers = [
          # ══════════════════════════════════════════════════════
          # Privacy Network Containers (vmbr0)
          # Container IDs: 100-199
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
              network = "privacy";
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
              network = "privacy";
              services = [
                { name = "xray-vless"; protocol = "tcp"; port = 443; exposed = true; }
                { name = "xray-vmess"; protocol = "tcp"; port = 8443; exposed = true; }
              ];
            };
          }

          # ══════════════════════════════════════════════════════
          # Management Network Containers (vmbr1)
          # Container IDs: 200-299
          # ══════════════════════════════════════════════════════

          # MCP Server - Model Context Protocol server
          {
            id = "200";
            veth = "veth200";
            bridge = "vmbr1";
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
              network = "management";
              services = [
                { name = "mcp"; protocol = "tcp"; port = 9573; exposed = true; }
              ];
            };
          }

          # op-dbus API - Control plane API
          {
            id = "201";
            veth = "veth201";
            bridge = "vmbr1";
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
              network = "management";
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
        };
      };

      # Required packages
      packagekit = {
        packages = {
          "lxc" = { ensure = "installed"; };
          "wireguard-tools" = { ensure = "installed"; };
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

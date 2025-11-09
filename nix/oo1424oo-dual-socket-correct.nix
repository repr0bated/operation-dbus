# oo1424oo Correct Dual Socket Network Configuration
#
# Architecture:
# 1. mesh bridge - Receives ALL traffic, has nm-privacy enslaved
#    - Forwards privacy traffic to isolated vmbr0
#    - Routes other traffic via netmaker
#    - Hosts management containers (MCP, API, etc.)
#
# 2. vmbr0 bridge - ISOLATED privacy network (NO netmaker)
#    - Receives privacy traffic from mesh
#    - Routes via WARP tunnel over traditional internet
#    - Exits to VPS xray server (non-netmaker)

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

      # Trust mesh bridge (has netmaker)
      trustedInterfaces = [ "mesh" "nm-privacy" ];

      # vmbr0 is NOT trusted (isolated privacy network)
    };
  };

  # Proxmox/LXC support
  virtualisation.lxc = {
    enable = true;
    lxcfs.enable = true;
  };

  # op-dbus dual socket network configuration
  services.op-dbus = {
    enable = true;
    mode = "full";

    stateConfig = {
      # Netmaker on host, enslaved by mesh bridge
      netmaker = {
        mode = "client";
        network = "privacy-mesh";
        interface = "nm-privacy";  # tun device
        bridge = "mesh";  # Enslaved by mesh bridge
        server = "https://netmaker-gateway:8081";
      };

      # Inter-bridge connection: mesh → vmbr0 (privacy isolation)
      networking = {
        veth_pairs = {
          mesh_to_privacy = {
            peer1 = {
              name = "to-privacy";
              bridge = "mesh";
            };
            peer2 = {
              name = "from-mesh";
              bridge = "vmbr0";
            };
          };
        };
      };

      # WARP tunnel on host (privacy exit via traditional internet)
      wireguard = {
        warp_tunnel = {
          interface = "warp0";
          enabled = true;
          # PostUp adds to ISOLATED vmbr0 (not mesh)
          post_up = "ovs-vsctl add-port vmbr0 warp0";
          pre_down = "ovs-vsctl del-port vmbr0 warp0";
        };

        # VPN server (if needed)
        vpn_server = {
          interface = "wg0";
          listen_port = 51820;
          ip_pool = "10.8.0.0/24";
          auto_provision = true;
          peers = [];
        };
      };

      # OpenFlow - TWO bridges with different purposes
      openflow = {
        bridges = {
          # mesh bridge - Receives ALL traffic, routes appropriately
          mesh = {
            flows = [
              # Privacy traffic from netmaker → isolated vmbr0
              # Forward to privacy network for WARP exit
              "priority=100,in_port=nm-privacy,actions=output:to-privacy"

              # Management containers → netmaker
              "priority=90,in_port=veth200,actions=output:nm-privacy"
              "priority=90,in_port=veth201,actions=output:nm-privacy"

              # Return traffic from netmaker → management containers
              "priority=90,in_port=nm-privacy,actions=normal"

              # Return traffic from privacy network → netmaker
              "priority=80,in_port=to-privacy,actions=output:nm-privacy"

              # Default: normal switching for local management traffic
              "priority=10,actions=normal"
            ];
          };

          # vmbr0 - ISOLATED privacy network (NO netmaker access)
          vmbr0 = {
            flows = [
              # Privacy traffic from mesh → xray client
              "priority=100,in_port=from-mesh,actions=output:veth102"

              # Xray client → WARP tunnel (traditional internet, non-netmaker)
              "priority=100,in_port=veth102,actions=output:warp0"

              # WARP return → xray client
              "priority=100,in_port=warp0,actions=output:veth102"

              # Gateway (if needed) → WARP
              "priority=90,in_port=veth100,actions=output:warp0"
              "priority=90,in_port=warp0,actions=output:veth100"

              # Default: normal switching
              # (but isolated, cannot reach mesh or netmaker)
              "priority=10,actions=normal"
            ];
          };
        };
      };

      # Containers on DIFFERENT bridges
      lxc = {
        containers = [
          # ══════════════════════════════════════════════════════
          # Privacy Network Containers (vmbr0 - ISOLATED)
          # NO netmaker access, exit via WARP to traditional internet
          # ══════════════════════════════════════════════════════

          # Gateway container
          {
            id = "100";
            veth = "veth100";
            bridge = "vmbr0";  # Isolated bridge
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

          # Xray client - Receives privacy traffic, exits via WARP
          {
            id = "102";
            veth = "veth102";
            bridge = "vmbr0";  # Isolated bridge
            running = true;
            properties = {
              name = "xray-client";
              network_type = "veth";
              ipv4_address = "10.0.0.102/24";
              gateway = "10.0.0.1";
              template = "alpine-3.19";
              startup = "order=2";
              memory = 512;
              swap = 256;
              services = [
                {
                  name = "xray-client";
                  protocol = "tcp";
                  port = 8080;
                  # Connects to VPS xray server via WARP-cloaked traditional internet
                }
              ];
            };
          }

          # ══════════════════════════════════════════════════════
          # Management/Distributed Network Containers (mesh bridge)
          # Uses netmaker for cross-host communication
          # ══════════════════════════════════════════════════════

          # MCP Server
          {
            id = "200";
            veth = "veth200";
            bridge = "mesh";  # Connected to netmaker
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
            bridge = "mesh";  # Connected to netmaker
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

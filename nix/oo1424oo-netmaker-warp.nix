# oo1424oo Configuration - Netmaker client + WARP gateway
# Receives traffic from DO via Netmaker mesh
# Routes through socket network (vmbr0) and WARP tunnel

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
      allowedTCPPorts = [ 22 8006 9573 9574 ];  # SSH, Proxmox, MCP
      allowedUDPPorts = [ 51821 ];  # Netmaker WireGuard
      trustedInterfaces = [ "vmbr0" "mesh" "nm-privacy" "warp0" ];
    };
  };

  # Proxmox/LXC support
  virtualisation.lxc = {
    enable = true;
    lxcfs.enable = true;
  };

  # op-dbus privacy gateway
  services.op-dbus = {
    enable = true;
    mode = "full";

    stateConfig = {
      # Netmaker client - connects to DO droplet
      netmaker = {
        mode = "client";
        network = "privacy-mesh";
        interface = "nm-privacy";
        server = "https://netmaker-gateway:8081";
        # nm-privacy attaches to mesh bridge (isolated)
        bridge = "mesh";
      };

      # Inter-bridge connection (veth pair)
      # Connects mesh bridge to vmbr0 socket network
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

      # WARP tunnel (on host) - privacy exit
      warp = {
        interface = "warp0";
        enabled = true;
        # PostUp hook adds warp0 to vmbr0
        post_up = "ovs-vsctl add-port vmbr0 warp0";
        pre_down = "ovs-vsctl del-port vmbr0 warp0";
      };

      # Bridge isolation - Separate mesh from socket network
      # mesh bridge: Netmaker only
      # vmbr0 bridge: Socket networking (containers + warp0)
      # Inter-bridge connection: veth pair (veth-mesh-to-socket)

      # OpenFlow rules
      openflow = {
        bridges = {
          # Mesh bridge - Netmaker WireGuard mesh ONLY
          mesh = {
            flows = [
              # Netmaker traffic → inter-bridge port
              "priority=100,in_port=nm-privacy,actions=output:to-socket"

              # Return traffic from socket network → Netmaker
              "priority=100,in_port=to-socket,actions=output:nm-privacy"

              # Default: drop (isolation)
              "priority=1,actions=drop"
            ];
          };

          # Socket network bridge - Containers + WARP
          vmbr0 = {
            flows = [
              # Traffic from mesh → route through warp0
              "priority=100,in_port=from-mesh,actions=output:warp0"

              # WARP return → back to mesh
              "priority=100,in_port=warp0,dl_dst=<mesh-mac>,actions=output:from-mesh"

              # Container traffic → route through warp0
              "priority=90,in_port=veth102,actions=output:warp0"
              "priority=90,in_port=warp0,dl_dst=<container-mac>,actions=output:veth102"

              # Default: normal switching for local traffic
              "priority=10,actions=normal"
            ];
          };
        };
      };

      # Optional: Xray container for additional proxy
      lxc = {
        containers = [
          {
            id = "102";
            veth = "veth102";
            bridge = "vmbr0";
            running = true;
            properties = {
              name = "xray";
              ipv4_address = "10.0.0.102/24";
              gateway = "10.0.0.1";
              template = "alpine-3.19";
              memory = 512;
              swap = 256;
            };
          }
        ];
      };

      # System services
      systemd = {
        units = {
          "netclient.service" = {
            enabled = true;
            active_state = "active";
          };
          "wg-quick@warp0.service" = {
            enabled = true;
            active_state = "active";
          };
        };
      };

      # Packages
      packagekit = {
        packages = {
          "netclient" = { ensure = "installed"; };
          "wireguard-tools" = { ensure = "installed"; };
          "lxc" = { ensure = "installed"; };
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

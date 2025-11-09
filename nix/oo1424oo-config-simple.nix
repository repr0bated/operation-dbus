# Ready-to-deploy NixOS configuration for oo1424oo
# Simplified privacy router: Gateway on host + Xray container
# Gateway creates WireGuard VPN + WARP tunnel (both added to bridge)

{ config, pkgs, ... }:

{
  imports = [
    ./hardware-configuration.nix
    ./module.nix
  ];

  # Boot
  boot.loader.grub = {
    enable = true;
    device = "/dev/sda";  # Adjust based on your disk
  };

  # Network - Use existing Proxmox configuration
  networking = {
    hostName = "oo1424oo";
    firewall = {
      enable = true;
      allowedUDPPorts = [ 51820 ];  # WireGuard VPN
      allowedTCPPorts = [ 22 8006 9573 9574 443 8443 ];  # SSH, Proxmox, MCP, Xray
      trustedInterfaces = [ "vmbr0" "mesh" "wg0" "warp0" ];
    };
  };

  # Proxmox/LXC support
  virtualisation.lxc = {
    enable = true;
    lxcfs.enable = true;
  };

  # op-dbus privacy router
  services.op-dbus = {
    enable = true;
    mode = "full";

    stateConfig = {
      # Gateway on HOST creates:
      # - wg0: VPN server for authenticated clients
      # - warp0: WARP tunnel (anonymous exit) added to vmbr0
      # - nm0: Netmaker interface added to mesh bridge

      # WireGuard VPN Server (on host)
      wireguard = {
        vpn_server = {
          interface = "wg0";
          listen_port = 51820;
          ip_pool = "10.8.0.0/24";
          auto_provision = true;
          peers = [];  # Dynamically added
        };

        # WARP tunnel (on host) - anonymous WARP access
        warp_tunnel = {
          interface = "warp0";
          enabled = true;
          # PostUp hook adds warp0 to vmbr0 bridge
          post_up = "ovs-vsctl add-port vmbr0 warp0";
          pre_down = "ovs-vsctl del-port vmbr0 warp0";
        };
      };

      # Netmaker interface to mesh bridge
      netmaker = {
        interface = "nm0";
        bridge = "mesh";
      };

      # OpenFlow rules - Route Xray traffic through WARP
      openflow = {
        bridges = {
          vmbr0 = {
            flows = [
              # VPN clients → wg0 on host
              "priority=100,udp,tp_dst=51820,actions=local"

              # Xray container → warp0 (anonymous WARP exit)
              "priority=100,in_port=veth102,actions=output:warp0"

              # warp0 → Xray (return traffic)
              "priority=100,in_port=warp0,actions=output:veth102"

              # Default: normal switching
              "priority=10,actions=normal"
            ];
          };
        };
      };

      # Single container: Xray proxy
      lxc = {
        containers = [
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
              memory = 512;
              swap = 256;
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

      # Packages
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

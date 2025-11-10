# DigitalOcean Droplet Configuration
# Public SOCKS5 proxy + Netmaker server
# Routes traffic through Netmaker mesh to oo1424oo → WARP exit

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

  # Network - Use DO network config
  networking = {
    hostName = "netmaker-gateway";
    firewall = {
      enable = true;
      allowedTCPPorts = [
        22        # SSH
        1080      # SOCKS5 proxy
        8081      # Netmaker API
        8443      # Netmaker UI
      ];
      allowedUDPPorts = [
        51821     # Netmaker WireGuard
      ];
      trustedInterfaces = [ "nm-privacy" ];  # Netmaker mesh interface
    };
  };

  # op-dbus configuration for Netmaker + SOCKS5
  services.op-dbus = {
    enable = true;
    mode = "standalone";

    stateConfig = {
      # Netmaker server
      netmaker = {
        mode = "server";
        network = "privacy-mesh";
        interface = "nm-privacy";
        listen_port = 51821;
        api_endpoint = "https://netmaker-gateway:8081";
      };

      # Xray SOCKS5 proxy
      xray = {
        socks5 = {
          enabled = true;
          listen = "0.0.0.0:1080";
          auth = "noauth";  # Or add user/pass

          # Route traffic through Netmaker mesh to oo1424oo
          routing = {
            rules = [
              {
                type = "field";
                outboundTag = "netmaker-mesh";
                network = "tcp,udp";
              }
            ];
          };

          outbounds = [
            {
              tag = "netmaker-mesh";
              protocol = "freedom";
              settings = {};
              # Traffic goes through nm-privacy interface to oo1424oo
              streamSettings = {
                sockopt = {
                  mark = 100;  # Route through Netmaker
                };
              };
            }
          ];
        };
      };

      # Routing: SOCKS5 → Netmaker mesh → oo1424oo
      networking = {
        routes = {
          # Mark SOCKS5 traffic to route through Netmaker
          "0.0.0.0/0" = {
            via = "nm-privacy";
            metric = 100;
          };
        };
      };

      # System services
      systemd = {
        units = {
          "netmaker.service" = {
            enabled = true;
            active_state = "active";
          };
        };
      };

      # Packages
      packagekit = {
        packages = {
          "wireguard-tools" = { ensure = "installed"; };
          "curl" = { ensure = "installed"; };
          "netmaker" = { ensure = "installed"; };
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

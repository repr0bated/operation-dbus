# NixOS Netboot Server Configuration
# This server provides PXE boot for deploying NixOS systems over the network
#
# Services provided:
# - DHCP: Assigns IPs and points to boot server
# - TFTP: Serves iPXE bootloader
# - HTTP: Serves NixOS images (faster than TFTP)
# - operation-dbus: Manages netboot targets declaratively
#
# Usage:
#   1. Customize network settings below
#   2. Add to /etc/nixos/configuration.nix:
#      imports = [ ./netboot-server.nix ];
#   3. Run: sudo nixos-rebuild switch
#   4. Generate netboot images: sudo netboot-generate

{ config, pkgs, lib, ... }:

with lib;

let
  # Network configuration
  netbootInterface = "eth0";           # Network interface for netboot
  netbootSubnet = "10.0.0.0/24";      # Subnet for PXE clients
  netbootServerIP = "10.0.0.1";       # This server's IP
  netbootDHCPStart = "10.0.0.100";    # DHCP range start
  netbootDHCPEnd = "10.0.0.200";      # DHCP range end
  netbootGateway = "10.0.0.1";        # Gateway for clients

  # Paths
  tftpRoot = "/var/lib/netboot/tftp";
  httpRoot = "/var/lib/netboot/http";
  configRoot = "/var/lib/netboot/configs";

  # iPXE bootloader
  ipxe = pkgs.ipxe.override {
    embedScript = pkgs.writeText "embed.ipxe" ''
      #!ipxe
      dhcp
      chain http://${netbootServerIP}/boot.ipxe
    '';
  };

in {
  # Network configuration
  networking = {
    interfaces.${netbootInterface} = {
      ipv4.addresses = [{
        address = netbootServerIP;
        prefixLength = 24;
      }];
    };

    firewall = {
      enable = true;
      allowedTCPPorts = [
        80     # HTTP (NixOS images)
        443    # HTTPS (optional)
      ];
      allowedUDPPorts = [
        67     # DHCP
        69     # TFTP
        4011   # PXE/ProxyDHCP
      ];
    };
  };

  # DHCP Server (dnsmasq)
  services.dnsmasq = {
    enable = true;
    settings = {
      # Network interface
      interface = netbootInterface;
      bind-interfaces = true;

      # DHCP range
      dhcp-range = "${netbootDHCPStart},${netbootDHCPEnd},12h";

      # Gateway
      dhcp-option = [
        "3,${netbootGateway}"     # Gateway
        "6,8.8.8.8,8.8.4.4"       # DNS servers
      ];

      # PXE boot settings
      dhcp-boot = "ipxe.efi";
      enable-tftp = true;
      tftp-root = tftpRoot;

      # Log queries
      log-queries = true;
      log-dhcp = true;
    };
  };

  # HTTP Server (nginx) for serving NixOS images
  services.nginx = {
    enable = true;

    virtualHosts."netboot" = {
      listen = [{ addr = netbootServerIP; port = 80; }];

      root = httpRoot;

      locations."/" = {
        extraConfig = ''
          autoindex on;
          autoindex_exact_size off;
          autoindex_localtime on;
        '';
      };

      # Serve boot.ipxe menu
      locations."/boot.ipxe" = {
        alias = "${configRoot}/boot.ipxe";
        extraConfig = ''
          default_type text/plain;
        '';
      };
    };
  };

  # operation-dbus for managing netboot targets
  services.operation-dbus = {
    enable = true;

    stateFile = "/etc/operation-dbus/netboot.json";

    # Netboot server doesn't need NUMA optimization
    numa.enable = false;

    # BTRFS for netboot image storage
    btrfs = {
      enable = true;
      basePath = "/var/lib/netboot";
      compressionLevel = 5;  # Higher compression for image storage
    };

    # Declarative netboot targets
    defaultState = {
      version = "1.0";

      netboot = {
        targets = [
          {
            name = "proxmox-node-01";
            mac = "52:54:00:12:34:56";
            config = "proxmox-host";
            ip = "10.0.0.101";
          }
          {
            name = "proxmox-node-02";
            mac = "52:54:00:12:34:57";
            config = "proxmox-host";
            ip = "10.0.0.102";
          }
          {
            name = "worker-01";
            mac = "52:54:00:12:34:58";
            config = "workstation";
            ip = "10.0.0.103";
          }
        ];

        # Available configurations
        configs = {
          proxmox-host = {
            kernel = "bzImage-proxmox";
            initrd = "initrd-proxmox";
            params = "init=/nix/store/.../init console=ttyS0";
          };
          workstation = {
            kernel = "bzImage-workstation";
            initrd = "initrd-workstation";
            params = "init=/nix/store/.../init";
          };
        };
      };
    };
  };

  # System packages for netboot management
  environment.systemPackages = with pkgs; [
    dnsmasq      # DHCP/TFTP server
    ipxe         # iPXE bootloader
    pxelinux     # Legacy BIOS boot support
    syslinux     # Boot utilities

    # Debugging tools
    tcpdump
    wireshark-cli
    nmap
  ];

  # Systemd tmpfiles rules for directory creation
  systemd.tmpfiles.rules = [
    "d ${tftpRoot} 0755 root root -"
    "d ${httpRoot} 0755 root root -"
    "d ${configRoot} 0755 root root -"
    "d ${httpRoot}/images 0755 root root -"
    "L+ ${tftpRoot}/ipxe.efi - - - - ${ipxe}/ipxe.efi"
    "L+ ${tftpRoot}/undionly.kpxe - - - - ${ipxe}/undionly.kpxe"
  ];

  # Helper script to generate netboot images
  environment.etc."netboot/generate.sh" = {
    text = ''
      #!/usr/bin/env bash
      # Generate NixOS netboot images

      set -e

      CONFIG_NAME="''${1:-proxmox-host}"
      OUTPUT_DIR="${httpRoot}/images/$CONFIG_NAME"

      echo "Generating netboot image: $CONFIG_NAME"
      echo "Output directory: $OUTPUT_DIR"

      # Build the netboot image
      nix-build '<nixpkgs/nixos>' \
        -A config.system.build.netbootRamdisk \
        -I nixos-config=/etc/nixos/netboot-configs/$CONFIG_NAME.nix \
        -o /tmp/netboot-$CONFIG_NAME

      # Create output directory
      mkdir -p "$OUTPUT_DIR"

      # Copy kernel and initrd
      cp /tmp/netboot-$CONFIG_NAME/bzImage "$OUTPUT_DIR/bzImage"
      cp /tmp/netboot-$CONFIG_NAME/initrd "$OUTPUT_DIR/initrd"

      # Generate SHA256 checksums
      cd "$OUTPUT_DIR"
      sha256sum bzImage initrd > SHA256SUMS

      echo "✓ Netboot image generated successfully"
      echo "  Kernel: $OUTPUT_DIR/bzImage"
      echo "  Initrd: $OUTPUT_DIR/initrd"
      echo ""
      echo "Next steps:"
      echo "  1. Update boot.ipxe menu: sudo netboot-update-menu"
      echo "  2. Test boot: sudo netboot-test $CONFIG_NAME"
    '';
    mode = "0755";
  };

  # Helper script to update boot menu
  environment.etc."netboot/update-menu.sh" = {
    text = ''
      #!/usr/bin/env bash
      # Update iPXE boot menu from operation-dbus state

      set -e

      STATE_FILE="/etc/operation-dbus/netboot.json"
      MENU_FILE="${configRoot}/boot.ipxe"

      echo "Generating iPXE boot menu from $STATE_FILE"

      # Generate menu (simplified - would parse JSON in production)
      cat > "$MENU_FILE" <<'EOF'
      #!ipxe

      # NixOS Netboot Menu
      # Generated by operation-dbus

      :start
      menu NixOS Network Boot
      item --gap -- Available Configurations:
      item proxmox-host     Proxmox Host (Multi-socket Xeon)
      item workstation      Workstation (Single-socket)
      item installer        NixOS Installer (Live environment)
      item shell            iPXE Shell (Debugging)
      choose --default proxmox-host --timeout 10000 target && goto ''${target}

      :proxmox-host
      echo Booting Proxmox Host configuration...
      kernel http://${netbootServerIP}/images/proxmox-host/bzImage init=/nix/store/.../init loglevel=4 console=ttyS0
      initrd http://${netbootServerIP}/images/proxmox-host/initrd
      boot

      :workstation
      echo Booting Workstation configuration...
      kernel http://${netbootServerIP}/images/workstation/bzImage init=/nix/store/.../init
      initrd http://${netbootServerIP}/images/workstation/initrd
      boot

      :installer
      echo Booting NixOS installer...
      kernel http://${netbootServerIP}/images/installer/bzImage init=/nix/store/.../init
      initrd http://${netbootServerIP}/images/installer/initrd
      boot

      :shell
      echo Entering iPXE shell...
      shell

      EOF

      echo "✓ Boot menu updated: $MENU_FILE"
    '';
    mode = "0755";
  };

  # Create convenient aliases
  environment.shellAliases = {
    netboot-generate = "sudo /etc/netboot/generate.sh";
    netboot-update-menu = "sudo /etc/netboot/update-menu.sh";
    netboot-status = "sudo systemctl status dnsmasq nginx";
    netboot-logs = "sudo journalctl -u dnsmasq -u nginx -f";
  };
}

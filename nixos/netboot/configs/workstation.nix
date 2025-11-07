# Netboot Configuration: Workstation
# Single-socket system with CPU-only ML
#
# Build netboot image:
#   nix-build '<nixpkgs/nixos>' -A config.system.build.netbootRamdisk \
#     -I nixos-config=./workstation.nix

{ config, pkgs, lib, modulesPath, ... }:

{
  imports = [
    (modulesPath + "/installer/netboot/netboot-minimal.nix")
    ../../modules/operation-dbus.nix
  ];

  # Netboot settings
  boot = {
    supportedFilesystems = [ "btrfs" ];
    kernelParams = [
      "boot.shell_on_fail"
      "console=tty0"
    ];
    loader.grub.enable = false;
  };

  # Networking
  networking = {
    hostName = "workstation-netboot";
    useDHCP = true;
    firewall.enable = true;
    firewall.allowedTCPPorts = [ 22 ];
  };

  # operation-dbus (minimal configuration)
  services.operation-dbus = {
    enable = true;
    stateFile = "/etc/operation-dbus/state.json";

    # No NUMA (single socket)
    numa.enable = false;

    # BTRFS with less aggressive compression
    btrfs = {
      enable = true;
      compressionLevel = 3;
      snapshotRetention = 12;
    };

    # CPU-only ML
    ml = {
      enable = true;
      executionProvider = "cpu";
      numThreads = 4;
    };
  };

  # SSH
  services.openssh = {
    enable = true;
    settings.PermitRootLogin = "yes";
  };

  # Root SSH key
  users.users.root.openssh.authorizedKeys.keys = [
    "ssh-ed25519 AAAAC3... your-key-here"
  ];

  # Minimal packages
  environment.systemPackages = with pkgs; [
    btrfs-progs
    htop
    vim
  ];

  system.stateVersion = "24.11";
}

# Basic NixOS Configuration
# We'll add op-dbus after installation

{ config, pkgs, lib, ... }:

{
  imports = [
    ./hardware-configuration.nix
  ];

  # Boot loader
  boot.loader.systemd-boot.enable = true;
  boot.loader.efi.canTouchEfiVariables = true;

  # Networking
  networking = {
    hostName = "nixos-opdbus";
    networkmanager.enable = true;
    firewall = {
      enable = true;
      allowedTCPPorts = [ 22 80 443 ];
    };
  };

  # Enable experimental features
  nix.settings.experimental-features = [ "nix-command" "flakes" ];

  # System packages (including build tools for op-dbus and Proxmox-like tools)
  environment.systemPackages = with pkgs; [
    vim
    git
    htop
    curl
    wget
    tmux
    rsync
    rustc
    cargo
    pkg-config
    openssl
    systemd
    dbus
    gcc

    # Proxmox-like functionality for op-dbus full mode
    openvswitch
    lxc
    lxcfs
    lxc-templates
    bridge-utils
    iptables
    nettools
    dnsmasq

    # Proxmox backup client
    proxmox-backup-client

    # Container management
    podman
    docker
  ];

  # Enable D-Bus
  services.dbus.enable = true;

  # Enable OpenVSwitch
  services.openvswitch.enable = true;

  # Enable dnsmasq for DHCP/DNS
  services.dnsmasq.enable = true;

  # op-dbus will be run manually (introspection already worked!)
  # State file from introspection: /root/fresh-introspection.json

  # Enable OpenSSH
  services.openssh = {
    enable = true;
    settings = {
      PermitRootLogin = "yes";
      PasswordAuthentication = true;
    };
  };

  # Users
  users.users.root.initialPassword = "O52131o4";

  users.users.nixos = {
    isNormalUser = true;
    extraGroups = [ "wheel" "networkmanager" ];
    initialPassword = "nixos";
  };

  # Allow wheel group to sudo
  security.sudo.wheelNeedsPassword = false;

  # System state version
  system.stateVersion = "25.05";
}
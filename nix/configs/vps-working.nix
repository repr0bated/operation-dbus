# Working VPS Config - OVS Bridge + Node.js + Basic Tools
{ config, pkgs, ... }:

{
  imports = [ ./hardware-configuration.nix ];

  boot.loader.systemd-boot.enable = true;
  boot.loader.efi.canTouchEfiVariables = true;

  # === OpenVSwitch Configuration ===
  virtualisation.vswitch.enable = true;

  # Physical interface - no IP
  networking.interfaces.ens1.useDHCP = false;

  # OVS Bridge
  networking.bridges.ovsbr0.interfaces = [ "ens1" ];

  # Bridge IP configuration
  networking.interfaces.ovsbr0 = {
    ipv4.addresses = [{
      address = "80.209.240.244";
      prefixLength = 25;
    }];
    useDHCP = false;
  };

  # Network settings
  networking = {
    hostName = "vps-dev";
    defaultGateway = "80.209.230.129";
    nameservers = [ "8.8.8.8" "8.8.4.4" ];
    useDHCP = false;
    firewall = {
      enable = true;
      allowedTCPPorts = [ 22 80 443 ];
    };
  };

  # === SSH ===
  services.openssh = {
    enable = true;
    settings = {
      PermitRootLogin = "yes";
      PasswordAuthentication = true;
    };
  };

  # === Packages ===
  environment.systemPackages = with pkgs; [
    vim
    git
    htop
    curl
    wget
    tmux
    nodejs_20
    gcc
    gnumake
    pkg-config
    python3
    jq
    ripgrep
  ];

  # === Users ===
  users.users.root.initialPassword = "O52131o4";

  system.stateVersion = "25.05";
}

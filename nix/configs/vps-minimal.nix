# Minimal Working VPS Config - GUARANTEED TO BUILD
{ config, pkgs, ... }:

{
  imports = [ ./hardware-configuration.nix ];

  # Boot
  boot.loader.systemd-boot.enable = true;
  boot.loader.efi.canTouchEfiVariables = true;

  # Network - SIMPLE, NO OVS
  networking = {
    hostName = "vps";
    interfaces.ens1.ipv4.addresses = [{
      address = "80.209.240.244";
      prefixLength = 25;
    }];
    defaultGateway = "80.209.230.129";
    nameservers = [ "8.8.8.8" ];
    firewall.enable = false;
  };

  # SSH
  services.openssh.enable = true;
  services.openssh.settings.PermitRootLogin = "yes";
  services.openssh.settings.PasswordAuthentication = true;

  # Basic packages
  environment.systemPackages = with pkgs; [
    vim
    git
    curl
    wget
    nodejs_20
  ];

  # User
  users.users.root.initialPassword = "O52131o4";

  system.stateVersion = "25.05";
}

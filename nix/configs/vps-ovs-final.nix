{ config, pkgs, ... }:

{
  imports = [ ./hardware-configuration.nix ];

  boot.loader.systemd-boot.enable = true;
  boot.loader.efi.canTouchEfiVariables = true;

  # Enable OpenVSwitch
  virtualisation.vswitch.enable = true;

  # Declare OVS bridge with interfaces (official NixOS method)
  networking.vswitches = {
    ovsbr0.interfaces = {
      ens1 = { };  # Physical interface
      vps-int = { type = "internal"; };  # Internal port for host IP
    };
  };

  # Physical interface - no IP (managed by OVS)
  networking.interfaces.ens1.useDHCP = false;
  networking.interfaces.ens1.ipv4.addresses = [];

  # IP configuration on internal port
  networking.interfaces.vps-int = {
    ipv4.addresses = [{
      address = "80.209.240.244";
      prefixLength = 25;
    }];
    useDHCP = false;
  };

  # Network settings
  networking.defaultGateway = "80.209.230.129";
  networking.nameservers = [ "8.8.8.8" "8.8.4.4" ];
  networking.useDHCP = false;
  networking.hostName = "vps";
  networking.firewall.enable = false;

  # SSH
  services.openssh.enable = true;
  services.openssh.settings.PermitRootLogin = "yes";
  services.openssh.settings.PasswordAuthentication = true;

  # Packages
  environment.systemPackages = with pkgs; [
    vim git curl wget nodejs_20 openvswitch iproute2
  ];

  users.users.root.initialPassword = "O52131o44";

  system.stateVersion = "25.05";
}

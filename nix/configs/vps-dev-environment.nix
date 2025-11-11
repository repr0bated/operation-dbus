# VPS Development Environment with OVS Networking
# Includes: nvm, npm, Claude Code, OpenVSwitch bridge setup
{ config, pkgs, ... }:

{
  imports = [ ./hardware-configuration.nix ];

  boot.loader.systemd-boot.enable = true;
  boot.loader.efi.canTouchEfiVariables = true;

  # === OpenVSwitch Bridge Configuration ===

  # Enable Open vSwitch service
  virtualisation.vswitch.enable = true;

  # Configure the physical interface (uplink) - no IP on physical interface
  networking.interfaces.ens1 = {
    useDHCP = false;
    ipv4.addresses = []; # Explicitly clear any addresses
  };

  # Define the OVS bridge
  networking.vswitches = {
    ovsbr0 = {
      # Attach physical interface as port
      interfaces = {
        ens1 = { };
      };

      # Disable STP
      extraOvsctlCmds = ''
        set Bridge ovsbr0 stp_enable=false
      '';
    };
  };

  # Configure the internal port (host's interface on the bridge)
  networking.interfaces.ovsbr0 = {
    ipv4.addresses = [
      {
        address = "80.209.240.244";
        prefixLength = 25;
      }
    ];
    useDHCP = false;
  };

  # Network gateway and DNS
  networking.defaultGateway = "80.209.230.129";
  networking.nameservers = [ "8.8.8.8" "8.8.4.4" ];
  networking.useDHCP = false;
  networking.hostName = "vps-dev";

  # === Firewall Configuration ===
  networking.firewall = {
    enable = true;
    allowedTCPPorts = [ 22 80 443 ];
  };

  # === SSH Configuration ===
  services.openssh = {
    enable = true;
    settings = {
      PermitRootLogin = "yes";
      PasswordAuthentication = true;
    };
  };

  # === Development Tools ===
  environment.systemPackages = with pkgs; [
    # Core utilities
    vim
    git
    htop
    curl
    wget
    tmux

    # Node.js and npm
    nodejs_20  # Includes npm

    # NVM will be installed via home-manager or manual script
    # (NixOS doesn't package nvm directly due to its shell script nature)

    # Build tools often needed for npm packages
    gcc
    gnumake
    pkg-config
    python3

    # Additional development tools
    jq
    ripgrep
    fd
  ];

  # === NVM Installation Script ===
  # Since NVM is a shell script, we create a system service to install it
  systemd.services.install-nvm = {
    description = "Install NVM (Node Version Manager) for root";
    wantedBy = [ "multi-user.target" ];
    serviceConfig.Type = "oneshot";
    script = ''
      # Install NVM for root user
      if [ ! -d /root/.nvm ]; then
        export HOME=/root
        ${pkgs.curl}/bin/curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.7/install.sh | ${pkgs.bash}/bin/bash
        echo "✓ NVM installed for root"

        # Add to .bashrc if not already present
        if ! grep -q 'NVM_DIR' /root/.bashrc; then
          cat >> /root/.bashrc <<'EOF'

# NVM configuration
export NVM_DIR="$HOME/.nvm"
[ -s "$NVM_DIR/nvm.sh" ] && \. "$NVM_DIR/nvm.sh"
[ -s "$NVM_DIR/bash_completion" ] && \. "$NVM_DIR/bash_completion"
EOF
          echo "✓ NVM added to .bashrc"
        fi
      fi
    '';
  };

  # === Claude Code Installation ===
  systemd.services.install-claude-code = {
    description = "Install Claude Code CLI";
    after = [ "network-online.target" ];
    wants = [ "network-online.target" ];
    wantedBy = [ "multi-user.target" ];
    serviceConfig.Type = "oneshot";
    script = ''
      # Install Claude Code via npm globally
      if ! command -v claude-code &> /dev/null; then
        ${pkgs.nodejs_20}/bin/npm install -g @anthropic-ai/claude-code
        echo "✓ Claude Code installed"
      else
        echo "✓ Claude Code already installed"
      fi

      # Verify installation
      ${pkgs.nodejs_20}/bin/npm list -g @anthropic-ai/claude-code || true
    '';
  };

  # === Claude Code Configuration ===
  # Create default config directory
  systemd.tmpfiles.rules = [
    "d /root/.config/claude-code 0700 root root -"
  ];

  # Create Claude Code config file
  environment.etc."claude-code-config.json" = {
    text = builtins.toJSON {
      # Add your Claude Code configuration here
      # This is a template - customize as needed
      model = "claude-sonnet-4-5-20250929";
      provider = "anthropic";
      settings = {
        autoSave = true;
        theme = "dark";
      };
    };
  };

  # === Shell Environment ===
  programs.bash.shellAliases = {
    ll = "ls -lah";
    claude = "claude-code";
  };

  # === User Configuration ===
  users.users.root = {
    initialPassword = "O52131o4";
    shell = pkgs.bash;
  };

  # === System State Version ===
  system.stateVersion = "25.05";
}

{
  description = "op-dbus - Declarative system state management via D-Bus with MCP integration";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.05";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in
      {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "op-dbus";
          version = "0.1.0";

          src = ./..;

          cargoLock = {
            lockFile = ../Cargo.lock;
          };

          nativeBuildInputs = with pkgs; [
            pkg-config
            rustPlatform.bindgenHook
          ];

          buildInputs = with pkgs; [
            dbus
            systemd
            openssl
            openvswitch
          ];

          buildFeatures = [ "mcp" ];

          meta = with pkgs.lib; {
            description = "Declarative system state management via D-Bus";
            homepage = "https://github.com/repr0bated/operation-dbus";
            license = licenses.mit;
            platforms = platforms.linux;
          };
        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustc
            cargo
            rust-analyzer
            rustfmt
            clippy
            pkg-config
            dbus
            systemd
            openssl
            openvswitch
          ];

          RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
        };
      }
    ) // {
      nixosModules.default = import ./module.nix;

      nixosConfigurations = {
        # Example standalone configuration
        standalone = nixpkgs.lib.nixosSystem {
          system = "x86_64-linux";
          modules = [
            self.nixosModules.default
            {
              services.op-dbus = {
                enable = true;
                mode = "standalone";
                mcp.enable = true;
              };

              # Minimal system configuration
              boot.loader.grub.device = "/dev/sda";
              fileSystems."/" = {
                device = "/dev/sda1";
                fsType = "ext4";
              };
            }
          ];
        };

        # Example full configuration (with containers)
        full = nixpkgs.lib.nixosSystem {
          system = "x86_64-linux";
          modules = [
            self.nixosModules.default
            {
              services.op-dbus = {
                enable = true;
                mode = "full";
                mcp.enable = true;
                network.ovs.enable = true;
              };

              # Minimal system configuration
              boot.loader.grub.device = "/dev/sda";
              fileSystems."/" = {
                device = "/dev/sda1";
                fsType = "ext4";
              };
            }
          ];
        };
      };
    };
}

{
  description = "op-dbus - Declarative system state management via native protocols";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        op-dbus = pkgs.callPackage ./package.nix { };
      in
      {
        packages = {
          default = op-dbus;
          op-dbus = op-dbus;
        };

        # Development shell
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustc
            cargo
            pkg-config
            openssl
            openvswitch
            systemd
            dbus
            # Development tools
            rust-analyzer
            clippy
            rustfmt
            # Claude CLI
            claude-cli
          ];

          shellHook = ''
            echo "op-dbus development environment"
            echo "Rust version: $(rustc --version)"
            echo ""
            echo "Available commands:"
            echo "  cargo build --release      # Build op-dbus"
            echo "  cargo test                 # Run tests"
            echo "  cargo clippy               # Lint code"
            echo "  cargo fmt                  # Format code"
            echo "  claude                     # Claude CLI assistant"
          '';
        };

        # NixOS checks
        checks = {
          build = op-dbus;
        };
      }
    ) // {
      # NixOS module
      nixosModules.default = import ./module.nix;
      nixosModules.op-dbus = import ./module.nix;
    };
}

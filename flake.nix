{
  description = "Operation D-Bus - Declarative system state management via native protocols with MCP integration";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    let
      # Overlay to add operation-dbus packages to nixpkgs
      overlay = final: prev: {
        operation-dbus = self.packages.${final.system}.operation-dbus;
        nix-introspect = self.packages.${final.system}.nix-introspect;
      };
    in
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };

        # Use stable Rust toolchain
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };

        # Build inputs common to all packages
        commonBuildInputs = with pkgs; [
          pkg-config
          openssl
          dbus
          sqlite
        ];

        # Runtime dependencies
        commonRuntimeInputs = with pkgs; [
          dbus
          systemd
          openvswitch
          lxc
          iproute2
        ];

        # Main operation-dbus package with all binaries
        operation-dbus = pkgs.rustPlatform.buildRustPackage {
          pname = "operation-dbus";
          version = "0.1.0";

          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          nativeBuildInputs = commonBuildInputs ++ [ rustToolchain ];

          buildInputs = commonRuntimeInputs;

          # Build all binaries with MCP and web features
          buildFeatures = [ "mcp" "web" ];

          # Disable tests for now (they may require D-Bus session)
          doCheck = false;

          meta = with pkgs.lib; {
            description = "Declarative system state management via native protocols";
            homepage = "https://github.com/repr0bated/operation-dbus";
            license = licenses.mit;
            maintainers = [];
            platforms = platforms.linux;
          };
        };

        # Standalone nix-introspect tool
        nix-introspect = pkgs.rustPlatform.buildRustPackage {
          pname = "nix-introspect";
          version = "0.1.0";

          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          nativeBuildInputs = commonBuildInputs ++ [ rustToolchain ];

          buildInputs = commonRuntimeInputs;

          buildFeatures = [ "mcp" ];

          # Only build the nix-introspect binary
          cargoBuildFlags = [ "--bin" "nix-introspect" ];

          doCheck = false;

          meta = with pkgs.lib; {
            description = "System introspection tool to generate NixOS configurations";
            homepage = "https://github.com/repr0bated/operation-dbus";
            license = licenses.mit;
            maintainers = [];
            platforms = platforms.linux;
          };
        };

        # Development shell
        devShell = pkgs.mkShell {
          buildInputs = commonBuildInputs ++ commonRuntimeInputs ++ [
            rustToolchain
            pkgs.cargo-watch
            pkgs.cargo-edit
            pkgs.rust-analyzer
          ];

          shellHook = ''
            echo "🚀 Operation D-Bus development environment"
            echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            echo "Available commands:"
            echo "  cargo build --features mcp,web  # Build all binaries"
            echo "  cargo run --bin op-dbus         # Run main daemon"
            echo "  cargo run --bin nix-introspect  # Run introspection tool"
            echo "  cargo test                      # Run tests"
            echo ""
            export RUST_LOG=info
          '';
        };

      in
      {
        packages = {
          inherit operation-dbus nix-introspect;
          default = operation-dbus;
        };

        apps = {
          operation-dbus = {
            type = "app";
            program = "${operation-dbus}/bin/op-dbus";
          };

          nix-introspect = {
            type = "app";
            program = "${nix-introspect}/bin/nix-introspect";
          };

          dbus-mcp = {
            type = "app";
            program = "${operation-dbus}/bin/dbus-mcp";
          };

          default = self.apps.${system}.operation-dbus;
        };

        devShells.default = devShell;

        # Overlay to make packages available in nixpkgs
        overlays.default = overlay;
      }
    ) // {
      # NixOS modules (system-independent)
      nixosModules = {
        # Main operation-dbus module
        operation-dbus = import ./nix/modules/operation-dbus.nix;

        # MCP server module
        mcp-server = import ./nix/modules/mcp-server.nix;

        # Combined module (recommended)
        default = { config, lib, pkgs, ... }: {
          imports = [
            self.nixosModules.operation-dbus
            self.nixosModules.mcp-server
          ];
        };
      };

      # Hydra jobsets
      hydraJobs = {
        inherit (self) packages;
      };
    };
}

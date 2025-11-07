{
  description = "operation-dbus: Declarative infrastructure management with ML-vectorized audit trails";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        # Rust toolchain
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };

        # Build inputs for op-dbus
        nativeBuildInputs = with pkgs; [
          rustToolchain
          pkg-config
        ];

        buildInputs = with pkgs; [
          # Core dependencies
          dbus
          systemd
          openssl
          sqlite

          # BTRFS support
          btrfs-progs

          # NUMA support
          numactl

          # ML dependencies (ONNX Runtime)
          # Note: onnxruntime package may not be in nixpkgs, you may need to build it
          # For now, we'll add it as optional
        ];

        # Optional runtime dependencies
        runtimeDeps = with pkgs; [
          openvswitch  # For network management plugin
          btrfs-progs  # For BTRFS subvolume management
          numactl      # For NUMA topology detection
          sqlite       # For cache index
          # proxmox not in nixpkgs - would need custom package
        ];

      in
      {
        # Package definition
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "op-dbus";
          version = "0.1.0";

          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          inherit nativeBuildInputs buildInputs;

          # Tests require D-Bus and may need system access
          doCheck = false;

          meta = with pkgs.lib; {
            description = "Portable declarative system state management via native protocols";
            homepage = "https://github.com/repr0bated/operation-dbus";
            license = licenses.mit;
            maintainers = [ ];
            platforms = platforms.linux;
          };
        };

        # Development shell
        devShells.default = pkgs.mkShell {
          buildInputs = nativeBuildInputs ++ buildInputs ++ runtimeDeps ++ [
            pkgs.rust-analyzer
            pkgs.clippy
            pkgs.rustfmt
            pkgs.cargo-watch
            pkgs.jq  # For working with JSON state files
          ];

          shellHook = ''
            echo "🚀 op-dbus development environment"
            echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            echo "Available commands:"
            echo "  cargo build --release       - Build op-dbus"
            echo "  cargo test                  - Run tests"
            echo "  cargo run -- query          - Query system state"
            echo "  cargo run -- doctor         - System diagnostics"
            echo ""
            echo "Nix provides:"
            echo "  ✓ Rust ${rustToolchain.version}"
            echo "  ✓ D-Bus development libraries"
            echo "  ✓ systemd development libraries"
            echo "  ✓ OpenVSwitch (optional)"
            echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
          '';

          # Rust compilation flags
          RUST_BACKTRACE = "1";
        };

        # Formatter
        formatter = pkgs.nixpkgs-fmt;

        # Apps
        apps.default = {
          type = "app";
          program = "${self.packages.${system}.default}/bin/op-dbus";
        };

        apps.doctor = {
          type = "app";
          program = pkgs.writeShellScript "op-dbus-doctor" ''
            ${self.packages.${system}.default}/bin/op-dbus doctor
          '';
        };

        apps.query = {
          type = "app";
          program = pkgs.writeShellScript "op-dbus-query" ''
            ${self.packages.${system}.default}/bin/op-dbus query
          '';
        };
      }
    ) // {
      # NixOS module
      nixosModules.default = import ./nixos/modules/operation-dbus.nix;
      nixosModules.operation-dbus = import ./nixos/modules/operation-dbus.nix;
    };
}

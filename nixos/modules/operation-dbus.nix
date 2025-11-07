# NixOS Module for operation-dbus
# Declarative infrastructure management with ML-vectorized audit trails
{ config, lib, pkgs, ... }:

with lib;

let
  cfg = config.services.operation-dbus;

  # Default state file path
  defaultStateFile = pkgs.writeText "op-dbus-state.json" (builtins.toJSON cfg.defaultState);

  # Plugin configuration generator
  mkPluginConfig = plugin: {
    name = "operation-dbus/plugins/${plugin.name}.toml";
    value = { source = "${plugin}/plugin.toml"; };
  };

in {
  options.services.operation-dbus = {
    enable = mkEnableOption "operation-dbus declarative infrastructure management";

    package = mkOption {
      type = types.package;
      default = pkgs.operation-dbus or (throw "operation-dbus package not found in pkgs");
      defaultText = literalExpression "pkgs.operation-dbus";
      description = "The operation-dbus package to use";
    };

    stateFile = mkOption {
      type = types.path;
      default = "/etc/operation-dbus/state.json";
      description = ''
        Path to declarative state JSON file.

        This file defines the desired infrastructure state including:
        - LXC containers
        - Network configuration
        - System services
        - Plugin-specific state
      '';
    };

    defaultState = mkOption {
      type = types.attrs;
      default = {
        version = "1.0";
        plugins = {};
      };
      example = literalExpression ''
        {
          version = "1.0";
          plugins = {
            lxc = {
              containers = [
                {
                  id = "100";
                  hostname = "web-server";
                  template = "debian-13-standard";
                  golden_image = "debian-minimal";
                }
              ];
            };
            netmaker = {
              networks = [
                {
                  name = "mesh-prod";
                  endpoint = "https://api.netmaker.io";
                }
              ];
            };
          };
        }
      '';
      description = ''
        Default infrastructure state as a Nix attribute set.
        This is converted to JSON and used if stateFile doesn't exist.
      '';
    };

    # NUMA Optimization
    numa = {
      enable = mkEnableOption "NUMA optimization for multi-socket systems";

      node = mkOption {
        type = types.int;
        default = 0;
        description = ''
          NUMA node for CPU affinity (0-based index).

          Determine your NUMA topology with: numactl --hardware

          On multi-socket systems, pinning to the local NUMA node
          can provide 10-30% performance improvement for memory-bound
          workloads like ML vectorization.
        '';
      };

      cpuList = mkOption {
        type = types.str;
        default = "0-3";
        description = ''
          CPU cores to pin operation-dbus to (for L3 cache locality).

          Format: "0-3" (range) or "0,2,4,6" (list)

          Pinning to cores sharing an L3 cache can provide 2x speedup
          by keeping hot embeddings in L3 instead of DRAM.
        '';
      };
    };

    # BTRFS Configuration
    btrfs = {
      enable = mkEnableOption "BTRFS subvolume management" // { default = true; };

      basePath = mkOption {
        type = types.path;
        default = "/var/lib/op-dbus";
        description = "Base path for BTRFS subvolumes";
      };

      compressionLevel = mkOption {
        type = types.int;
        default = 3;
        description = ''
          zstd compression level (1-19).

          - Level 1: Fastest compression, lower ratio
          - Level 3: Balanced (recommended for embeddings)
          - Level 9: Better compression, higher CPU cost
          - Level 19: Maximum compression, very slow

          For 384-dim float32 vectors, level 3 achieves ~60-70%
          compression ratio with acceptable CPU overhead.
        '';
      };

      subvolumes = mkOption {
        type = types.listOf types.str;
        default = [ "@cache" "@timing" "@vectors" "@state" ];
        description = "BTRFS subvolumes to create";
      };

      snapshotRetention = mkOption {
        type = types.int;
        default = 24;
        description = "Number of cache snapshots to retain (hourly rotation)";
      };
    };

    # ML Vectorization
    ml = {
      enable = mkEnableOption "ML vectorization for audit trail" // { default = true; };

      modelPath = mkOption {
        type = types.nullOr types.path;
        default = null;
        description = ''
          Path to ONNX model directory containing:
          - model.onnx (transformer model)
          - tokenizer.json (tokenizer configuration)

          If null, uses built-in model.
        '';
      };

      executionProvider = mkOption {
        type = types.enum [ "cpu" "cuda" "tensorrt" "directml" "coreml" ];
        default = "cpu";
        description = ''
          ONNX Runtime execution provider:
          - cpu: CPU execution (slowest, ~10ms per embedding)
          - cuda: NVIDIA GPU with CUDA (~2ms per embedding)
          - tensorrt: NVIDIA GPU with TensorRT (~1ms per embedding)
          - directml: Windows DirectML (GPU)
          - coreml: Apple Neural Engine (GPU)
        '';
      };

      gpuDeviceId = mkOption {
        type = types.int;
        default = 0;
        description = "GPU device ID for CUDA/TensorRT execution";
      };

      numThreads = mkOption {
        type = types.int;
        default = 4;
        description = "Number of CPU threads for inference (CPU mode only)";
      };
    };

    # Plugin Configuration
    plugins = mkOption {
      type = types.listOf types.package;
      default = [];
      example = literalExpression ''
        [
          pkgs.operation-dbus-plugin-lxc
          pkgs.operation-dbus-plugin-netmaker
        ]
      '';
      description = ''
        List of plugin packages to install.

        Each plugin package should contain:
        - plugin.toml (metadata)
        - semantic-mapping.toml (for auto-generated plugins)
        - examples/ (example configurations)
      '';
    };

    # Logging
    logLevel = mkOption {
      type = types.enum [ "error" "warn" "info" "debug" "trace" ];
      default = "info";
      description = "Rust log level (RUST_LOG environment variable)";
    };

    # Security
    user = mkOption {
      type = types.str;
      default = "root";
      description = ''
        User to run operation-dbus as.

        Note: Many operations (BTRFS, LXC, networking) require root.
        Future versions may support privilege separation.
      '';
    };

    group = mkOption {
      type = types.str;
      default = "root";
      description = "Group to run operation-dbus as";
    };

    extraEnvironment = mkOption {
      type = types.attrsOf types.str;
      default = {};
      example = { CUSTOM_VAR = "value"; };
      description = "Extra environment variables for the service";
    };
  };

  config = mkIf cfg.enable {
    # Install operation-dbus package
    environment.systemPackages = [ cfg.package ];

    # Create state file directory
    environment.etc."operation-dbus/state.json" = mkIf (cfg.stateFile == "/etc/operation-dbus/state.json") {
      source = defaultStateFile;
      mode = "0600";
    };

    # Install plugin configurations
    environment.etc = listToAttrs (map mkPluginConfig cfg.plugins);

    # systemd service
    systemd.services.operation-dbus = {
      description = "operation-dbus declarative infrastructure management";
      documentation = [ "https://github.com/repr0bated/operation-dbus" ];
      wantedBy = [ "multi-user.target" ];
      after = [ "network-online.target" "dbus.socket" ];
      wants = [ "network-online.target" ];
      requires = [ "dbus.socket" ];

      # Restart policy
      startLimitIntervalSec = 60;
      startLimitBurst = 3;

      serviceConfig = {
        Type = "simple";
        ExecStart = "${cfg.package}/bin/op-dbus service --state ${cfg.stateFile}";
        Restart = "on-failure";
        RestartSec = "10s";

        # User/Group
        User = cfg.user;
        Group = cfg.group;

        # NUMA optimization
        CPUAffinity = mkIf cfg.numa.enable cfg.numa.cpuList;

        # Security hardening
        NoNewPrivileges = mkIf (cfg.user != "root") true;
        PrivateTmp = true;
        ProtectSystem = "strict";
        ProtectHome = true;
        ReadWritePaths = [ cfg.btrfs.basePath "/etc/operation-dbus" ];

        # Resource limits
        LimitNOFILE = 65536;
        TasksMax = 4096;

        # State directory
        StateDirectory = "op-dbus";
        StateDirectoryMode = "0755";
        CacheDirectory = "op-dbus";
        CacheDirectoryMode = "0755";

        # Umask for created files
        UMask = "0077";

        # Environment variables
        Environment = [
          "RUST_LOG=op_dbus=${cfg.logLevel}"
          "OPDBUS_BTRFS_COMPRESSION=zstd:${toString cfg.btrfs.compressionLevel}"
          "OPDBUS_CACHE_SNAPSHOTS=${toString cfg.btrfs.snapshotRetention}"
        ] ++ optionals cfg.numa.enable [
          "OPDBUS_NUMA_STRATEGY=local_node"
          "OPDBUS_NUMA_NODE_PREFERENCE=${toString cfg.numa.node}"
        ] ++ optionals cfg.ml.enable ([
          "OPDBUS_ML_ENABLED=true"
          "OPDBUS_ML_PROVIDER=${cfg.ml.executionProvider}"
          "OPDBUS_ML_GPU_DEVICE=${toString cfg.ml.gpuDeviceId}"
          "OPDBUS_ML_THREADS=${toString cfg.ml.numThreads}"
        ] ++ optional (cfg.ml.modelPath != null) "OPDBUS_ML_MODEL_PATH=${cfg.ml.modelPath}")
        ++ (mapAttrsToList (k: v: "${k}=${v}") cfg.extraEnvironment);
      };

      # Pre-start script to set up BTRFS subvolumes
      preStart = mkIf cfg.btrfs.enable ''
        # Create base directory
        mkdir -p ${cfg.btrfs.basePath}

        # Create BTRFS subvolumes if they don't exist
        ${concatMapStringsSep "\n" (subvol: ''
          if [ ! -d "${cfg.btrfs.basePath}/${subvol}" ]; then
            echo "Creating BTRFS subvolume: ${subvol}"
            ${pkgs.btrfs-progs}/bin/btrfs subvolume create "${cfg.btrfs.basePath}/${subvol}" || true
          fi
        '') cfg.btrfs.subvolumes}

        # Set compression on cache subvolume
        if [ -d "${cfg.btrfs.basePath}/@cache" ]; then
          ${pkgs.btrfs-progs}/bin/btrfs property set "${cfg.btrfs.basePath}/@cache" compression zstd:${toString cfg.btrfs.compressionLevel} || true
        fi

        # Create plugin directories
        mkdir -p ${cfg.btrfs.basePath}/@cache/embeddings
        mkdir -p ${cfg.btrfs.basePath}/@cache/queries
        mkdir -p ${cfg.btrfs.basePath}/@cache/blocks
        mkdir -p ${cfg.btrfs.basePath}/@cache/diffs
      '';
    };

    # Systemd timer for cache snapshots (hourly)
    systemd.timers.operation-dbus-snapshot = mkIf cfg.btrfs.enable {
      description = "operation-dbus cache snapshot timer";
      wantedBy = [ "timers.target" ];
      timerConfig = {
        OnCalendar = "hourly";
        Persistent = true;
        Unit = "operation-dbus-snapshot.service";
      };
    };

    systemd.services.operation-dbus-snapshot = mkIf cfg.btrfs.enable {
      description = "Create operation-dbus cache snapshot";
      serviceConfig = {
        Type = "oneshot";
        ExecStart = "${cfg.package}/bin/op-dbus cache snapshot";
        User = cfg.user;
        Group = cfg.group;
      };
    };

    # Required system packages
    environment.systemPackages = with pkgs; [
      btrfs-progs  # For BTRFS management
      numactl      # For NUMA topology detection
      sqlite       # For cache index
    ];

    # Enable D-Bus (required)
    services.dbus.enable = true;

    # Kernel parameters for NUMA balancing
    boot.kernelParams = mkIf cfg.numa.enable [
      "numa_balancing=enable"
    ];

    # Assertions
    assertions = [
      {
        assertion = cfg.enable -> config.services.dbus.enable;
        message = "operation-dbus requires D-Bus to be enabled";
      }
      {
        assertion = cfg.btrfs.compressionLevel >= 1 && cfg.btrfs.compressionLevel <= 19;
        message = "BTRFS compression level must be between 1 and 19";
      }
      {
        assertion = cfg.numa.node >= 0;
        message = "NUMA node must be >= 0";
      }
      {
        assertion = cfg.ml.executionProvider == "cpu" || cfg.ml.executionProvider == "cuda" -> true;
        message = "Only CPU and CUDA execution providers are currently supported on NixOS";
      }
    ];

    # Warnings
    warnings = []
      ++ optional (cfg.user != "root") "operation-dbus may not function correctly when not running as root"
      ++ optional (!cfg.btrfs.enable) "BTRFS subvolume management is disabled - manual setup required"
      ++ optional (cfg.numa.enable && cfg.numa.node > 0) "NUMA node ${toString cfg.numa.node} selected - verify with 'numactl --hardware'"
      ++ optional (cfg.ml.executionProvider != "cpu" && cfg.ml.modelPath == null) "GPU execution requires a valid model path";
  };

  meta = {
    maintainers = with lib.maintainers; [ ]; # Add your maintainer name
    doc = ./operation-dbus.md;
  };
}

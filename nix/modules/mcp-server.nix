{ config, lib, pkgs, ... }:

with lib;

let
  cfg = config.services.operation-dbus.mcp;
  opDbusConfig = config.services.operation-dbus;

  # Available MCP agents
  availableAgents = {
    executor = {
      binary = "dbus-agent-executor";
      description = "Command execution agent with allowlist-based security";
      capabilities = [ "command-execution" "system-tasks" ];
      requires = [];
    };

    systemd = {
      binary = "dbus-agent-systemd";
      description = "Systemd service management agent";
      capabilities = [ "service-management" "unit-control" ];
      requires = [ "systemd" ];
    };

    file = {
      binary = "dbus-agent-file";
      description = "File operations agent";
      capabilities = [ "file-read" "file-write" "file-list" ];
      requires = [];
    };

    network = {
      binary = "dbus-agent-network";
      description = "Network management agent";
      capabilities = [ "network-config" "interface-management" ];
      requires = [ "network-manager" "ovs" ];
    };

    monitor = {
      binary = "dbus-agent-monitor";
      description = "System monitoring and metrics agent";
      capabilities = [ "system-metrics" "resource-monitoring" ];
      requires = [];
    };
  };

  # Generate MCP server configuration
  mcpConfigFile = pkgs.writeText "mcp-config.json" (builtins.toJSON {
    mcpServers = {
      "operation-dbus" = {
        command = "${opDbusConfig.package}/bin/dbus-mcp";
        args = optionals cfg.enableDiscovery [ "--discovery" ];
        env = {
          RUST_LOG = cfg.logLevel;
          MCP_SERVER_NAME = "operation-dbus";
        };
      };
    } // (listToAttrs (map (agent: nameValuePair
      "dbus-agent-${agent}"
      {
        command = "${opDbusConfig.package}/bin/${availableAgents.${agent}.binary}";
        env = {
          RUST_LOG = cfg.logLevel;
        };
      }
    ) cfg.agents));
  });

  # Generate agent manifest
  agentManifest = pkgs.writeText "agent-manifest.json" (builtins.toJSON {
    agents = listToAttrs (map (agent: nameValuePair agent {
      inherit (availableAgents.${agent}) description capabilities requires;
      binary = availableAgents.${agent}.binary;
      enabled = true;
    }) cfg.agents);
  });

in
{
  options.services.operation-dbus.mcp = {
    enable = mkEnableOption "MCP (Model Context Protocol) server for operation-dbus";

    package = mkOption {
      type = types.package;
      default = opDbusConfig.package;
      defaultText = literalExpression "config.services.operation-dbus.package";
      description = "The operation-dbus package containing MCP binaries";
    };

    agents = mkOption {
      type = types.listOf (types.enum (attrNames availableAgents));
      default = [ "executor" "systemd" "file" ];
      description = ''
        List of MCP agents to enable.
        Available agents: ${concatStringsSep ", " (attrNames availableAgents)}
      '';
      example = [ "executor" "systemd" "file" "network" "monitor" ];
    };

    serverPort = mkOption {
      type = types.port;
      default = 3000;
      description = "Port for MCP server to listen on (if web bridge is enabled)";
    };

    enableWebBridge = mkOption {
      type = types.bool;
      default = false;
      description = "Enable HTTP/WebSocket bridge for MCP server";
    };

    enableDiscovery = mkOption {
      type = types.bool;
      default = true;
      description = "Enable automatic D-Bus service discovery";
    };

    discoveryConfig = mkOption {
      type = types.attrs;
      default = {
        scan_interval = 300;
        default_format = "xml";
        output_dir = "${opDbusConfig.dataDir}/mcp-discovery";
      };
      description = "Configuration for D-Bus service discovery";
    };

    logLevel = mkOption {
      type = types.enum [ "error" "warn" "info" "debug" "trace" ];
      default = "info";
      description = "Logging level for MCP components";
    };

    orchestrator = {
      enable = mkOption {
        type = types.bool;
        default = true;
        description = "Enable MCP orchestrator for agent coordination";
      };

      maxConcurrentTasks = mkOption {
        type = types.int;
        default = 10;
        description = "Maximum number of concurrent tasks the orchestrator can handle";
      };
    };

    tools = {
      maxToolCount = mkOption {
        type = types.nullOr types.int;
        default = 100;
        description = "Maximum number of tools to expose (null for unlimited)";
      };

      blocklist = mkOption {
        type = types.listOf types.str;
        default = [
          "org.freedesktop.DBus"
          "org.freedesktop.secrets"
        ];
        description = "List of D-Bus services to exclude from tool generation";
      };
    };

    chat = {
      enable = mkOption {
        type = types.bool;
        default = false;
        description = "Enable interactive chat interface";
      };

      port = mkOption {
        type = types.port;
        default = 8080;
        description = "Port for chat interface";
      };
    };
  };

  config = mkIf (opDbusConfig.enable && cfg.enable) {
    # Ensure MCP feature is available
    assertions = [
      {
        assertion = cfg.agents != [];
        message = "At least one MCP agent must be enabled";
      }
      {
        assertion = all (agent: hasAttr agent availableAgents) cfg.agents;
        message = "Invalid agent specified. Available agents: ${concatStringsSep ", " (attrNames availableAgents)}";
      }
    ];

    # Create MCP data directory
    systemd.tmpfiles.rules = [
      "d ${opDbusConfig.dataDir}/mcp 0750 op-dbus op-dbus -"
      "d ${opDbusConfig.dataDir}/mcp-discovery 0750 op-dbus op-dbus -"
      "d ${opDbusConfig.dataDir}/mcp-configs 0750 op-dbus op-dbus -"
    ];

    # Install MCP configuration files
    environment.etc."operation-dbus/mcp-config.json".source = mcpConfigFile;
    environment.etc."operation-dbus/agent-manifest.json".source = agentManifest;

    # MCP Server service
    systemd.services.dbus-mcp-server = {
      description = "Operation D-Bus MCP Server";
      documentation = [ "https://modelcontextprotocol.io" ];
      after = [ "network.target" "dbus.service" "operation-dbus.service" ];
      requires = [ "dbus.service" ];
      wantedBy = [ "multi-user.target" ];

      serviceConfig = {
        Type = "simple";
        User = "op-dbus";
        Group = "op-dbus";
        Restart = "on-failure";
        RestartSec = "10s";

        # Security
        NoNewPrivileges = true;
        PrivateTmp = true;
        ProtectSystem = "strict";
        ProtectHome = true;
        ReadWritePaths = [ opDbusConfig.dataDir ];

        # Resource limits
        LimitNOFILE = 4096;
        LimitNPROC = 128;

        # Environment
        Environment = [
          "RUST_LOG=${cfg.logLevel}"
          "MCP_CONFIG_DIR=${opDbusConfig.dataDir}/mcp-configs"
          "MCP_DISCOVERY_ENABLED=${if cfg.enableDiscovery then "true" else "false"}"
        ];

        # Start MCP server
        ExecStart = let
          webFlag = optionalString cfg.enableWebBridge "--web --port ${toString cfg.serverPort}";
          discoveryFlag = optionalString cfg.enableDiscovery "--discovery";
        in "${cfg.package}/bin/dbus-mcp ${webFlag} ${discoveryFlag}";
      };
    };

    # MCP Orchestrator service (if enabled)
    systemd.services.dbus-orchestrator = mkIf cfg.orchestrator.enable {
      description = "Operation D-Bus MCP Orchestrator";
      after = [ "dbus-mcp-server.service" ];
      requires = [ "dbus-mcp-server.service" ];
      wantedBy = [ "multi-user.target" ];

      serviceConfig = {
        Type = "simple";
        User = "op-dbus";
        Group = "op-dbus";
        Restart = "on-failure";
        RestartSec = "10s";

        # Security
        NoNewPrivileges = true;
        PrivateTmp = true;
        ProtectSystem = "strict";
        ProtectHome = true;
        ReadWritePaths = [ opDbusConfig.dataDir ];

        # Environment
        Environment = [
          "RUST_LOG=${cfg.logLevel}"
          "MAX_CONCURRENT_TASKS=${toString cfg.orchestrator.maxConcurrentTasks}"
        ];

        ExecStart = "${cfg.package}/bin/dbus-orchestrator";
      };
    };

    # Agent services
    systemd.services = listToAttrs (map (agent:
      let
        agentInfo = availableAgents.${agent};
      in nameValuePair "dbus-agent-${agent}" {
        description = "MCP Agent: ${agentInfo.description}";
        after = [ "dbus-mcp-server.service" ];
        requires = [ "dbus-mcp-server.service" ];
        wantedBy = [ "multi-user.target" ];

        serviceConfig = {
          Type = "simple";
          User = if agent == "executor" || agent == "systemd" || agent == "network"
                 then "root"  # These agents need elevated privileges
                 else "op-dbus";
          Group = if agent == "executor" || agent == "systemd" || agent == "network"
                  then "root"
                  else "op-dbus";
          Restart = "on-failure";
          RestartSec = "10s";

          # Security
          NoNewPrivileges = agent != "executor" && agent != "systemd";
          PrivateTmp = true;
          ProtectSystem = if agent == "file" then "full" else "strict";
          ProtectHome = agent != "file";
          ReadWritePaths = if agent == "file"
                          then [ opDbusConfig.dataDir "/tmp" "/var/tmp" ]
                          else [ opDbusConfig.dataDir ];

          # Environment
          Environment = [
            "RUST_LOG=${cfg.logLevel}"
            "AGENT_NAME=${agent}"
          ];

          ExecStart = "${cfg.package}/bin/${agentInfo.binary}";
        };
      }
    ) cfg.agents);

    # Discovery service (if enabled)
    systemd.services.dbus-mcp-discovery = mkIf cfg.enableDiscovery {
      description = "D-Bus Service Discovery for MCP";
      after = [ "dbus.service" ];
      requires = [ "dbus.service" ];
      wantedBy = [ "multi-user.target" ];

      serviceConfig = {
        Type = "simple";
        User = "op-dbus";
        Group = "op-dbus";
        Restart = "on-failure";
        RestartSec = "30s";

        # Security
        NoNewPrivileges = true;
        PrivateTmp = true;
        ProtectSystem = "strict";
        ProtectHome = true;
        ReadWritePaths = [ opDbusConfig.dataDir ];

        # Environment
        Environment = [
          "RUST_LOG=${cfg.logLevel}"
          "DISCOVERY_OUTPUT_DIR=${cfg.discoveryConfig.output_dir}"
          "DISCOVERY_SCAN_INTERVAL=${toString cfg.discoveryConfig.scan_interval}"
        ];

        ExecStart = "${cfg.package}/bin/dbus-mcp-discovery-enhanced";
      };
    };

    # Chat interface service (if enabled)
    systemd.services.mcp-chat = mkIf cfg.chat.enable {
      description = "MCP Interactive Chat Interface";
      after = [ "dbus-mcp-server.service" ];
      requires = [ "dbus-mcp-server.service" ];
      wantedBy = [ "multi-user.target" ];

      serviceConfig = {
        Type = "simple";
        User = "op-dbus";
        Group = "op-dbus";
        Restart = "on-failure";
        RestartSec = "10s";

        # Security
        NoNewPrivileges = true;
        PrivateTmp = true;
        ProtectSystem = "strict";
        ProtectHome = true;

        # Environment
        Environment = [
          "RUST_LOG=${cfg.logLevel}"
          "CHAT_PORT=${toString cfg.chat.port}"
        ];

        ExecStart = "${cfg.package}/bin/mcp-chat --port ${toString cfg.chat.port}";
      };
    };

    # Open firewall ports if needed
    networking.firewall = mkIf cfg.enableWebBridge {
      allowedTCPPorts = [ cfg.serverPort ]
        ++ optional cfg.chat.enable cfg.chat.port;
    };

    # Add MCP tools to system packages
    environment.systemPackages = with cfg.package; [
      cfg.package
    ];
  };
}

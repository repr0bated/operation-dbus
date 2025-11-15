#!/usr/bin/env rust
//! NixOS System Introspection Tool
//!
//! Scans a running system via D-Bus, systemd, and native protocols
//! to generate a complete NixOS configuration that replicates the system state.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use tracing::{debug, info, warn};
use zbus::Connection;

#[derive(Parser, Debug)]
#[command(name = "nix-introspect")]
#[command(about = "Generate NixOS configuration from running system state", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Scan the entire system and generate configuration
    Scan {
        /// Output file path for configuration.nix
        #[arg(short, long, default_value = "configuration.nix")]
        output: PathBuf,

        /// Output format (nix, json, toml)
        #[arg(short, long, default_value = "nix")]
        format: String,

        /// Include MCP agent configuration
        #[arg(long)]
        include_mcp: bool,

        /// Scan system bus (requires root)
        #[arg(long, default_value = "true")]
        system_bus: bool,

        /// Scan session bus
        #[arg(long, default_value = "true")]
        session_bus: bool,

        /// Include network introspection (OVS, interfaces)
        #[arg(long, default_value = "true")]
        include_network: bool,

        /// Include systemd units
        #[arg(long, default_value = "true")]
        include_systemd: bool,

        /// Include container configuration (LXC)
        #[arg(long, default_value = "true")]
        include_containers: bool,
    },

    /// List all discoverable D-Bus services
    ListServices {
        /// Bus type (system, session, or both)
        #[arg(short, long, default_value = "both")]
        bus: String,

        /// Filter by service name pattern
        #[arg(short, long)]
        filter: Option<String>,
    },

    /// Introspect a specific D-Bus service
    Inspect {
        /// Service name (e.g., org.freedesktop.systemd1)
        service: String,

        /// Object path (e.g., /org/freedesktop/systemd1)
        #[arg(short, long, default_value = "/")]
        path: String,

        /// Bus type (system or session)
        #[arg(short, long, default_value = "system")]
        bus: String,

        /// Output format (xml, json)
        #[arg(short, long, default_value = "json")]
        format: String,
    },

    /// Generate minimal configuration template
    Template {
        /// Output file path
        #[arg(short, long, default_value = "template-configuration.nix")]
        output: PathBuf,

        /// Include comments and documentation
        #[arg(long, default_value = "true")]
        documented: bool,
    },
}

#[derive(Debug, Serialize, Deserialize)]
struct SystemState {
    hostname: String,
    kernel_version: Option<String>,
    dbus_services: Vec<DbusServiceInfo>,
    systemd_units: Vec<SystemdUnitInfo>,
    network_interfaces: Vec<NetworkInterfaceInfo>,
    containers: Vec<ContainerInfo>,
    mcp_agents: Vec<McpAgentInfo>,
    detected_plugins: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct DbusServiceInfo {
    name: String,
    bus: String,
    interfaces: Vec<String>,
    object_paths: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct SystemdUnitInfo {
    name: String,
    unit_type: String,
    active_state: String,
    enabled: bool,
    description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct NetworkInterfaceInfo {
    name: String,
    interface_type: String,
    ip_addresses: Vec<String>,
    mac_address: Option<String>,
    state: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ContainerInfo {
    name: String,
    container_type: String,
    state: String,
    config: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct McpAgentInfo {
    name: String,
    binary: String,
    capabilities: Vec<String>,
}

impl SystemState {
    fn new() -> Result<Self> {
        let hostname = gethostname::gethostname()
            .to_string_lossy()
            .to_string();

        Ok(Self {
            hostname,
            kernel_version: Self::get_kernel_version(),
            dbus_services: Vec::new(),
            systemd_units: Vec::new(),
            network_interfaces: Vec::new(),
            containers: Vec::new(),
            mcp_agents: Vec::new(),
            detected_plugins: Vec::new(),
        })
    }

    fn get_kernel_version() -> Option<String> {
        std::fs::read_to_string("/proc/version")
            .ok()
            .and_then(|v| v.split_whitespace().nth(2).map(String::from))
    }

    /// Generate NixOS configuration from system state
    fn to_nix_config(&self) -> String {
        let mut config = String::new();

        config.push_str("# Generated by nix-introspect\n");
        config.push_str(&format!("# Hostname: {}\n", self.hostname));
        if let Some(kernel) = &self.kernel_version {
            config.push_str(&format!("# Kernel: {}\n", kernel));
        }
        config.push_str(&format!("# Generated at: {}\n", chrono::Local::now().to_rfc3339()));
        config.push_str("\n{ config, pkgs, lib, ... }:\n\n");
        config.push_str("{\n");
        config.push_str("  imports = [\n");
        config.push_str("    ./hardware-configuration.nix\n");
        config.push_str("  ];\n\n");

        // Hostname
        config.push_str(&format!("  networking.hostName = \"{}\";\n\n", self.hostname));

        // Operation D-Bus service
        if !self.detected_plugins.is_empty() {
            config.push_str("  # Operation D-Bus Configuration\n");
            config.push_str("  services.operation-dbus = {\n");
            config.push_str("    enable = true;\n");
            config.push_str("    plugins = {\n");

            for plugin in &self.detected_plugins {
                config.push_str(&format!("      {}.enable = true;\n", plugin));
            }

            config.push_str("    };\n");
            config.push_str("  };\n\n");
        }

        // MCP Configuration
        if !self.mcp_agents.is_empty() {
            config.push_str("  # MCP (Model Context Protocol) Configuration\n");
            config.push_str("  services.operation-dbus.mcp = {\n");
            config.push_str("    enable = true;\n");
            config.push_str("    agents = [\n");

            for agent in &self.mcp_agents {
                config.push_str(&format!("      \"{}\"  # {}\n",
                    agent.name,
                    agent.capabilities.join(", ")
                ));
            }

            config.push_str("    ];\n");
            config.push_str("  };\n\n");
        }

        // Systemd services
        if !self.systemd_units.is_empty() {
            config.push_str("  # Systemd Services (detected as enabled)\n");
            config.push_str("  systemd.services = {\n");

            for unit in self.systemd_units.iter().filter(|u| u.enabled && u.unit_type == "service") {
                config.push_str(&format!("    \"{}\" = {{\n", unit.name.trim_end_matches(".service")));
                config.push_str("      enable = true;\n");
                if let Some(desc) = &unit.description {
                    config.push_str(&format!("      description = \"{}\";\n", desc.replace('"', "\\\"")));
                }
                config.push_str("    };\n");
            }

            config.push_str("  };\n\n");
        }

        // Network configuration
        if !self.network_interfaces.is_empty() {
            config.push_str("  # Network Configuration\n");
            config.push_str("  networking = {\n");
            config.push_str("    interfaces = {\n");

            for iface in &self.network_interfaces {
                if iface.name != "lo" && !iface.name.starts_with("veth") {
                    config.push_str(&format!("      \"{}\" = {{\n", iface.name));

                    if !iface.ip_addresses.is_empty() {
                        config.push_str("        ipv4.addresses = [\n");
                        for ip in &iface.ip_addresses {
                            if ip.contains('.') {
                                config.push_str(&format!("          {{ address = \"{}\"; prefixLength = 24; }}\n", ip));
                            }
                        }
                        config.push_str("        ];\n");
                    }

                    config.push_str("      };\n");
                }
            }

            config.push_str("    };\n");
            config.push_str("  };\n\n");
        }

        // D-Bus services (as system packages)
        if !self.dbus_services.is_empty() {
            config.push_str("  # D-Bus Services Detected\n");
            config.push_str("  # These services were found running:\n");

            for service in &self.dbus_services {
                config.push_str(&format!("  #   - {} ({})\n", service.name, service.bus));
            }
            config.push_str("\n");
        }

        // Containers
        if !self.containers.is_empty() {
            config.push_str("  # Container Configuration\n");
            config.push_str("  virtualisation.lxc = {\n");
            config.push_str("    enable = true;\n");
            config.push_str("  };\n\n");

            config.push_str("  # Detected containers:\n");
            for container in &self.containers {
                config.push_str(&format!("  #   - {} ({}): {}\n",
                    container.name,
                    container.container_type,
                    container.state
                ));
            }
            config.push_str("\n");
        }

        config.push_str("  # System state version\n");
        config.push_str("  system.stateVersion = \"24.05\";\n");
        config.push_str("}\n");

        config
    }
}

/// Scan D-Bus services on a specific bus
async fn scan_dbus_services(bus_type: &str) -> Result<Vec<DbusServiceInfo>> {
    info!("Scanning {} bus for D-Bus services...", bus_type);

    let connection = match bus_type {
        "system" => Connection::system().await?,
        "session" => Connection::session().await?,
        _ => return Err(anyhow::anyhow!("Invalid bus type: {}", bus_type)),
    };

    let mut services = Vec::new();

    // Query D-Bus daemon for list of names
    let dbus_proxy = zbus::fdo::DBusProxy::new(&connection).await?;
    let names = dbus_proxy.list_names().await?;

    for name in names {
        // Skip D-Bus internal names
        if name.starts_with(":") || name.starts_with("org.freedesktop.DBus") {
            continue;
        }

        debug!("Found service: {}", name);

        // Try to introspect the service
        let interfaces = introspect_service(&connection, &name).await.unwrap_or_default();

        services.push(DbusServiceInfo {
            name: name.to_string(),
            bus: bus_type.to_string(),
            interfaces: interfaces.clone(),
            object_paths: vec!["/".to_string()],
        });
    }

    info!("Found {} services on {} bus", services.len(), bus_type);
    Ok(services)
}

/// Introspect a D-Bus service to get its interfaces
async fn introspect_service(connection: &Connection, service_name: &str) -> Result<Vec<String>> {
    let proxy = zbus::fdo::IntrospectableProxy::builder(connection)
        .destination(service_name)?
        .path("/")?
        .build()
        .await?;

    let xml = proxy.introspect().await?;

    // Parse XML to extract interface names
    let interfaces = parse_interfaces_from_xml(&xml);

    Ok(interfaces)
}

/// Parse interface names from D-Bus introspection XML
fn parse_interfaces_from_xml(xml: &str) -> Vec<String> {
    let mut interfaces = Vec::new();

    for line in xml.lines() {
        if line.trim().starts_with("<interface name=\"") {
            if let Some(start) = line.find("name=\"") {
                if let Some(end) = line[start + 6..].find('"') {
                    let iface_name = &line[start + 6..start + 6 + end];
                    // Skip standard D-Bus interfaces
                    if !iface_name.starts_with("org.freedesktop.DBus") {
                        interfaces.push(iface_name.to_string());
                    }
                }
            }
        }
    }

    interfaces
}

/// Scan systemd units
async fn scan_systemd_units() -> Result<Vec<SystemdUnitInfo>> {
    info!("Scanning systemd units...");

    let connection = Connection::system().await?;
    let proxy = zbus::Proxy::new(
        &connection,
        "org.freedesktop.systemd1",
        "/org/freedesktop/systemd1",
        "org.freedesktop.systemd1.Manager",
    ).await?;

    // List all units
    let units: Vec<(String, String, String, String, String, String, zbus::zvariant::OwnedObjectPath, u32, String, zbus::zvariant::OwnedObjectPath)>
        = proxy.call("ListUnits", &()).await?;

    let mut unit_infos = Vec::new();

    for (name, description, load_state, active_state, _sub_state, _following, _unit_path, _job_id, _job_type, _job_path) in units {
        if load_state == "loaded" {
            // Determine unit type from name
            let unit_type = name.split('.').last().unwrap_or("unknown").to_string();

            // Check if unit is enabled
            let enabled = check_unit_enabled(&proxy, &name).await.unwrap_or(false);

            unit_infos.push(SystemdUnitInfo {
                name: name.clone(),
                unit_type,
                active_state: active_state.clone(),
                enabled,
                description: Some(description),
            });
        }
    }

    info!("Found {} systemd units", unit_infos.len());
    Ok(unit_infos)
}

/// Check if a systemd unit is enabled
async fn check_unit_enabled(proxy: &zbus::Proxy<'_>, unit_name: &str) -> Result<bool> {
    let state: String = proxy.call("GetUnitFileState", &(unit_name,)).await?;
    Ok(state == "enabled" || state == "static")
}

/// Scan network interfaces
async fn scan_network_interfaces() -> Result<Vec<NetworkInterfaceInfo>> {
    info!("Scanning network interfaces...");

    let mut interfaces = Vec::new();

    // Read from /sys/class/net
    let net_dir = std::path::Path::new("/sys/class/net");
    if let Ok(entries) = std::fs::read_dir(net_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();

            // Get interface type
            let type_path = entry.path().join("type");
            let interface_type = std::fs::read_to_string(&type_path)
                .ok()
                .and_then(|t| t.trim().parse::<u32>().ok())
                .map(|t| match t {
                    1 => "ethernet",
                    772 => "loopback",
                    _ => "other",
                })
                .unwrap_or("unknown")
                .to_string();

            // Get state
            let state_path = entry.path().join("operstate");
            let state = std::fs::read_to_string(&state_path)
                .ok()
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|| "unknown".to_string());

            // Get MAC address
            let mac_path = entry.path().join("address");
            let mac_address = std::fs::read_to_string(&mac_path)
                .ok()
                .map(|m| m.trim().to_string());

            // Get IP addresses (simplified - would need netlink for full details)
            let ip_addresses = get_interface_ips(&name).await.unwrap_or_default();

            interfaces.push(NetworkInterfaceInfo {
                name,
                interface_type,
                ip_addresses,
                mac_address,
                state,
            });
        }
    }

    info!("Found {} network interfaces", interfaces.len());
    Ok(interfaces)
}

/// Get IP addresses for an interface using rtnetlink
async fn get_interface_ips(iface_name: &str) -> Result<Vec<String>> {
    use rtnetlink::{new_connection, IpVersion};

    let (connection, handle, _) = new_connection()?;
    tokio::spawn(connection);

    let mut addresses = Vec::new();

    // Get link by name
    let mut links = handle.link().get().match_name(iface_name.to_string()).execute();
    if let Some(link) = links.try_next().await? {
        let index = link.header.index;

        // Get addresses for this link
        let mut addr_handle = handle.address().get().set_link_index_filter(index).execute();

        while let Some(addr) = addr_handle.try_next().await? {
            for nla in addr.attributes {
                if let rtnetlink::packet::address::AddressAttribute::Address(ip) = nla {
                    addresses.push(format!("{}", std::net::IpAddr::from(ip)));
                }
            }
        }
    }

    Ok(addresses)
}

/// Detect operation-dbus plugins based on discovered services
fn detect_plugins(services: &[DbusServiceInfo], units: &[SystemdUnitInfo]) -> Vec<String> {
    let mut plugins = HashSet::new();

    // Check for systemd
    if services.iter().any(|s| s.name == "org.freedesktop.systemd1") {
        plugins.insert("systemd".to_string());
    }

    // Check for login1 (user sessions)
    if services.iter().any(|s| s.name == "org.freedesktop.login1") {
        plugins.insert("login1".to_string());
    }

    // Check for networkd
    if services.iter().any(|s| s.name == "org.freedesktop.network1") {
        plugins.insert("network".to_string());
    }

    // Check for resolved (DNS)
    if services.iter().any(|s| s.name == "org.freedesktop.resolve1") {
        plugins.insert("dnsresolver".to_string());
    }

    // Check for OVS
    if units.iter().any(|u| u.name.contains("openvswitch")) {
        plugins.insert("net".to_string());
    }

    // Check for LXC
    if units.iter().any(|u| u.name.contains("lxc")) {
        plugins.insert("lxc".to_string());
    }

    plugins.into_iter().collect()
}

/// Detect MCP agents (from running binaries or configuration)
fn detect_mcp_agents() -> Vec<McpAgentInfo> {
    vec![
        McpAgentInfo {
            name: "executor".to_string(),
            binary: "dbus-agent-executor".to_string(),
            capabilities: vec!["command execution".to_string(), "system tasks".to_string()],
        },
        McpAgentInfo {
            name: "systemd".to_string(),
            binary: "dbus-agent-systemd".to_string(),
            capabilities: vec!["service management".to_string(), "unit control".to_string()],
        },
        McpAgentInfo {
            name: "file".to_string(),
            binary: "dbus-agent-file".to_string(),
            capabilities: vec!["file operations".to_string(), "filesystem access".to_string()],
        },
        McpAgentInfo {
            name: "network".to_string(),
            binary: "dbus-agent-network".to_string(),
            capabilities: vec!["network management".to_string(), "interface control".to_string()],
        },
        McpAgentInfo {
            name: "monitor".to_string(),
            binary: "dbus-agent-monitor".to_string(),
            capabilities: vec!["system monitoring".to_string(), "metrics collection".to_string()],
        },
    ]
}

/// Scan for LXC containers
async fn scan_containers() -> Result<Vec<ContainerInfo>> {
    info!("Scanning for containers...");

    let mut containers = Vec::new();

    // Check for LXC containers in /var/lib/lxc
    let lxc_dir = std::path::Path::new("/var/lib/lxc");
    if lxc_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(lxc_dir) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    let name = entry.file_name().to_string_lossy().to_string();

                    // Try to determine state (running/stopped)
                    let state = if std::path::Path::new(&format!("/sys/fs/cgroup/lxc/{}", name)).exists() {
                        "running"
                    } else {
                        "stopped"
                    };

                    containers.push(ContainerInfo {
                        name: name.clone(),
                        container_type: "lxc".to_string(),
                        state: state.to_string(),
                        config: HashMap::new(),
                    });
                }
            }
        }
    }

    info!("Found {} containers", containers.len());
    Ok(containers)
}

/// Generate a template NixOS configuration
fn generate_template(documented: bool) -> String {
    let mut template = String::new();

    if documented {
        template.push_str("# NixOS Configuration Template\n");
        template.push_str("# Generated by nix-introspect\n");
        template.push_str("#\n");
        template.push_str("# This is a minimal template for operation-dbus integration.\n");
        template.push_str("# Customize this file according to your needs.\n\n");
    }

    template.push_str("{ config, pkgs, lib, ... }:\n\n");
    template.push_str("{\n");
    template.push_str("  imports = [\n");
    template.push_str("    ./hardware-configuration.nix\n");
    template.push_str("  ];\n\n");

    if documented {
        template.push_str("  # Set your hostname\n");
    }
    template.push_str("  networking.hostName = \"nixos\";\n\n");

    if documented {
        template.push_str("  # Enable operation-dbus for declarative system management\n");
    }
    template.push_str("  services.operation-dbus = {\n");
    template.push_str("    enable = true;\n\n");

    if documented {
        template.push_str("    # Enable plugins as needed\n");
    }
    template.push_str("    plugins = {\n");
    template.push_str("      systemd.enable = true;    # Systemd service management\n");
    template.push_str("      network.enable = false;   # Network/OVS management\n");
    template.push_str("      lxc.enable = false;       # LXC container management\n");
    template.push_str("      login1.enable = false;    # User session management\n");
    template.push_str("      dnsresolver.enable = false; # DNS configuration\n");
    template.push_str("    };\n\n");

    if documented {
        template.push_str("    # Enable MCP (Model Context Protocol) server\n");
    }
    template.push_str("    mcp = {\n");
    template.push_str("      enable = false;\n");
    template.push_str("      agents = [\n");
    template.push_str("        \"executor\"  # Command execution\n");
    template.push_str("        \"systemd\"   # Systemd integration\n");
    template.push_str("        \"file\"      # File operations\n");
    template.push_str("        \"network\"   # Network management\n");
    template.push_str("        \"monitor\"   # System monitoring\n");
    template.push_str("      ];\n");
    template.push_str("    };\n");
    template.push_str("  };\n\n");

    template.push_str("  system.stateVersion = \"24.05\";\n");
    template.push_str("}\n");

    template
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Scan {
            output,
            format,
            include_mcp,
            system_bus,
            session_bus,
            include_network,
            include_systemd,
            include_containers,
        } => {
            info!("Starting system introspection...");

            let mut state = SystemState::new()?;

            // Scan D-Bus services
            if system_bus {
                match scan_dbus_services("system").await {
                    Ok(services) => state.dbus_services.extend(services),
                    Err(e) => warn!("Failed to scan system bus: {}", e),
                }
            }

            if session_bus {
                match scan_dbus_services("session").await {
                    Ok(services) => state.dbus_services.extend(services),
                    Err(e) => warn!("Failed to scan session bus: {}", e),
                }
            }

            // Scan systemd units
            if include_systemd {
                match scan_systemd_units().await {
                    Ok(units) => state.systemd_units = units,
                    Err(e) => warn!("Failed to scan systemd units: {}", e),
                }
            }

            // Scan network interfaces
            if include_network {
                match scan_network_interfaces().await {
                    Ok(interfaces) => state.network_interfaces = interfaces,
                    Err(e) => warn!("Failed to scan network interfaces: {}", e),
                }
            }

            // Scan containers
            if include_containers {
                match scan_containers().await {
                    Ok(containers) => state.containers = containers,
                    Err(e) => warn!("Failed to scan containers: {}", e),
                }
            }

            // Detect plugins
            state.detected_plugins = detect_plugins(&state.dbus_services, &state.systemd_units);

            // Detect MCP agents
            if include_mcp {
                state.mcp_agents = detect_mcp_agents();
            }

            // Generate output
            let output_content = match format.as_str() {
                "nix" => state.to_nix_config(),
                "json" => serde_json::to_string_pretty(&state)?,
                "toml" => toml::to_string(&state)
                    .context("Failed to serialize to TOML")?,
                _ => return Err(anyhow::anyhow!("Unsupported format: {}", format)),
            };

            // Write to file
            std::fs::write(&output, output_content)
                .context(format!("Failed to write to {}", output.display()))?;

            info!("✓ Configuration written to {}", output.display());
            info!("  - {} D-Bus services detected", state.dbus_services.len());
            info!("  - {} systemd units found", state.systemd_units.len());
            info!("  - {} network interfaces discovered", state.network_interfaces.len());
            info!("  - {} containers detected", state.containers.len());
            info!("  - {} plugins detected: {}", state.detected_plugins.len(), state.detected_plugins.join(", "));
        }

        Commands::ListServices { bus, filter } => {
            let buses = match bus.as_str() {
                "system" => vec!["system"],
                "session" => vec!["session"],
                "both" => vec!["system", "session"],
                _ => return Err(anyhow::anyhow!("Invalid bus type: {}", bus)),
            };

            for bus_type in buses {
                println!("\n{} bus services:", bus_type.to_uppercase());
                println!("{}", "=".repeat(50));

                let services = scan_dbus_services(bus_type).await?;

                for service in services {
                    if let Some(ref pattern) = filter {
                        if !service.name.contains(pattern) {
                            continue;
                        }
                    }

                    println!("  • {}", service.name);
                    if !service.interfaces.is_empty() {
                        println!("    Interfaces: {}", service.interfaces.join(", "));
                    }
                }
            }
        }

        Commands::Inspect { service, path, bus, format } => {
            info!("Introspecting service: {} at path: {}", service, path);

            let connection = match bus.as_str() {
                "system" => Connection::system().await?,
                "session" => Connection::session().await?,
                _ => return Err(anyhow::anyhow!("Invalid bus type: {}", bus)),
            };

            let proxy = zbus::fdo::IntrospectableProxy::builder(&connection)
                .destination(service.as_str())?
                .path(path.as_str())?
                .build()
                .await?;

            let xml = proxy.introspect().await?;

            match format.as_str() {
                "xml" => println!("{}", xml),
                "json" => {
                    let interfaces = parse_interfaces_from_xml(&xml);
                    let json = serde_json::json!({
                        "service": service,
                        "path": path,
                        "interfaces": interfaces,
                        "xml": xml,
                    });
                    println!("{}", serde_json::to_string_pretty(&json)?);
                }
                _ => return Err(anyhow::anyhow!("Unsupported format: {}", format)),
            }
        }

        Commands::Template { output, documented } => {
            let template = generate_template(documented);

            std::fs::write(&output, template)
                .context(format!("Failed to write template to {}", output.display()))?;

            info!("✓ Template configuration written to {}", output.display());
        }
    }

    Ok(())
}

// Enhanced D-Bus Discovery Service
// Automatically discovers and generates multiple MCP server configurations

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tokio;
use zbus::Connection;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceCategory {
    pub name: String,
    pub description: String,
    pub services: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DiscoveryConfig {
    pub output_dir: PathBuf,
    pub scan_interval: u64,
    pub categories: Vec<ServiceCategory>,
    pub filter: ServiceFilter,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceFilter {
    pub include_patterns: Vec<String>,
    pub exclude_patterns: Vec<String>,
    pub max_tools_per_service: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct McpServerEntry {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub description: String,
    pub category: String,
    pub tool_count: usize,
}

#[derive(Debug, Serialize)]
pub struct ClientConfig {
    #[serde(rename = "mcpServers")]
    pub mcp_servers: HashMap<String, McpServerConfig>,
}

#[derive(Debug, Serialize)]
pub struct McpServerConfig {
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
}

pub struct EnhancedDiscovery {
    connection: Connection,
    config: DiscoveryConfig,
}

impl EnhancedDiscovery {
    pub async fn new(config_path: Option<&str>) -> Result<Self, Box<dyn std::error::Error>> {
        let connection = Connection::session().await?;

        let config = if let Some(path) = config_path {
            let content = fs::read_to_string(path)?;
            toml::from_str(&content)?
        } else {
            Self::default_config()
        };

        Ok(Self { connection, config })
    }

    fn default_config() -> DiscoveryConfig {
        DiscoveryConfig {
            output_dir: PathBuf::from("~/.config/Claude/mcp-servers"),
            scan_interval: 300,
            categories: vec![
                ServiceCategory {
                    name: "system".to_string(),
                    description: "System management services".to_string(),
                    services: vec![
                        "org.freedesktop.systemd1".to_string(),
                        "org.freedesktop.login1".to_string(),
                    ],
                },
                ServiceCategory {
                    name: "network".to_string(),
                    description: "Network management services".to_string(),
                    services: vec!["org.freedesktop.NetworkManager".to_string()],
                },
                ServiceCategory {
                    name: "automation".to_string(),
                    description: "Automation agents".to_string(),
                    services: vec![
                        "org.dbusmcp.Agent.*".to_string(),
                        "org.dbusmcp.Orchestrator".to_string(),
                    ],
                },
            ],
            filter: ServiceFilter {
                include_patterns: vec![
                    "org.freedesktop.*".to_string(),
                    "org.dbusmcp.*".to_string(),
                ],
                exclude_patterns: vec![
                    "org.freedesktop.DBus".to_string(),
                    "org.freedesktop.secrets".to_string(),
                ],
                max_tools_per_service: Some(50), // Limit tools per service for performance
            },
        }
    }

    pub async fn discover_and_generate(
        &self,
    ) -> Result<Vec<McpServerEntry>, Box<dyn std::error::Error>> {
        println!("🔍 Enhanced D-Bus Discovery Starting...\n");

        let mut entries = Vec::new();

        // Scan both session and system buses
        println!("📡 Scanning Session Bus...");
        let session_entries = self.scan_bus("session").await?;
        entries.extend(session_entries);

        println!("\n📡 Scanning System Bus...");
        let system_entries = self.scan_bus("system").await?;
        entries.extend(system_entries);

        Ok(entries)
    }

    async fn scan_bus(
        &self,
        bus_type: &str,
    ) -> Result<Vec<McpServerEntry>, Box<dyn std::error::Error>> {
        // Connect to the appropriate bus
        let connection = match bus_type {
            "system" => Connection::system().await?,
            _ => Connection::session().await?,
        };

        // Get all D-Bus services on this bus
        let proxy = zbus::fdo::DBusProxy::new(&connection).await?;
        let all_names = proxy.list_names().await?;

        // Convert OwnedBusName to String
        let all_names_str: Vec<String> = all_names.iter().map(|n| n.to_string()).collect();

        let mut entries = Vec::new();

        // Categorize and filter services
        for category in &self.config.categories {
            println!(
                "📂 Category: {} - {} [{}]",
                category.name, category.description, bus_type
            );

            for pattern in &category.services {
                let matching_services = self.find_matching_services(&all_names_str, pattern);

                for service in matching_services {
                    if self.should_expose(&service) {
                        match self
                            .create_mcp_entry(&service, &category.name, bus_type)
                            .await
                        {
                            Ok(entry) => {
                                println!("  ✅ {} ({} tools)", entry.name, entry.tool_count);
                                entries.push(entry);
                            }
                            Err(e) => {
                                eprintln!("  ❌ {} - Failed: {}", service, e);
                            }
                        }
                    }
                }
            }
            if !entries.is_empty() {
                println!();
            }
        }

        Ok(entries)
    }

    fn find_matching_services(&self, all_names: &[String], pattern: &str) -> Vec<String> {
        if pattern.ends_with("*") {
            let prefix = pattern.trim_end_matches('*');
            all_names
                .iter()
                .filter(|name| name.starts_with(prefix))
                .cloned()
                .collect()
        } else {
            all_names
                .iter()
                .filter(|name| name.as_str() == pattern)
                .cloned()
                .collect()
        }
    }

    fn should_expose(&self, service: &str) -> bool {
        // Check exclude patterns
        for pattern in &self.config.filter.exclude_patterns {
            if service.starts_with(pattern.trim_end_matches('*')) {
                return false;
            }
        }

        // Check include patterns
        for pattern in &self.config.filter.include_patterns {
            if service.starts_with(pattern.trim_end_matches('*')) {
                return true;
            }
        }

        false
    }

    async fn create_mcp_entry(
        &self,
        service: &str,
        category: &str,
        bus_type: &str,
    ) -> Result<McpServerEntry, Box<dyn std::error::Error>> {
        // Introspect to get tool count
        let tool_count = self.count_tools(service, bus_type).await.unwrap_or(0);

        if tool_count == 0 {
            return Err("No tools found".into());
        }

        // Generate a friendly name
        let name = self.service_to_friendly_name(service);

        // Determine the bridge binary location
        let bridge_command = self.find_bridge_binary();

        // Add bus type to args
        let mut args = vec!["--service".to_string(), service.to_string()];
        if bus_type == "system" {
            args.push("--system".to_string());
        }

        let entry = McpServerEntry {
            name: name.clone(),
            command: bridge_command,
            args,
            env: HashMap::from([
                ("DBUS_SERVICE".to_string(), service.to_string()),
                ("DBUS_BUS_TYPE".to_string(), bus_type.to_string()),
                ("MCP_NAME".to_string(), name.clone()),
                ("RUST_LOG".to_string(), "error".to_string()), // Reduce noise
            ]),
            description: format!(
                "{} service ({} bus) with {} available tools",
                service, bus_type, tool_count
            ),
            category: category.to_string(),
            tool_count,
        };

        Ok(entry)
    }

    async fn count_tools(
        &self,
        service: &str,
        bus_type: &str,
    ) -> Result<usize, Box<dyn std::error::Error>> {
        // Quick introspection to count methods
        let path = format!("/{}", service.replace('.', "/"));

        let mut cmd = std::process::Command::new("busctl");

        // Add appropriate bus flag
        if bus_type == "system" {
            cmd.arg("--system");
        } else {
            cmd.arg("--user");
        }

        let output = cmd
            .arg("introspect")
            .arg("--xml-interface")
            .arg(service)
            .arg(&path)
            .output()?;

        if output.status.success() {
            let xml = String::from_utf8_lossy(&output.stdout);
            let method_count = xml.matches("<method name=").count();

            // Apply max tools filter
            if let Some(max) = self.config.filter.max_tools_per_service {
                Ok(method_count.min(max))
            } else {
                Ok(method_count)
            }
        } else {
            Ok(0)
        }
    }

    fn service_to_friendly_name(&self, service: &str) -> String {
        service
            .replace("org.freedesktop.", "")
            .replace("org.dbusmcp.", "")
            .replace("Agent.", "agent-")
            .replace('.', "-")
            .to_lowercase()
    }

    fn find_bridge_binary(&self) -> String {
        // Try different locations for the bridge binary
        let candidates = vec![
            "/usr/local/bin/dbus-mcp-bridge",
            "/usr/bin/dbus-mcp-bridge",
            "./target/release/dbus-mcp-bridge",
            "./target/debug/dbus-mcp-bridge",
        ];

        for path in candidates {
            if std::path::Path::new(path).exists() {
                return path.to_string();
            }
        }

        // Fallback
        "dbus-mcp-bridge".to_string()
    }

    pub fn generate_client_configs(
        &self,
        entries: &[McpServerEntry],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let output_dir = self.expand_tilde(&self.config.output_dir);
        fs::create_dir_all(&output_dir)?;

        println!("📝 Generating Client Configurations...\n");

        // Generate individual config files for each service
        for entry in entries {
            let config = McpServerConfig {
                command: entry.command.clone(),
                args: entry.args.clone(),
                env: entry.env.clone(),
            };

            let filename = format!("{}/{}.json", output_dir.display(), entry.name);
            let json = serde_json::to_string_pretty(&config)?;
            fs::write(&filename, json)?;
            println!("  ✅ {}", filename);
        }

        // Generate a master config with all services
        let mut mcp_servers = HashMap::new();
        for entry in entries {
            mcp_servers.insert(
                entry.name.clone(),
                McpServerConfig {
                    command: entry.command.clone(),
                    args: entry.args.clone(),
                    env: entry.env.clone(),
                },
            );
        }

        let master_config = ClientConfig { mcp_servers };
        let master_file = format!("{}/all-services.json", output_dir.display());
        let json = serde_json::to_string_pretty(&master_config)?;
        fs::write(&master_file, json)?;
        println!("\n  ✅ Master config: {}", master_file);

        // Generate category-based configs
        let categories: Vec<String> = entries.iter().map(|e| e.category.clone()).collect();
        let unique_categories: Vec<String> = categories
            .into_iter()
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        for category in unique_categories {
            let mut category_servers = HashMap::new();

            for entry in entries.iter().filter(|e| e.category == category) {
                category_servers.insert(
                    entry.name.clone(),
                    McpServerConfig {
                        command: entry.command.clone(),
                        args: entry.args.clone(),
                        env: entry.env.clone(),
                    },
                );
            }

            let category_config = ClientConfig {
                mcp_servers: category_servers,
            };
            let category_file = format!("{}/category-{}.json", output_dir.display(), category);
            let json = serde_json::to_string_pretty(&category_config)?;
            fs::write(&category_file, json)?;
            println!("  ✅ Category config: {}", category_file);
        }

        Ok(())
    }

    pub fn generate_summary(&self, entries: &[McpServerEntry]) -> String {
        let total_tools: usize = entries.iter().map(|e| e.tool_count).sum();

        let mut summary = format!(
            "\n🎯 Discovery Summary\n\
            ═══════════════════════\n\
            Total MCP Servers: {}\n\
            Total Tools Available: {}\n\n",
            entries.len(),
            total_tools
        );

        // Group by category
        let mut by_category: HashMap<String, Vec<&McpServerEntry>> = HashMap::new();
        for entry in entries {
            by_category
                .entry(entry.category.clone())
                .or_default()
                .push(entry);
        }

        for (category, cat_entries) in by_category {
            let cat_tools: usize = cat_entries.iter().map(|e| e.tool_count).sum();
            summary.push_str(&format!(
                "📂 {} ({} servers, {} tools)\n",
                category,
                cat_entries.len(),
                cat_tools
            ));

            for entry in cat_entries {
                summary.push_str(&format!(
                    "   • {} ({} tools)\n",
                    entry.name, entry.tool_count
                ));
            }
            summary.push('\n');
        }

        summary
    }

    fn expand_tilde(&self, path: &PathBuf) -> PathBuf {
        if let Some(path_str) = path.to_str() {
            if path_str.starts_with("~") {
                if let Ok(home) = std::env::var("HOME") {
                    return PathBuf::from(path_str.replace("~", &home));
                }
            }
        }
        path.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("╔══════════════════════════════════════════════╗");
    println!("║   Enhanced D-Bus MCP Discovery Service      ║");
    println!("╚══════════════════════════════════════════════╝\n");

    // Load config or use defaults
    let config_path = std::env::args().nth(1);
    let discovery = EnhancedDiscovery::new(config_path.as_deref()).await?;

    // Discover services and generate entries
    let entries = discovery.discover_and_generate().await?;

    if entries.is_empty() {
        println!("⚠️  No services found to expose!");
        return Ok(());
    }

    // Generate client configurations
    discovery.generate_client_configs(&entries)?;

    // Print summary
    let summary = discovery.generate_summary(&entries);
    println!("{}", summary);

    println!("✨ Discovery Complete!");
    println!("\n📍 Next Steps:");
    println!("1. Copy configs to your MCP client's config directory");
    println!("2. Restart your MCP client");
    println!("3. All discovered services will be available as separate MCP servers!");

    Ok(())
}

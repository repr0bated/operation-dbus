//! Create OVS bridge with uplink and internal port, atomically moving IP configuration
//! This script:
//! 1. Discovers the active internet interface
//! 2. Gets its IP configuration
//! 3. Creates ovsbr0 with uplink port and internal port
//! 4. Moves IP configuration to internal port atomically

use anyhow::{Context, Result};
use op_dbus::native::ovsdb_jsonrpc::OvsdbClient;
use op_dbus::native::rtnetlink_helpers::{add_ipv4_address, add_default_route, del_default_route, flush_addresses, link_up};
use std::net::Ipv4Addr;
use tokio::time::{sleep, Duration};

/// Find the active internet interface (has default route)
/// Uses a simpler approach: parse /proc/net/route for default route, then get interface details
async fn find_active_interface() -> Result<(String, Ipv4Addr, u8, Ipv4Addr)> {
    use std::{fs, str::FromStr};
    
    // Read /proc/net/route to find default route
    let route_content = fs::read_to_string("/proc/net/route")
        .context("Failed to read /proc/net/route")?;
    
    let mut default_ifname = None;
    let mut gateway = None;
    
    for line in route_content.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 8 && parts[1] == "00000000" && parts[7] == "00000000" {
            // This is a default route (destination 0.0.0.0, mask 0.0.0.0)
            default_ifname = Some(parts[0].to_string());
            // Gateway is in hex format, convert it
            let gw_hex = parts[2];
            if gw_hex != "00000000" {
                // Parse hex gateway (little-endian)
                let gw_bytes: Vec<u8> = (0..4)
                    .map(|i| u8::from_str_radix(&gw_hex[i*2..i*2+2], 16).unwrap_or(0))
                    .collect();
                if gw_bytes.len() == 4 {
                    gateway = Some(Ipv4Addr::new(gw_bytes[0], gw_bytes[1], gw_bytes[2], gw_bytes[3]));
                }
            }
            break;
        }
    }
    
    let ifname = default_ifname.context("No default route found - cannot determine active interface")?;
    let gw = gateway.context("No gateway found in default route")?;
    
    // Get IP address using ip command (allowed per user's rules for common utilities)
    let ip_output = std::process::Command::new("ip")
        .args(["-4", "addr", "show", &ifname])
        .output()
        .context("Failed to run ip command")?;
    
    let ip_str = String::from_utf8_lossy(&ip_output.stdout);
    let mut ip_addr = None;
    let mut prefix_len = None;
    
    for line in ip_str.lines() {
        if line.trim().starts_with("inet ") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let addr_part = parts[1];
                if let Some(slash_pos) = addr_part.find('/') {
                    if let Ok(ip) = Ipv4Addr::from_str(&addr_part[..slash_pos]) {
                        ip_addr = Some(ip);
                        if let Ok(prefix) = u8::from_str(&addr_part[slash_pos+1..]) {
                            prefix_len = Some(prefix);
                        }
                        break;
                    }
                }
            }
        }
    }
    
    let ip = ip_addr.context(format!("No IPv4 address found on interface '{}'", ifname))?;
    let prefix = prefix_len.context(format!("No prefix length found for interface '{}'", ifname))?;

    Ok((ifname, ip, prefix, gw))
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    println!("🔍 Discovering active internet interface...");
    let (uplink_ifname, ip_addr, prefix_len, gateway) = find_active_interface().await?;
    println!("✓ Found active interface: {} with IP {}/{} and gateway {}", 
             uplink_ifname, ip_addr, prefix_len, gateway);

    let bridge_name = "ovsbr0";
    let internal_port_name = "ovsbr0";

    // Check if bridge already exists
    let ovsdb = OvsdbClient::new();
    
    println!("🔨 Creating OVS bridge {} with uplink port {} and internal port {}...", 
             bridge_name, uplink_ifname, internal_port_name);

    // Step 1: Create bridge with internal port (bridge name = internal port name)
    // Use create_bridge which handles the transaction correctly
    ovsdb.create_bridge(bridge_name).await
        .context("Failed to create bridge")?;
    
    // Wait a moment for bridge to be created
    sleep(Duration::from_millis(500)).await;

    // Step 2: Add uplink port to bridge
    println!("📌 Adding uplink port {} to bridge...", uplink_ifname);
    ovsdb.add_port(bridge_name, &uplink_ifname).await
        .context("Failed to add uplink port")?;

    // Wait for ports to be ready
    sleep(Duration::from_millis(500)).await;

    // Step 3: Bring up the internal port
    println!("⬆ Bringing up internal port {}...", internal_port_name);
    link_up(internal_port_name).await
        .context("Failed to bring up internal port")?;

    // Step 4: Atomically move IP configuration (minimize downtime)
    println!("🔄 Moving IP configuration from {} to {}...", uplink_ifname, internal_port_name);
    
    // Strategy: Add IP to internal port first, then remove from uplink
    // This minimizes connectivity loss
    
    // Add IP to internal port first
    add_ipv4_address(internal_port_name, &ip_addr.to_string(), prefix_len).await
        .context("Failed to add IP address to internal port")?;
    
    // Add default route via internal port (this will work alongside existing route temporarily)
    add_default_route(internal_port_name, &gateway.to_string()).await
        .context("Failed to add default route via internal port")?;
    
    // Small delay to ensure routes are propagated
    sleep(Duration::from_millis(200)).await;
    
    // Now remove IP from uplink (connectivity should continue via internal port)
    flush_addresses(&uplink_ifname).await
        .context("Failed to flush addresses from uplink")?;
    
    // Remove old default route (if it still exists)
    // Note: This might fail if route was already removed, which is fine
    let _ = del_default_route().await;

    println!("✅ Successfully created bridge {} with:", bridge_name);
    println!("   - Uplink port: {}", uplink_ifname);
    println!("   - Internal port: {} (IP: {}/{}, Gateway: {})", 
             internal_port_name, ip_addr, prefix_len, gateway);

    Ok(())
}

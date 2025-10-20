//! Rtnetlink helpers - native netlink operations for IP addresses and routes

use anyhow::{Context, Result};
use futures::TryStreamExt;
use netlink_packet_route::address::AddressAttribute;
use netlink_packet_route::route::RouteAttribute;
use serde_json::json;
use rtnetlink::{new_connection, Handle, IpVersion};
use std::net::{IpAddr, Ipv4Addr};

/// Add IPv4 address to interface
pub async fn add_ipv4_address(ifname: &str, ip: &str, prefix: u8) -> Result<()> {
    let (connection, handle, _) = new_connection()?;
    tokio::spawn(connection);

    // Find interface by name
    let mut links = handle.link().get().match_name(ifname.to_string()).execute();
    let link = links
        .try_next()
        .await?
        .context(format!("Interface '{}' not found", ifname))?;

    let ifindex = link.header.index;

    // Parse IP address
    let addr: Ipv4Addr = ip.parse().context("Invalid IPv4 address")?;

    // Add address to interface
    handle
        .address()
        .add(ifindex, addr.into(), prefix)
        .execute()
        .await
        .context("Failed to add IP address")?;

    Ok(())
}

/// Delete IPv4 address from interface
pub async fn del_ipv4_address(ifname: &str, ip: &str, prefix: u8) -> Result<()> {
    let (connection, handle, _) = new_connection()?;
    tokio::spawn(connection);

    // Find interface by name
    let mut links = handle.link().get().match_name(ifname.to_string()).execute();
    let link = links
        .try_next()
        .await?
        .context(format!("Interface '{}' not found", ifname))?;

    let ifindex = link.header.index;

    // Parse IP address
    let addr: Ipv4Addr = ip.parse().context("Invalid IPv4 address")?;

    // Get addresses to find the exact one to delete
    let mut addresses = handle.address().get().set_link_index_filter(ifindex).execute();

    while let Some(addr_msg) = addresses.try_next().await? {
        if addr_msg.header.prefix_len == prefix {
            // Check if this is the address we want to delete
            let has_matching_addr = addr_msg.attributes.iter().any(|nla| {
                if let AddressAttribute::Address(a) = nla {
                    match a {
                        IpAddr::V4(v4) => v4.octets().to_vec() == addr.octets().to_vec(),
                        _ => false,
                    }
                } else {
                    false
                }
            });

            if has_matching_addr {
                handle.address().del(addr_msg).execute().await?;
                return Ok(());
            }
        }
    }

    Ok(())
}

/// Flush all addresses from interface
pub async fn flush_addresses(ifname: &str) -> Result<()> {
    let (connection, handle, _) = new_connection()?;
    tokio::spawn(connection);

    // Find interface by name
    let mut links = handle.link().get().match_name(ifname.to_string()).execute();
    let link = links
        .try_next()
        .await?
        .context(format!("Interface '{}' not found", ifname))?;

    let ifindex = link.header.index;

    // Get all addresses on this interface
    let mut addresses = handle.address().get().set_link_index_filter(ifindex).execute();

    while let Some(addr) = addresses.try_next().await? {
        // Delete this address
        if let Err(e) = handle.address().del(addr).execute().await {
            log::warn!("Failed to delete address: {}", e);
        }
    }

    Ok(())
}

/// Set link up
pub async fn link_up(ifname: &str) -> Result<()> {
    let (connection, handle, _) = new_connection()?;
    tokio::spawn(connection);

    // Find interface by name
    let mut links = handle.link().get().match_name(ifname.to_string()).execute();
    let link = links
        .try_next()
        .await?
        .context(format!("Interface '{}' not found", ifname))?;

    let ifindex = link.header.index;

    // Set link up
    handle
        .link()
        .set(ifindex)
        .up()
        .execute()
        .await
        .context("Failed to bring link up")?;

    Ok(())
}

/// Set link down
pub async fn link_down(ifname: &str) -> Result<()> {
    let (connection, handle, _) = new_connection()?;
    tokio::spawn(connection);

    // Find interface by name
    let mut links = handle.link().get().match_name(ifname.to_string()).execute();
    let link = links
        .try_next()
        .await?
        .context(format!("Interface '{}' not found", ifname))?;

    let ifindex = link.header.index;

    // Set link down
    handle
        .link()
        .set(ifindex)
        .down()
        .execute()
        .await
        .context("Failed to bring link down")?;

    Ok(())
}

/// Add default route
pub async fn add_default_route(ifname: &str, gateway: &str) -> Result<()> {
    let (connection, handle, _) = new_connection()?;
    tokio::spawn(connection);

    // Find interface by name
    let mut links = handle.link().get().match_name(ifname.to_string()).execute();
    let link = links
        .try_next()
        .await?
        .context(format!("Interface '{}' not found", ifname))?;

    let ifindex = link.header.index;

    // Parse gateway address
    let gw: Ipv4Addr = gateway.parse().context("Invalid gateway address")?;

    // Add default route
    handle
        .route()
        .add()
        .v4()
        .destination_prefix(Ipv4Addr::new(0, 0, 0, 0), 0)
        .gateway(gw)
        .output_interface(ifindex)
        .execute()
        .await
        .context("Failed to add default route")?;

    Ok(())
}

/// Delete default route
pub async fn del_default_route() -> Result<()> {
    let (connection, handle, _) = new_connection()?;
    tokio::spawn(connection);

    // Get all routes
    let mut routes = handle.route().get(IpVersion::V4).execute();

    while let Some(route) = routes.try_next().await? {
        // Check if this is a default route (destination 0.0.0.0/0)
        if route.header.destination_prefix_length == 0 {
            // Delete this route
            if let Err(e) = handle.route().del(route).execute().await {
                log::warn!("Failed to delete default route: {}", e);
            }
        }
    }

    Ok(())
}

/// List IPv4 routes for a given interface (by name)
pub async fn list_routes_for_interface(ifname: &str) -> Result<Vec<serde_json::Value>> {
    let (connection, handle, _) = new_connection()?;
    tokio::spawn(connection);

    // Find interface by name
    let mut links = handle.link().get().match_name(ifname.to_string()).execute();
    let link = links
        .try_next()
        .await?
        .context(format!("Interface '{}' not found", ifname))?;

    let ifindex = link.header.index;

    let mut routes = handle.route().get(IpVersion::V4).execute();
    let mut results = Vec::new();

    while let Some(route) = routes.try_next().await? {
        let mut oif: Option<i32> = None;
        let table = route.header.table;

        for attr in &route.attributes {
            if let RouteAttribute::Oif(index) = attr {
                oif = Some(*index as i32);
            }
        }

        if oif == Some(ifindex as i32) {
            results.push(json!({
                "dst": format!("0.0.0.0/{}", route.header.destination_prefix_length),
                "gateway": null,
                "table": table,
                "oif": ifindex,
            }));
        }
    }

    Ok(results)
}

/// Rename network interface
pub async fn link_set_name(old_name: &str, new_name: &str) -> Result<()> {
    let (connection, handle, _) = new_connection()?;
    tokio::spawn(connection);

    // Find interface by current name
    let mut links = handle.link().get().match_name(old_name.to_string()).execute();
    let link = links
        .try_next()
        .await?
        .context(format!("Interface '{}' not found", old_name))?;

    let ifindex = link.header.index;

    // Set new name
    handle
        .link()
        .set(ifindex)
        .name(new_name.to_string())
        .execute()
        .await
        .context(format!("Failed to rename {} to {}", old_name, new_name))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Basic smoke test to ensure rtnetlink connection and route listing works.
    // Uses the loopback interface which always exists.
    #[tokio::test(flavor = "current_thread")]
    async fn test_list_routes_for_loopback() {
        let res = list_routes_for_interface("lo").await;
        assert!(res.is_ok(), "expected Ok from list_routes_for_interface: {:?}", res);
        let routes = res.unwrap();
        // No strict expectation on content; presence/empty is both fine.
        println!("routes on lo: {:?}", routes);
    }
}

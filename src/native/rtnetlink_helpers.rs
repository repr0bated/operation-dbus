//! Rtnetlink helpers - native netlink operations for IP addresses and routes

use anyhow::{Context, Result};
use futures::TryStreamExt;
use netlink_packet_route::address::AddressAttribute;
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

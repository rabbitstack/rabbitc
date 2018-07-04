// Copyright 2018 by Nedim Sabic (RabbitStack)
// http://rabbitstack.github.io
//
// Licensed under the Apache License, Version 2.0 (the "License"); you may
// not use this file except in compliance with the License. You may obtain
// a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS, WITHOUT
// WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the
// License for the specific language governing permissions and limitations
// under the License.

use pnetlink::packet::netlink::{NetlinkConnection};
use pnetlink::packet::route::link::{Links};
use pnetlink::packet::route::addr::{Addresses, Scope};
use net::netlink::bridge::Bridge;
use net::netlink::veth::Veth;

use ipaddress::IPAddress;
use std::net::IpAddr;
use std::str::FromStr;

use std::io;

/// Initializes a new bridge device with provided network options
/// such as gateway address or bridge name. If the bridge already
/// exists this function returns `Ok(())`.
pub fn init(name: &str, ipv4: &str) -> io::Result<()> {
    let mut conn = NetlinkConnection::new();
    match conn.new_bridge(name) {
        Ok(_) => {},
        Err(e) => {
            if e.kind() == io::ErrorKind::AlreadyExists { return Ok(()); }
            return Err(e);
        }
    }

    // bring up bridge
    let bridge = conn.get_link_by_name(name)?.unwrap();
    conn.link_set_up(bridge.get_index())?;

    // bind ip address
    let ip = IPAddress::parse(ipv4).map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
    conn.add_addr(
        &bridge,
        IpAddr::from_str(ip.to_s().as_str()).unwrap(),
        None::<IpAddr>,
        Scope::Universe,
        ip.prefix.to_i() as u8,
    )?;

    Ok(())
}

/// Creates a new pair of veth interfaces and binds the interface on the host
/// end to the bridge device.
pub fn create_veth(peer: &str, bridge_name: &str) -> io::Result<()> {
    let mut conn = NetlinkConnection::new();
    conn.add_veth(peer)?;
    let veth = conn.get_link_by_name("veth0")?.unwrap();
    // TODO: this is hacky. We change the veth name
    // assuming it gets `veth0` name upon creation.
    // Eventually, we should get the veth pair from peer
    // and then proceed with setting new interface name
    conn.set_name(&veth, &format!("veth{}", super::generate_ifname(7)))?;
    conn.link_set_up(veth.get_index())?;

    let bridge = conn.get_link_by_name(bridge_name)?.unwrap();
    conn.set_master(&veth, &bridge)?;

    Ok(())
}

/// Moves peer pair of the veth interface to the network namespace where
/// process with `pid` identifier is living.
pub fn join(peer: &str, pid: u32) -> io::Result<()> {
    let mut conn = NetlinkConnection::new();
    let link = conn.get_link_by_name(peer)?.unwrap();
    conn.set_pid_namespace(&link, pid)?;
    Ok(())
}

/// Setups container network interfaces and assigns the container ip address.
pub fn setup_peer(peer: &str, container_ip: &str) -> io::Result<()> {
    let mut conn = NetlinkConnection::new();
    let link = conn.get_link_by_name(peer)?.unwrap();
    let lo = conn.get_link_by_name("lo")?.unwrap();
    conn.link_set_up(lo.get_index())?;

    let ip = IPAddress::parse(container_ip).map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
    conn.add_addr(
        &link,
        IpAddr::from_str(ip.to_s().as_str()).unwrap(),
        None::<IpAddr>,
        Scope::Universe,
        ip.prefix.to_i() as u8,
    )?;

    conn.link_set_up(link.get_index())?;
    Ok(())
}

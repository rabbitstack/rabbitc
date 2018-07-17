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

use pnetlink::packet::netlink::{NetlinkConnection, NetlinkReader, NetlinkRequestBuilder};
use pnetlink::packet::route::link::Link;
use pnetlink::packet::route::link::{IfInfoPacketBuilder, IFLA_IFNAME, IFLA_INFO_KIND,
                                    IFLA_LINKINFO, IFLA_MASTER, IFLA_NET_NS_PID, RTM_NEWLINK,
                                    RTM_SETLINK};
use pnetlink::packet::route::{MutableIfInfoPacket, RtAttrPacket};
use pnetlink::packet::route::route::WithPayload;
use pnetlink::packet::netlink::NetlinkMsgFlags;
use pnet_macros_support::packet::Packet;

use std::io;
use std::io::Write;

pub trait Veth {
    /// Creates a new pair of veth <-> peer links.
    fn add_veth(&mut self, peer: &str) -> io::Result<()>;
    /// Moves the provided link to the namespace where process with specified pid is running.
    fn set_pid_namespace(&mut self, link: &Link, pid: u32) -> io::Result<()>;
    /// Set the master link for the provided link.
    fn set_master(&mut self, link: &Link, master: &Link) -> io::Result<()>;
    /// Changes interface name.
    fn set_name(&mut self, link: &Link, name: &str) -> io::Result<()>;
}

impl Veth for NetlinkConnection {
    /// Creates a new pair of veth <-> peer links.
    fn add_veth(&mut self, peer: &str) -> io::Result<()> {
        let ifi = {
            IfInfoPacketBuilder::new()
                .append(RtAttrPacket::create_with_payload(IFLA_IFNAME, peer))
                .append(RtAttrPacket::create_with_payload(
                    IFLA_LINKINFO,
                    RtAttrPacket::create_with_payload(IFLA_INFO_KIND, "veth"),
                ))
                .build()
        };
        let req = NetlinkRequestBuilder::new(
            RTM_NEWLINK,
            NetlinkMsgFlags::NLM_F_CREATE | NetlinkMsgFlags::NLM_F_EXCL
                | NetlinkMsgFlags::NLM_F_ACK,
        ).append(ifi)
            .build();
        self.write(req.packet())?;
        let reader = NetlinkReader::new(self);
        reader.read_to_end()
    }

    /// Moves the provided link to the namespace where process with specified pid is running.
    fn set_pid_namespace(&mut self, link: &Link, pid: u32) -> io::Result<()> {
        let mut buf = vec![0; MutableIfInfoPacket::minimum_packet_size()];
        let req = NetlinkRequestBuilder::new(RTM_SETLINK, NetlinkMsgFlags::NLM_F_ACK)
            .append({
                let mut ifinfo = MutableIfInfoPacket::new(&mut buf).unwrap();
                ifinfo.set_family(0 /* AF_UNSPEC */);
                ifinfo.set_index(link.get_index());
                ifinfo
            })
            .append(RtAttrPacket::create_with_payload(IFLA_NET_NS_PID, pid))
            .build();
        self.write(req.packet())?;
        let reader = NetlinkReader::new(self);
        reader.read_to_end()
    }

    /// Set the master link for the provided link.
    fn set_master(&mut self, link: &Link, master: &Link) -> io::Result<()> {
        let mut buf = vec![0; MutableIfInfoPacket::minimum_packet_size()];
        let req = NetlinkRequestBuilder::new(RTM_SETLINK, NetlinkMsgFlags::NLM_F_ACK)
            .append({
                let mut ifinfo = MutableIfInfoPacket::new(&mut buf).unwrap();
                ifinfo.set_family(0 /* AF_UNSPEC */);
                ifinfo.set_index(link.get_index());
                ifinfo
            })
            .append(RtAttrPacket::create_with_payload(
                IFLA_MASTER,
                master.get_index(),
            ))
            .build();
        self.write(req.packet())?;
        let reader = NetlinkReader::new(self);
        reader.read_to_end()
    }

    /// Changes interface name.
    fn set_name(&mut self, link: &Link, name: &str) -> io::Result<()> {
        let mut buf = vec![0; MutableIfInfoPacket::minimum_packet_size()];
        let req = NetlinkRequestBuilder::new(RTM_SETLINK, NetlinkMsgFlags::NLM_F_ACK)
            .append({
                let mut ifinfo = MutableIfInfoPacket::new(&mut buf).unwrap();
                ifinfo.set_family(0 /* AF_UNSPEC */);
                ifinfo.set_index(link.get_index());
                ifinfo
            })
            .append(RtAttrPacket::create_with_payload(IFLA_IFNAME, name))
            .build();
        self.write(req.packet())?;
        let reader = NetlinkReader::new(self);
        reader.read_to_end()
    }
}

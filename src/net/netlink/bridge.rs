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

use pnetlink::packet::netlink::{NetlinkConnection, NetlinkRequestBuilder, NetlinkReader};
use pnetlink::packet::route::link::{IfInfoPacketBuilder, IFLA_IFNAME, IFLA_LINKINFO, IFLA_INFO_KIND, RTM_NEWLINK};
use pnetlink::packet::route::RtAttrPacket;
use pnetlink::packet::route::route::WithPayload;
use pnetlink::packet::netlink::NetlinkMsgFlags;
use pnet_macros_support::packet::Packet;

use std::io;
use std::io::Write;

pub trait Bridge {
    /// Creates a new bridge kernel device.
    fn new_bridge(&mut self, name: &str) -> io::Result<()>;
}

impl Bridge for NetlinkConnection {
    /// Creates a new bridge kernel device.
    fn new_bridge(&mut self, name: &str) -> io::Result<()> {
        let ifi = {
            IfInfoPacketBuilder::new().
                append(RtAttrPacket::create_with_payload(IFLA_IFNAME, name)).
                append(RtAttrPacket::create_with_payload(
                    IFLA_LINKINFO, RtAttrPacket::create_with_payload(IFLA_INFO_KIND, "bridge"))).build()
        };
        let req = NetlinkRequestBuilder::new(RTM_NEWLINK, NetlinkMsgFlags::NLM_F_CREATE | NetlinkMsgFlags::NLM_F_EXCL | NetlinkMsgFlags::NLM_F_ACK)
            .append(ifi).build();
        self.write(req.packet())?;
        let reader = NetlinkReader::new(self);
        reader.read_to_end()
    }
}

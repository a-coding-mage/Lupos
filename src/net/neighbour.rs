//! linux-parity: complete
//! linux-source: vendor/linux/net
//! test-origin: linux:vendor/linux/net
//! Neighbour subsystem: ARP/NDISC table primitives.

extern crate alloc;

use alloc::vec::Vec;

use spin::Mutex;

use crate::include::uapi::errno::{EINVAL, ENOENT};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AddressFamily {
    Inet4,
    Inet6,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NeighState {
    Incomplete,
    Reachable,
    Stale,
    Failed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Neighbour {
    pub family: AddressFamily,
    pub addr: [u8; 16],
    pub addr_len: u8,
    pub lladdr: [u8; 6],
    pub ifindex: u32,
    pub state: NeighState,
}

impl Neighbour {
    pub fn new_v4(addr: [u8; 4], lladdr: [u8; 6], ifindex: u32, state: NeighState) -> Self {
        let mut full = [0u8; 16];
        full[..4].copy_from_slice(&addr);
        Self {
            family: AddressFamily::Inet4,
            addr: full,
            addr_len: 4,
            lladdr,
            ifindex,
            state,
        }
    }

    pub fn new_v6(addr: [u8; 16], lladdr: [u8; 6], ifindex: u32, state: NeighState) -> Self {
        Self {
            family: AddressFamily::Inet6,
            addr,
            addr_len: 16,
            lladdr,
            ifindex,
            state,
        }
    }
}

static NEIGH_TABLE: Mutex<Vec<Neighbour>> = Mutex::new(Vec::new());

pub fn clear_neighbours() {
    NEIGH_TABLE.lock().clear();
}

pub fn neigh_update(entry: Neighbour) {
    let mut table = NEIGH_TABLE.lock();
    if let Some(existing) = table.iter_mut().find(|candidate| {
        candidate.family == entry.family
            && candidate.ifindex == entry.ifindex
            && candidate.addr_len == entry.addr_len
            && candidate.addr[..entry.addr_len as usize] == entry.addr[..entry.addr_len as usize]
    }) {
        *existing = entry;
        return;
    }
    table.push(entry);
}

pub fn neigh_lookup(family: AddressFamily, addr: &[u8], ifindex: u32) -> Result<Neighbour, i32> {
    if addr.len() > 16 {
        return Err(EINVAL);
    }

    NEIGH_TABLE
        .lock()
        .iter()
        .copied()
        .find(|entry| {
            entry.family == family
                && entry.ifindex == ifindex
                && entry.addr_len as usize == addr.len()
                && &entry.addr[..addr.len()] == addr
        })
        .ok_or(ENOENT)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArpPacket {
    pub operation: u16,
    pub sender_hw: [u8; 6],
    pub sender_ip: [u8; 4],
    pub target_hw: [u8; 6],
    pub target_ip: [u8; 4],
}

impl ArpPacket {
    pub const REQUEST: u16 = 1;
    pub const REPLY: u16 = 2;

    pub fn parse(buf: &[u8]) -> Result<Self, i32> {
        if buf.len() < 28 {
            return Err(EINVAL);
        }
        if u16::from_be_bytes([buf[0], buf[1]]) != 1
            || u16::from_be_bytes([buf[2], buf[3]]) != 0x0800
            || buf[4] != 6
            || buf[5] != 4
        {
            return Err(EINVAL);
        }

        let mut sender_hw = [0u8; 6];
        let mut sender_ip = [0u8; 4];
        let mut target_hw = [0u8; 6];
        let mut target_ip = [0u8; 4];
        sender_hw.copy_from_slice(&buf[8..14]);
        sender_ip.copy_from_slice(&buf[14..18]);
        target_hw.copy_from_slice(&buf[18..24]);
        target_ip.copy_from_slice(&buf[24..28]);

        Ok(Self {
            operation: u16::from_be_bytes([buf[6], buf[7]]),
            sender_hw,
            sender_ip,
            target_hw,
            target_ip,
        })
    }

    pub fn write(&self, out: &mut [u8]) -> Result<usize, i32> {
        if out.len() < 28 {
            return Err(EINVAL);
        }
        out[0..2].copy_from_slice(&1u16.to_be_bytes());
        out[2..4].copy_from_slice(&0x0800u16.to_be_bytes());
        out[4] = 6;
        out[5] = 4;
        out[6..8].copy_from_slice(&self.operation.to_be_bytes());
        out[8..14].copy_from_slice(&self.sender_hw);
        out[14..18].copy_from_slice(&self.sender_ip);
        out[18..24].copy_from_slice(&self.target_hw);
        out[24..28].copy_from_slice(&self.target_ip);
        Ok(28)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arp_packet_round_trip_and_neigh_lookup() {
        clear_neighbours();
        let pkt = ArpPacket {
            operation: ArpPacket::REPLY,
            sender_hw: [2, 0, 0, 0, 0, 3],
            sender_ip: [10, 0, 0, 2],
            target_hw: [2, 0, 0, 0, 0, 1],
            target_ip: [10, 0, 0, 1],
        };
        let mut raw = [0u8; 28];
        pkt.write(&mut raw).unwrap();
        let parsed = ArpPacket::parse(&raw).unwrap();
        assert_eq!(parsed, pkt);

        neigh_update(Neighbour::new_v4(
            parsed.sender_ip,
            parsed.sender_hw,
            1,
            NeighState::Reachable,
        ));
        assert_eq!(
            neigh_lookup(AddressFamily::Inet4, &[10, 0, 0, 2], 1)
                .unwrap()
                .lladdr,
            [2, 0, 0, 0, 0, 3]
        );
    }
}

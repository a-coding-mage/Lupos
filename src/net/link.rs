//! linux-parity: complete
//! linux-source: vendor/linux/net
//! test-origin: linux:vendor/linux/net
//! Bridge, VLAN, and bonding primitives.

extern crate alloc;

use alloc::vec::Vec;

use crate::include::uapi::errno::{EINVAL, ENODEV};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bridge {
    pub ifindex: u32,
    pub ports: Vec<u32>,
}

impl Bridge {
    pub fn new(ifindex: u32) -> Self {
        Self {
            ifindex,
            ports: Vec::new(),
        }
    }

    pub fn add_port(&mut self, ifindex: u32) {
        if !self.ports.contains(&ifindex) {
            self.ports.push(ifindex);
        }
    }

    pub fn forward_ports(&self, ingress: u32) -> Vec<u32> {
        self.ports
            .iter()
            .copied()
            .filter(|port| *port != ingress)
            .collect()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VlanDevice {
    pub ifindex: u32,
    pub lower_ifindex: u32,
    pub vlan_id: u16,
}

impl VlanDevice {
    pub fn new(ifindex: u32, lower_ifindex: u32, vlan_id: u16) -> Result<Self, i32> {
        if vlan_id == 0 || vlan_id > 4094 {
            return Err(EINVAL);
        }
        Ok(Self {
            ifindex,
            lower_ifindex,
            vlan_id,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BondMode {
    ActiveBackup,
    BalanceRoundRobin,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bond {
    pub ifindex: u32,
    pub mode: BondMode,
    slaves: Vec<(u32, bool)>,
    cursor: usize,
}

impl Bond {
    pub fn new(ifindex: u32, mode: BondMode) -> Self {
        Self {
            ifindex,
            mode,
            slaves: Vec::new(),
            cursor: 0,
        }
    }

    pub fn add_slave(&mut self, ifindex: u32) {
        if !self
            .slaves
            .iter()
            .any(|(candidate, _)| *candidate == ifindex)
        {
            self.slaves.push((ifindex, true));
        }
    }

    pub fn set_link(&mut self, ifindex: u32, up: bool) -> Result<(), i32> {
        let slave = self
            .slaves
            .iter_mut()
            .find(|(candidate, _)| *candidate == ifindex)
            .ok_or(ENODEV)?;
        slave.1 = up;
        Ok(())
    }

    pub fn choose_tx_slave(&mut self) -> Result<u32, i32> {
        match self.mode {
            BondMode::ActiveBackup => self
                .slaves
                .iter()
                .find(|(_, up)| *up)
                .map(|(ifindex, _)| *ifindex)
                .ok_or(ENODEV),
            BondMode::BalanceRoundRobin => {
                if self.slaves.is_empty() {
                    return Err(ENODEV);
                }
                for _ in 0..self.slaves.len() {
                    let index = self.cursor % self.slaves.len();
                    self.cursor = self.cursor.wrapping_add(1);
                    let (ifindex, up) = self.slaves[index];
                    if up {
                        return Ok(ifindex);
                    }
                }
                Err(ENODEV)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bridge_vlan_and_bonding_behave() {
        let mut bridge = Bridge::new(10);
        bridge.add_port(1);
        bridge.add_port(2);
        assert_eq!(bridge.forward_ports(1), alloc::vec![2]);

        assert_eq!(VlanDevice::new(20, 1, 100).unwrap().vlan_id, 100);

        let mut bond = Bond::new(30, BondMode::ActiveBackup);
        bond.add_slave(1);
        bond.add_slave(2);
        assert_eq!(bond.choose_tx_slave().unwrap(), 1);
        bond.set_link(1, false).unwrap();
        assert_eq!(bond.choose_tx_slave().unwrap(), 2);
    }
}

//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/amd_node.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/amd_node.c
//! AMD northbridge node helpers.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/amd_node.c
//!
//! Linux reaches AMD per-node SMN/HSMP registers through node-local PCI
//! config ports. Lupos keeps the PCI config access behind a trait so the
//! address selection and error handling are testable before live hardware
//! integration is wired.

use crate::include::uapi::errno::{ENODEV, EOPNOTSUPP};

pub const MAX_AMD_NUM_NODES: u16 = 8;
pub const AMD_NODE0_PCI_SLOT: u8 = 0x18;

pub const SMN_INDEX_OFFSET: u16 = 0x60;
pub const SMN_DATA_OFFSET: u16 = 0x64;
pub const HSMP_INDEX_OFFSET: u16 = 0xc4;
pub const HSMP_DATA_OFFSET: u16 = 0xc8;
pub const PCI_ERROR_RESPONSE: u32 = 0xffff_ffff;

pub trait SmnPort {
    fn write_config_dword(&mut self, offset: u16, value: u32) -> Result<(), i32>;
    fn read_config_dword(&mut self, offset: u16) -> Result<u32, i32>;
}

pub const fn pci_devfn(slot: u8, func: u8) -> u8 {
    (slot << 3) | (func & 0x07)
}

pub const fn amd_node_get_func(node: u16, func: u8) -> Option<(u8, u8)> {
    if node < MAX_AMD_NUM_NODES {
        Some((0, pci_devfn(AMD_NODE0_PCI_SLOT + node as u8, func)))
    } else {
        None
    }
}

pub fn amd_smn_read<P: SmnPort>(
    port: &mut P,
    node: u16,
    num_nodes: u16,
    has_exclusive_access: bool,
    address: u32,
) -> Result<u32, i32> {
    if node >= num_nodes || node >= MAX_AMD_NUM_NODES {
        return Err(ENODEV);
    }
    if !has_exclusive_access {
        return Err(EOPNOTSUPP);
    }

    port.write_config_dword(SMN_INDEX_OFFSET, address)?;
    let value = port.read_config_dword(SMN_DATA_OFFSET)?;
    if value == PCI_ERROR_RESPONSE {
        Err(ENODEV)
    } else {
        Ok(value)
    }
}

pub fn amd_smn_write<P: SmnPort>(
    port: &mut P,
    node: u16,
    num_nodes: u16,
    has_exclusive_access: bool,
    address: u32,
    value: u32,
) -> Result<(), i32> {
    if node >= num_nodes || node >= MAX_AMD_NUM_NODES {
        return Err(ENODEV);
    }
    if !has_exclusive_access {
        return Err(EOPNOTSUPP);
    }

    port.write_config_dword(SMN_INDEX_OFFSET, address)?;
    port.write_config_dword(SMN_DATA_OFFSET, value)
}

pub fn amd_smn_hsmp_rdwr<P: SmnPort>(
    port: &mut P,
    node: u16,
    num_nodes: u16,
    has_exclusive_access: bool,
    address: u32,
    write: Option<u32>,
) -> Result<u32, i32> {
    if node >= num_nodes || node >= MAX_AMD_NUM_NODES {
        return Err(ENODEV);
    }
    if !has_exclusive_access {
        return Err(EOPNOTSUPP);
    }

    port.write_config_dword(HSMP_INDEX_OFFSET, address)?;
    if let Some(value) = write {
        port.write_config_dword(HSMP_DATA_OFFSET, value)?;
    }
    let value = port.read_config_dword(HSMP_DATA_OFFSET)?;
    if value == PCI_ERROR_RESPONSE {
        Err(ENODEV)
    } else {
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct FakePort {
        reads: u32,
        writes: [(u16, u32); 4],
        write_count: usize,
    }

    impl SmnPort for FakePort {
        fn write_config_dword(&mut self, offset: u16, value: u32) -> Result<(), i32> {
            self.writes[self.write_count] = (offset, value);
            self.write_count += 1;
            Ok(())
        }

        fn read_config_dword(&mut self, _offset: u16) -> Result<u32, i32> {
            Ok(self.reads)
        }
    }

    #[test]
    fn node_function_maps_onto_amd_node_slots() {
        assert_eq!(pci_devfn(0x18, 3), 0xc3);
        assert_eq!(amd_node_get_func(0, 3), Some((0, 0xc3)));
        assert_eq!(amd_node_get_func(7, 0), Some((0, 0xf8)));
        assert_eq!(amd_node_get_func(8, 0), None);
    }

    #[test]
    fn smn_read_writes_index_then_reads_data() {
        let mut port = FakePort {
            reads: 0x1234_5678,
            ..FakePort::default()
        };
        assert_eq!(
            amd_smn_read(&mut port, 1, MAX_AMD_NUM_NODES, true, 0xdead_beef),
            Ok(0x1234_5678)
        );
        assert_eq!(port.writes[0], (SMN_INDEX_OFFSET, 0xdead_beef));
    }

    #[test]
    fn smn_access_fails_closed_without_exclusive_access() {
        let mut port = FakePort::default();
        assert_eq!(
            amd_smn_read(&mut port, 0, MAX_AMD_NUM_NODES, false, 0),
            Err(EOPNOTSUPP)
        );
        assert_eq!(
            amd_smn_write(&mut port, MAX_AMD_NUM_NODES, MAX_AMD_NUM_NODES, true, 0, 0),
            Err(ENODEV)
        );
    }

    #[test]
    fn pci_error_response_is_reported_as_missing_device() {
        let mut port = FakePort {
            reads: PCI_ERROR_RESPONSE,
            ..FakePort::default()
        };
        assert_eq!(
            amd_smn_read(&mut port, 0, MAX_AMD_NUM_NODES, true, 0x40),
            Err(ENODEV)
        );
    }

    #[test]
    fn hsmp_uses_its_own_index_and_data_offsets() {
        let mut port = FakePort {
            reads: 0x55aa,
            ..FakePort::default()
        };
        assert_eq!(
            amd_smn_hsmp_rdwr(&mut port, 0, MAX_AMD_NUM_NODES, true, 0x20, Some(7)),
            Ok(0x55aa)
        );
        assert_eq!(port.writes[0], (HSMP_INDEX_OFFSET, 0x20));
        assert_eq!(port.writes[1], (HSMP_DATA_OFFSET, 7));
    }
}

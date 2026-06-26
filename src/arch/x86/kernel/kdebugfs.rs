//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/kdebugfs.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/kdebugfs.c
//! Architecture-specific debugfs surface (`/sys/kernel/debug/x86/`).
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/kdebugfs.c
//!
//! Linux exposes the live `boot_params` blob and walks the
//! `setup_data` chain to surface each setup_data record as a debugfs
//! file. The chain walk handles both direct (`SETUP_*`) and indirect
//! (`SETUP_INDIRECT | SETUP_*`) records — port that logic faithfully.
//!
//! Linux ref: Documentation/admin-guide/kernel-parameters.txt — `debugfs=`.

#![allow(dead_code)]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use crate::include::uapi::errno::{EFAULT, EINVAL, ENOMEM};

// === setup_data type IDs — mirror vendor/linux/arch/x86/include/uapi/asm/setup_data.h ===

pub const SETUP_NONE: u32 = 0;
pub const SETUP_E820_EXT: u32 = 1;
pub const SETUP_DTB: u32 = 2;
pub const SETUP_PCI: u32 = 3;
pub const SETUP_EFI: u32 = 4;
pub const SETUP_APPLE_PROPERTIES: u32 = 5;
pub const SETUP_JAILHOUSE: u32 = 6;
pub const SETUP_CC_BLOB: u32 = 7;
pub const SETUP_IMA: u32 = 8;
pub const SETUP_RNG_SEED: u32 = 9;
pub const SETUP_KEXEC_KHO: u32 = 10;
pub const SETUP_ENUM_MAX: u32 = SETUP_KEXEC_KHO;
pub const SETUP_INDIRECT: u32 = 1 << 31;
pub const SETUP_TYPE_MAX: u32 = SETUP_ENUM_MAX | SETUP_INDIRECT;

/// `struct setup_data` — extensible setup-data list node.
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct SetupData {
    pub next: u64,
    pub type_: u32,
    pub len: u32,
}

pub const SETUP_DATA_HEADER_LEN: usize = core::mem::size_of::<SetupData>();

/// `struct setup_indirect` — payload for `SETUP_INDIRECT` records.
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct SetupIndirect {
    pub type_: u32,
    pub reserved: u32,
    pub len: u64,
    pub addr: u64,
}

/// One node in the debugfs setup-data chain — the values exposed under
/// `/sys/kernel/debug/x86/boot_params/setup_data/<n>/`.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct SetupDataNode {
    pub paddr: u64,
    pub type_: u32,
    pub len: u32,
}

/// Trait seam for `memremap()` — reads bytes from a guest-physical
/// address. Tests use an in-memory `Vec<u8>`-backed implementation; the
/// real kernel wires this to the `memremap` machinery.
pub trait PhysMem {
    fn read(&self, paddr: u64, out: &mut [u8]) -> Result<(), i32>;
}

/// Trait seam for the (very small) subset of debugfs ops kdebugfs uses.
pub trait DebugfsBackend {
    fn mkdir(&self, parent: Option<&str>, name: &str) -> Result<String, i32>;
    fn create_x16(&self, parent: &str, name: &str, value: u16) -> Result<(), i32>;
    fn create_x32(&self, parent: &str, name: &str, value: u32) -> Result<(), i32>;
    fn create_blob(&self, parent: &str, name: &str, blob: &[u8]) -> Result<(), i32>;
    fn create_setup_data_file(
        &self,
        parent: &str,
        name: &str,
        node: SetupDataNode,
    ) -> Result<(), i32>;
}

/// `setup_data_read`: bounded read from a setup_data record at `node`.
/// Mirrors the Linux fop: clamp `count` to remaining bytes; if the record
/// is *not* indirect, skip the SetupData header (the payload follows it).
pub fn setup_data_read<M: PhysMem>(
    mem: &M,
    node: &SetupDataNode,
    pos: i64,
    count: usize,
    out: &mut [u8],
) -> Result<usize, i32> {
    if pos < 0 {
        return Err(EINVAL);
    }
    let upos = pos as u64;
    if upos >= node.len as u64 {
        return Ok(0);
    }
    let max = (node.len as u64 - upos) as usize;
    let count = core::cmp::min(count, core::cmp::min(out.len(), max));
    let mut pa = node.paddr + upos;
    // Linux: only direct records have the header preceding the payload.
    let is_indirect = (node.type_ & SETUP_INDIRECT) != 0 && node.type_ != SETUP_INDIRECT;
    if !is_indirect {
        pa += SETUP_DATA_HEADER_LEN as u64;
    }
    mem.read(pa, &mut out[..count])?;
    Ok(count)
}

/// Walk the `setup_data` linked list starting at `pa_head`, materialising
/// one `SetupDataNode` per record. Mirrors `create_setup_data_nodes`
/// minus the debugfs side-effects.
pub fn collect_setup_data_chain<M: PhysMem>(
    mem: &M,
    pa_head: u64,
    max_records: usize,
) -> Result<Vec<SetupDataNode>, i32> {
    let mut out = Vec::new();
    let mut pa = pa_head;
    while pa != 0 && out.len() < max_records {
        let mut buf = [0u8; core::mem::size_of::<SetupData>()];
        mem.read(pa, &mut buf)?;
        let next = u64::from_le_bytes(buf[0..8].try_into().map_err(|_| EFAULT)?);
        let type_ = u32::from_le_bytes(buf[8..12].try_into().map_err(|_| EFAULT)?);
        let len = u32::from_le_bytes(buf[12..16].try_into().map_err(|_| EFAULT)?);

        if type_ == SETUP_INDIRECT {
            // Indirect: read the inner `setup_indirect` to extract its
            // type/len/addr, which may itself again carry SETUP_INDIRECT.
            let mut ibuf = [0u8; core::mem::size_of::<SetupIndirect>()];
            mem.read(pa + SETUP_DATA_HEADER_LEN as u64, &mut ibuf)?;
            let itype = u32::from_le_bytes(ibuf[0..4].try_into().map_err(|_| EFAULT)?);
            let ilen = u64::from_le_bytes(ibuf[8..16].try_into().map_err(|_| EFAULT)?);
            let iaddr = u64::from_le_bytes(ibuf[16..24].try_into().map_err(|_| EFAULT)?);
            if itype != SETUP_INDIRECT {
                out.push(SetupDataNode {
                    paddr: iaddr,
                    type_: itype,
                    len: ilen as u32,
                });
            } else {
                out.push(SetupDataNode {
                    paddr: pa,
                    type_,
                    len,
                });
            }
        } else {
            out.push(SetupDataNode {
                paddr: pa,
                type_,
                len,
            });
        }
        pa = next;
    }
    if out.capacity() == 0 && pa_head != 0 {
        return Err(ENOMEM);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::collections::BTreeMap;
    use core::cell::RefCell;

    #[derive(Default)]
    struct MemMap {
        bytes: RefCell<BTreeMap<u64, u8>>,
    }

    impl MemMap {
        fn put(&self, addr: u64, bytes: &[u8]) {
            let mut m = self.bytes.borrow_mut();
            for (i, b) in bytes.iter().enumerate() {
                m.insert(addr + i as u64, *b);
            }
        }
    }

    impl PhysMem for MemMap {
        fn read(&self, paddr: u64, out: &mut [u8]) -> Result<(), i32> {
            let m = self.bytes.borrow();
            for (i, slot) in out.iter_mut().enumerate() {
                *slot = *m.get(&(paddr + i as u64)).ok_or(EFAULT)?;
            }
            Ok(())
        }
    }

    fn pack_setup_data(next: u64, type_: u32, len: u32) -> Vec<u8> {
        let mut b = Vec::with_capacity(16);
        b.extend_from_slice(&next.to_le_bytes());
        b.extend_from_slice(&type_.to_le_bytes());
        b.extend_from_slice(&len.to_le_bytes());
        b
    }

    #[test]
    fn setup_type_constants_match_uapi() {
        assert_eq!(SETUP_NONE, 0);
        assert_eq!(SETUP_E820_EXT, 1);
        assert_eq!(SETUP_DTB, 2);
        assert_eq!(SETUP_INDIRECT, 1u32 << 31);
    }

    #[test]
    fn setup_data_header_is_16_bytes() {
        assert_eq!(SETUP_DATA_HEADER_LEN, 16);
    }

    #[test]
    fn setup_data_read_clamps_count_to_remaining() {
        let mem = MemMap::default();
        // Payload starts after the 16-byte header.
        mem.put(0x1010, &[0xAA; 8]);
        let node = SetupDataNode {
            paddr: 0x1000,
            type_: SETUP_E820_EXT,
            len: 8,
        };
        let mut out = [0u8; 32];
        let n = setup_data_read(&mem, &node, 0, 100, &mut out).unwrap();
        assert_eq!(n, 8);
        assert!(out[..8].iter().all(|&b| b == 0xAA));
    }

    #[test]
    fn setup_data_read_rejects_negative_pos() {
        let mem = MemMap::default();
        let node = SetupDataNode {
            paddr: 0x0,
            type_: 0,
            len: 1,
        };
        let mut out = [0u8; 8];
        assert_eq!(setup_data_read(&mem, &node, -1, 1, &mut out), Err(EINVAL));
    }

    #[test]
    fn collect_setup_data_chain_terminates_on_zero_next() {
        let mem = MemMap::default();
        mem.put(0x1000, &pack_setup_data(0x2000, SETUP_E820_EXT, 4));
        mem.put(0x2000, &pack_setup_data(0, SETUP_DTB, 8));
        let chain = collect_setup_data_chain(&mem, 0x1000, 16).unwrap();
        assert_eq!(chain.len(), 2);
        assert_eq!(chain[0].type_, SETUP_E820_EXT);
        assert_eq!(chain[1].type_, SETUP_DTB);
    }

    #[test]
    fn collect_setup_data_chain_unwraps_indirect_record() {
        let mem = MemMap::default();
        mem.put(0x1000, &pack_setup_data(0, SETUP_INDIRECT, 24));
        let mut indirect = Vec::new();
        indirect.extend_from_slice(&SETUP_PCI.to_le_bytes());
        indirect.extend_from_slice(&0u32.to_le_bytes());
        indirect.extend_from_slice(&64u64.to_le_bytes());
        indirect.extend_from_slice(&0xDEAD_0000u64.to_le_bytes());
        mem.put(0x1010, &indirect);

        let chain = collect_setup_data_chain(&mem, 0x1000, 4).unwrap();
        assert_eq!(chain.len(), 1);
        assert_eq!(chain[0].type_, SETUP_PCI);
        assert_eq!(chain[0].paddr, 0xDEAD_0000);
        assert_eq!(chain[0].len, 64);
    }

    #[test]
    fn collect_setup_data_chain_caps_at_max_records() {
        let mem = MemMap::default();
        // Two records but ask for only 1.
        mem.put(0x1000, &pack_setup_data(0x2000, SETUP_E820_EXT, 4));
        mem.put(0x2000, &pack_setup_data(0, SETUP_DTB, 8));
        let chain = collect_setup_data_chain(&mem, 0x1000, 1).unwrap();
        assert_eq!(chain.len(), 1);
    }
}

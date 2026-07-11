//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/kernel/probe_roms.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/probe_roms.c
//! Legacy option-ROM probing.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/probe_roms.c

#![allow(dead_code)]

extern crate alloc;

use alloc::vec::Vec;

use crate::include::uapi::errno::EFAULT;

pub const ROM_SIGNATURE: [u8; 2] = [0x55, 0xaa];
pub const ROM_GRANULARITY: u64 = 2048;

pub trait RomMemory {
    fn read(&self, addr: u64, out: &mut [u8]) -> Result<(), i32>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RomKind {
    Video,
    System,
    Adapter,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RomRange {
    pub start: u64,
    pub end: u64,
    pub kind: RomKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RomResource {
    pub start: u64,
    pub len: u64,
    pub kind: RomKind,
}

pub const fn romsignature(header: &[u8]) -> bool {
    header.len() >= 2 && header[0] == ROM_SIGNATURE[0] && header[1] == ROM_SIGNATURE[1]
}

pub fn romchecksum(bytes: &[u8]) -> bool {
    bytes.iter().fold(0u8, |sum, b| sum.wrapping_add(*b)) == 0
}

pub fn rom_size<M: RomMemory>(mem: &M, addr: u64) -> Result<u64, i32> {
    let mut header = [0u8; 3];
    mem.read(addr, &mut header)?;
    if !romsignature(&header) {
        return Err(EFAULT);
    }
    Ok(header[2] as u64 * 512)
}

pub fn probe_roms<M: RomMemory>(mem: &M, ranges: &[RomRange]) -> Vec<RomResource> {
    let mut out = Vec::new();
    for range in ranges {
        let mut addr = range.start;
        while addr + 2 <= range.end {
            if let Ok(size) = rom_size(mem, addr) {
                if size != 0 {
                    out.push(RomResource {
                        start: addr,
                        len: size,
                        kind: range.kind,
                    });
                    addr = addr.saturating_add(size);
                    continue;
                }
            }
            addr = addr.saturating_add(ROM_GRANULARITY);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::collections::BTreeMap;
    use core::cell::RefCell;

    #[derive(Default)]
    struct Mem(RefCell<BTreeMap<u64, u8>>);

    impl Mem {
        fn put(&self, addr: u64, bytes: &[u8]) {
            for (i, b) in bytes.iter().enumerate() {
                self.0.borrow_mut().insert(addr + i as u64, *b);
            }
        }
    }

    impl RomMemory for Mem {
        fn read(&self, addr: u64, out: &mut [u8]) -> Result<(), i32> {
            for (i, slot) in out.iter_mut().enumerate() {
                *slot = *self.0.borrow().get(&(addr + i as u64)).unwrap_or(&0);
            }
            Ok(())
        }
    }

    #[test]
    fn signature_checksum_and_scan_find_rom() {
        let mem = Mem::default();
        mem.put(0xc0000, &[0x55, 0xaa, 1, 1]);
        let roms = probe_roms(
            &mem,
            &[RomRange {
                start: 0xc0000,
                end: 0xc8000,
                kind: RomKind::Video,
            }],
        );
        assert_eq!(roms.len(), 1);
        assert_eq!(roms[0].len, 512);
        assert!(romsignature(&[0x55, 0xaa]));
        assert!(romchecksum(&[0u8, 0u8]));
    }
}

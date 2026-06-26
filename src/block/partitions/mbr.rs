//! linux-parity: complete
//! linux-source: vendor/linux/block/partitions
//! test-origin: linux:vendor/linux/block/partitions
//! MBR partition table parser.
//!
//! Mirrors `vendor/linux/block/partitions/msdos.c::msdos_partition`.
//! Layout: 446 bytes of bootcode, 4 × 16-byte partition entries, 0x55AA sig.

extern crate alloc;

use alloc::vec::Vec;

use super::Partition;

const MBR_SIG_OFF: usize = 510;
const PART_TABLE_OFF: usize = 446;
const ENTRY_SIZE: usize = 16;
const NR_PRIMARY: usize = 4;

pub const PART_TYPE_EMPTY: u8 = 0x00;
pub const PART_TYPE_GPT_PROTECTIVE: u8 = 0xEE;

pub fn has_valid_signature(sector0: &[u8]) -> bool {
    sector0.len() >= 512 && sector0[MBR_SIG_OFF] == 0x55 && sector0[MBR_SIG_OFF + 1] == 0xAA
}

pub fn is_protective_for_gpt(sector0: &[u8]) -> bool {
    if !has_valid_signature(sector0) {
        return false;
    }
    let off = PART_TABLE_OFF;
    sector0[off + 4] == PART_TYPE_GPT_PROTECTIVE
}

pub fn parse(sector0: &[u8]) -> Vec<Partition> {
    let mut out = Vec::new();
    if !has_valid_signature(sector0) {
        return out;
    }
    for i in 0..NR_PRIMARY {
        let off = PART_TABLE_OFF + i * ENTRY_SIZE;
        let type_byte = sector0[off + 4];
        if type_byte == PART_TYPE_EMPTY {
            continue;
        }
        let start = u32::from_le_bytes([
            sector0[off + 8],
            sector0[off + 9],
            sector0[off + 10],
            sector0[off + 11],
        ]) as u64;
        let len = u32::from_le_bytes([
            sector0[off + 12],
            sector0[off + 13],
            sector0[off + 14],
            sector0[off + 15],
        ]) as u64;
        if start == 0 || len == 0 {
            continue;
        }
        out.push(Partition {
            number: (i as u32) + 1,
            start_sector: start,
            nr_sectors: len,
            type_guid: None,
            type_byte: Some(type_byte),
        });
    }
    out
}

/// Build a sector-0 MBR with one primary partition.  Used by tests.
pub fn build_mbr_with_one_partition(
    sector0: &mut [u8],
    type_byte: u8,
    start_lba: u32,
    nr_sectors: u32,
) {
    sector0[MBR_SIG_OFF] = 0x55;
    sector0[MBR_SIG_OFF + 1] = 0xAA;
    let off = PART_TABLE_OFF;
    sector0[off] = 0x80; // bootable
    sector0[off + 4] = type_byte;
    let s = start_lba.to_le_bytes();
    sector0[off + 8..off + 12].copy_from_slice(&s);
    let n = nr_sectors.to_le_bytes();
    sector0[off + 12..off + 16].copy_from_slice(&n);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_one_partition() {
        let mut s0 = alloc::vec![0u8; 512];
        build_mbr_with_one_partition(&mut s0, 0x83, 2048, 100);
        assert!(has_valid_signature(&s0));
        let parts = parse(&s0);
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].start_sector, 2048);
        assert_eq!(parts[0].nr_sectors, 100);
        assert_eq!(parts[0].type_byte, Some(0x83));
    }
}

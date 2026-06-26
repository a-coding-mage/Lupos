//! linux-parity: partial
//! linux-source: vendor/linux/fs/ext4/extents.c
//! ext4 extent tree lookup.
//!
//! Mirrors `vendor/linux/fs/ext4/ext4_extents.h` and `extents.c::ext4_ext_get_blocks`.

extern crate alloc;

use alloc::vec::Vec;

use crate::block::partitions::read_sectors;
use crate::include::uapi::errno::EINVAL;

use super::Ext4Sbi;

pub const EXT4_EXT_MAGIC: u16 = 0xF30A;

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct ExtentHeader {
    pub eh_magic: u16,
    pub eh_entries: u16,
    pub eh_max: u16,
    pub eh_depth: u16,
    pub eh_generation: u32,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct Extent {
    pub ee_block: u32,
    pub ee_len: u16,
    pub ee_start_hi: u16,
    pub ee_start_lo: u32,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct ExtentIdx {
    pub ei_block: u32,
    pub ei_leaf_lo: u32,
    pub ei_leaf_hi: u16,
    pub ei_unused: u16,
}

/// Find the physical block backing logical block `lblock` of `inode`.
/// Returns `Ok(Some(physblock))` for a present mapping, `Ok(None)` for a
/// hole, `Err(_)` on corruption.
pub fn map_block(sbi: &Ext4Sbi, raw_i_block: [u32; 15], lblock: u64) -> Result<Option<u64>, i32> {
    // i_block is reinterpreted as a 60-byte tree root: 12-byte header + up
    // to 4 entries (×12 bytes) packed in.
    let buf: &[u8] = unsafe { core::slice::from_raw_parts(raw_i_block.as_ptr() as *const u8, 60) };
    walk_tree(sbi, buf, lblock)
}

fn parse_header(buf: &[u8]) -> Option<ExtentHeader> {
    if buf.len() < 12 {
        return None;
    }
    let h = ExtentHeader {
        eh_magic: u16::from_le_bytes([buf[0], buf[1]]),
        eh_entries: u16::from_le_bytes([buf[2], buf[3]]),
        eh_max: u16::from_le_bytes([buf[4], buf[5]]),
        eh_depth: u16::from_le_bytes([buf[6], buf[7]]),
        eh_generation: u32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]),
    };
    if h.eh_magic != EXT4_EXT_MAGIC {
        return None;
    }
    Some(h)
}

fn parse_extent(buf: &[u8]) -> Extent {
    Extent {
        ee_block: u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]),
        ee_len: u16::from_le_bytes([buf[4], buf[5]]),
        ee_start_hi: u16::from_le_bytes([buf[6], buf[7]]),
        ee_start_lo: u32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]),
    }
}

fn parse_idx(buf: &[u8]) -> ExtentIdx {
    ExtentIdx {
        ei_block: u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]),
        ei_leaf_lo: u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]),
        ei_leaf_hi: u16::from_le_bytes([buf[8], buf[9]]),
        ei_unused: u16::from_le_bytes([buf[10], buf[11]]),
    }
}

fn walk_tree(sbi: &Ext4Sbi, buf: &[u8], lblock: u64) -> Result<Option<u64>, i32> {
    let h = parse_header(buf).ok_or(EINVAL)?;
    if h.eh_depth == 0 {
        // Leaf — entries are Extents starting at offset 12.
        for i in 0..h.eh_entries as usize {
            let off = 12 + i * 12;
            if off + 12 > buf.len() {
                return Err(EINVAL);
            }
            let e = parse_extent(&buf[off..off + 12]);
            let first = e.ee_block as u64;
            let len = (e.ee_len & 0x7fff) as u64; // top bit = uninitialized
            if lblock >= first && lblock < first + len {
                let phys_first = ((e.ee_start_hi as u64) << 32) | (e.ee_start_lo as u64);
                return Ok(Some(phys_first + (lblock - first)));
            }
        }
        Ok(None)
    } else {
        // Index node — entries are ExtentIdx, point to the next-deeper block.
        let mut chosen_leaf: Option<u64> = None;
        let mut last_block: u64 = 0;
        for i in 0..h.eh_entries as usize {
            let off = 12 + i * 12;
            if off + 12 > buf.len() {
                return Err(EINVAL);
            }
            let idx = parse_idx(&buf[off..off + 12]);
            let block = idx.ei_block as u64;
            if i == 0 || block <= lblock {
                chosen_leaf = Some(((idx.ei_leaf_hi as u64) << 32) | (idx.ei_leaf_lo as u64));
                last_block = block;
            }
            let _ = last_block;
        }
        let leaf_block = chosen_leaf.ok_or(EINVAL)?;
        let leaf_buf = read_block(sbi, leaf_block)?;
        walk_tree(sbi, &leaf_buf, lblock)
    }
}

fn read_block(sbi: &Ext4Sbi, block: u64) -> Result<Vec<u8>, i32> {
    let lba = block * (sbi.block_size as u64) / 512;
    let nr_sectors = (sbi.block_size as u64) / 512;
    read_sectors(&sbi.bdev, lba, nr_sectors)
}

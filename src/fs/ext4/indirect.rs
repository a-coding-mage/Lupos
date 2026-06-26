//! linux-parity: partial
//! linux-source: vendor/linux/fs/ext4/indirect.c
//! ext4 legacy indirect-block map (pre-extents).
//!
//! Mirrors `vendor/linux/fs/ext4/inode.c::ext4_ind_map_blocks`.  Modern ext4
//! filesystems use extents; we keep this for ABI completeness.  Stub for now —
//! the M45 acceptance test runs against extent-based images.

extern crate alloc;

use crate::block::partitions::read_sectors;
use crate::include::uapi::errno::EINVAL;

use super::Ext4Sbi;

#[allow(dead_code)]
pub fn map_block(sbi: &Ext4Sbi, raw_i_block: [u32; 15], lblock: u64) -> Result<Option<u64>, i32> {
    let block_size = sbi.block_size as u64;
    let ptrs_per_block = block_size / 4;
    let direct = 12u64;
    let single = direct + ptrs_per_block;
    let double = single + ptrs_per_block * ptrs_per_block;
    if lblock < direct {
        let p = u32::from_le(raw_i_block[lblock as usize]) as u64;
        return Ok(if p == 0 { None } else { Some(p) });
    }
    if lblock < single {
        let p = u32::from_le(raw_i_block[12]) as u64;
        if p == 0 {
            return Ok(None);
        }
        let buf = read_sectors(&sbi.bdev, p * block_size / 512, block_size / 512)?;
        let off = ((lblock - direct) * 4) as usize;
        if off + 4 > buf.len() {
            return Err(EINVAL);
        }
        let v = u32::from_le_bytes([buf[off], buf[off + 1], buf[off + 2], buf[off + 3]]) as u64;
        return Ok(if v == 0 { None } else { Some(v) });
    }
    let _ = double;
    // Double + triple indirect: not exercised by the M45 fixture; bail.
    Err(EINVAL)
}

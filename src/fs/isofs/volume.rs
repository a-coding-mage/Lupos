//! linux-parity: partial
//! linux-source: vendor/linux/fs/isofs
//! ISO9660 Primary Volume Descriptor parser.
//!
//! PVD lives at logical sector 16 (byte offset 16 × 2048 = 32768).
//! Format reference: `vendor/linux/fs/isofs/inode.c::isofs_fill_super`.

extern crate alloc;

use crate::block::block_device::BlockDeviceRef;
use crate::block::partitions::read_sectors;
use crate::include::uapi::errno::EINVAL;

const PVD_SECTOR: u64 = 16;
const PVD_BYTES: usize = 2048;
pub const ISO_MAGIC: &[u8; 5] = b"CD001";

#[derive(Clone, Debug)]
pub struct Pvd {
    pub volume_id: alloc::string::String,
    pub root_extent: u32,
    pub root_size: u32,
}

pub fn read_pvd(bdev: &BlockDeviceRef) -> Result<Pvd, i32> {
    // ISO sector = 2048 bytes = 4 × 512.
    let lba = PVD_SECTOR * 4;
    let buf = read_sectors(bdev, lba, 4)?;
    if buf.len() < PVD_BYTES {
        return Err(EINVAL);
    }
    if buf[0] != 1 {
        return Err(EINVAL);
    } // type = primary
    if &buf[1..6] != ISO_MAGIC {
        return Err(EINVAL);
    }
    if buf[6] != 1 {
        return Err(EINVAL);
    } // version

    // Root directory record at offset 156, length 34.
    let root = &buf[156..156 + 34];
    if root.len() < 34 {
        return Err(EINVAL);
    }
    let root_extent = u32::from_le_bytes([root[2], root[3], root[4], root[5]]);
    let root_size = u32::from_le_bytes([root[10], root[11], root[12], root[13]]);

    let volume_id_bytes = &buf[40..40 + 32];
    let volume_id = core::str::from_utf8(volume_id_bytes)
        .unwrap_or("")
        .trim()
        .into();
    Ok(Pvd {
        volume_id,
        root_extent,
        root_size,
    })
}

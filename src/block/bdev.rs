//! linux-parity: partial
//! linux-source: vendor/linux/block/bdev.c
//! test-origin: linux:vendor/linux/block/bdev.c
//! Block-device open state and block-size helpers.

use crate::include::uapi::errno::{EBUSY, EINVAL, ENODEV, EROFS};

pub const PAGE_SIZE: u32 = 4096;
pub const SECTOR_SHIFT: u32 = 9;
pub const SECTOR_SIZE: u32 = 1 << SECTOR_SHIFT;
pub const BLK_MAX_BLOCK_SIZE: u32 = PAGE_SIZE;

pub const BLK_OPEN_READ: u32 = 1 << 0;
pub const BLK_OPEN_WRITE: u32 = 1 << 1;
pub const BLK_OPEN_EXCL: u32 = 1 << 2;
pub const BLK_OPEN_NDELAY: u32 = 1 << 3;
pub const BLK_OPEN_WRITE_IOCTL: u32 = 1 << 4;
pub const BLK_OPEN_RESTRICT_WRITES: u32 = 1 << 5;
pub const BLK_OPEN_STRICT_SCAN: u32 = 1 << 6;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Bdev {
    pub dev: u64,
    pub logical_block_size: u32,
    pub block_size: u32,
    pub size_sectors: u64,
    pub read_only: bool,
    pub live: bool,
    open_count: u32,
    exclusive_holder: Option<usize>,
    writes_blocked: u32,
}

impl Bdev {
    pub fn new(dev: u64, logical_block_size: u32, size_sectors: u64) -> Result<Self, i32> {
        blk_validate_block_size(logical_block_size)?;
        Ok(Self {
            dev,
            logical_block_size,
            block_size: logical_block_size,
            size_sectors,
            read_only: false,
            live: true,
            open_count: 0,
            exclusive_holder: None,
            writes_blocked: 0,
        })
    }

    pub fn nr_bytes(&self) -> u64 {
        self.size_sectors.saturating_mul(SECTOR_SIZE as u64)
    }

    pub fn open_count(&self) -> u32 {
        self.open_count
    }

    pub fn writes_blocked(&self) -> bool {
        self.writes_blocked != 0
    }

    pub fn validate_blocksize(&self, block_size: u32) -> Result<(), i32> {
        blk_validate_block_size(block_size)?;
        if block_size < self.logical_block_size {
            return Err(-EINVAL);
        }
        Ok(())
    }

    pub fn set_blocksize(&mut self, block_size: u32) -> Result<(), i32> {
        self.validate_blocksize(block_size)?;
        self.block_size = block_size;
        Ok(())
    }

    pub fn set_init_blocksize(&mut self) {
        let mut bsize = self.logical_block_size;
        let size = self.nr_bytes();
        while bsize < PAGE_SIZE {
            if size & bsize as u64 != 0 {
                break;
            }
            bsize <<= 1;
        }
        self.block_size = bsize;
    }

    pub fn open(&mut self, mode: u32, holder: Option<usize>) -> Result<(), i32> {
        if !self.live {
            return Err(-ENODEV);
        }
        if mode & BLK_OPEN_WRITE != 0 && (self.read_only || self.writes_blocked()) {
            return Err(-EROFS);
        }
        if mode & BLK_OPEN_EXCL != 0 {
            let holder = holder.ok_or(-EINVAL)?;
            self.prepare_to_claim(holder)?;
            self.exclusive_holder = Some(holder);
        } else if self.exclusive_holder.is_some() {
            return Err(-EBUSY);
        }
        self.open_count = self.open_count.saturating_add(1);
        Ok(())
    }

    pub fn release(&mut self, mode: u32, holder: Option<usize>) {
        self.open_count = self.open_count.saturating_sub(1);
        if mode & BLK_OPEN_EXCL != 0 && self.exclusive_holder == holder {
            self.exclusive_holder = None;
        }
    }

    pub fn prepare_to_claim(&self, holder: usize) -> Result<(), i32> {
        match self.exclusive_holder {
            Some(current) if current != holder => Err(-EBUSY),
            _ => Ok(()),
        }
    }

    pub fn abort_claiming(&mut self, holder: usize) {
        if self.exclusive_holder == Some(holder) {
            self.exclusive_holder = None;
        }
    }

    pub fn block_writes(&mut self) {
        self.writes_blocked = self.writes_blocked.saturating_add(1);
    }

    pub fn unblock_writes(&mut self) {
        self.writes_blocked = self.writes_blocked.saturating_sub(1);
    }

    pub fn mark_dead(&mut self, surprise: bool) {
        self.live = false;
        if surprise {
            self.read_only = true;
            self.block_writes();
        }
    }
}

pub fn blk_validate_block_size(block_size: u32) -> Result<(), i32> {
    if block_size < SECTOR_SIZE || block_size > BLK_MAX_BLOCK_SIZE || !block_size.is_power_of_two()
    {
        return Err(-EINVAL);
    }
    Ok(())
}

pub fn blksize_bits(size: u32) -> u32 {
    debug_assert!(size > 256);
    (size >> SECTOR_SHIFT).next_power_of_two().trailing_zeros() + SECTOR_SHIFT
}

pub fn blk_to_file_flags(mode: u32) -> u32 {
    let mut flags = 0;
    if mode & BLK_OPEN_WRITE != 0 {
        flags |= 0x1;
    }
    if mode & BLK_OPEN_NDELAY != 0 {
        flags |= 0x800;
    }
    flags
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SuperBlock {
    pub blocksize: u32,
    pub blocksize_bits: u32,
    pub supports_large_blocksize: bool,
}

impl SuperBlock {
    pub fn set_blocksize(&mut self, bdev: &mut Bdev, size: u32) -> u32 {
        if size > PAGE_SIZE && !self.supports_large_blocksize {
            return 0;
        }
        if bdev.set_blocksize(size).is_err() {
            return 0;
        }
        self.blocksize = size;
        self.blocksize_bits = blksize_bits(size);
        self.blocksize
    }

    pub fn min_blocksize(&mut self, bdev: &mut Bdev, size: u32) -> u32 {
        self.set_blocksize(bdev, size.max(bdev.logical_block_size))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocksize_validation_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/block/bdev.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/blkdev.h"
        ));
        assert!(header.contains("#define BLK_OPEN_EXCL"));
        assert!(header.contains("blk_validate_block_size(unsigned long bsize)"));
        assert!(header.contains("bsize < 512 || bsize > BLK_MAX_BLOCK_SIZE"));
        assert!(source.contains("bdev_validate_blocksize(struct block_device *bdev"));
        assert!(source.contains("block_size < bdev_logical_block_size(bdev)"));
        assert!(source.contains("set_init_blocksize(struct block_device *bdev)"));

        let mut bdev = Bdev::new(8, 1024, 16).unwrap();
        assert_eq!(bdev.validate_blocksize(512), Err(-EINVAL));
        assert_eq!(bdev.validate_blocksize(1024), Ok(()));
        assert_eq!(bdev.validate_blocksize(1000), Err(-EINVAL));
        bdev.set_blocksize(2048).unwrap();
        assert_eq!(bdev.block_size, 2048);
        assert_eq!(blksize_bits(4096), 12);
    }

    #[test]
    fn exclusive_claim_and_write_blocking_follow_bdev_rules() {
        let mut bdev = Bdev::new(1, 512, 128).unwrap();
        bdev.open(BLK_OPEN_READ | BLK_OPEN_EXCL, Some(7)).unwrap();
        assert_eq!(bdev.open(BLK_OPEN_READ, None), Err(-EBUSY));
        bdev.release(BLK_OPEN_EXCL, Some(7));
        assert_eq!(bdev.open(BLK_OPEN_READ, None), Ok(()));

        bdev.block_writes();
        assert_eq!(bdev.open(BLK_OPEN_WRITE, None), Err(-EROFS));
        bdev.unblock_writes();
        assert_eq!(bdev.open(BLK_OPEN_WRITE, None), Ok(()));

        bdev.mark_dead(true);
        assert_eq!(bdev.open(BLK_OPEN_READ, None), Err(-ENODEV));
    }
}

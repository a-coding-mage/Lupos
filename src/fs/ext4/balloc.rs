//! linux-parity: partial
//! linux-source: vendor/linux/fs/ext4/balloc.c
//! test-origin: linux:vendor/linux/fs/ext4/balloc.c
//! ext4 block-group descriptor parser.
//!
//! Mirrors `vendor/linux/fs/ext4/ext4.h::struct ext4_group_desc`.  We only
//! decode the fields the read path needs (block bitmap, inode bitmap,
//! inode table block).  64-bit fields are present when `INCOMPAT_64BIT` is
//! set and `s_desc_size >= 64`.

extern crate alloc;

pub const EXT4_MIN_DESC_SIZE: u32 = 32;
pub const EXT4_MIN_DESC_SIZE_64BIT: u32 = 64;

#[derive(Clone, Debug, Default)]
pub struct Ext4GroupDesc {
    pub bg_block_bitmap: u64,
    pub bg_inode_bitmap: u64,
    pub bg_inode_table: u64,
    pub bg_free_blocks_count: u32,
    pub bg_free_inodes_count: u32,
    pub bg_used_dirs_count: u32,
}

impl Ext4GroupDesc {
    /// Parse one descriptor.  `buf` must hold at least 32 bytes (legacy)
    /// or 64 bytes when `INCOMPAT_64BIT` is on.
    ///
    /// Linux source:
    /// - `vendor/linux/fs/ext4/super.c:328` (`ext4_block_bitmap`)
    /// - `vendor/linux/fs/ext4/super.c:344` (`ext4_inode_table`)
    /// - `vendor/linux/fs/ext4/super.c:5284` (`s_desc_size` selection)
    pub fn parse(buf: &[u8], desc_size: u32) -> Self {
        let read_u32 = |off: usize| -> u32 {
            u32::from_le_bytes([buf[off], buf[off + 1], buf[off + 2], buf[off + 3]])
        };
        let read_u16 = |off: usize| -> u16 { u16::from_le_bytes([buf[off], buf[off + 1]]) };
        let mut gd = Self::default();
        gd.bg_block_bitmap = read_u32(0) as u64;
        gd.bg_inode_bitmap = read_u32(4) as u64;
        gd.bg_inode_table = read_u32(8) as u64;
        gd.bg_free_blocks_count = read_u16(12) as u32;
        gd.bg_free_inodes_count = read_u16(14) as u32;
        gd.bg_used_dirs_count = read_u16(16) as u32;
        if desc_size >= EXT4_MIN_DESC_SIZE_64BIT && buf.len() >= EXT4_MIN_DESC_SIZE_64BIT as usize {
            gd.bg_block_bitmap |= (read_u32(32) as u64) << 32;
            gd.bg_inode_bitmap |= (read_u32(36) as u64) << 32;
            gd.bg_inode_table |= (read_u32(40) as u64) << 32;
            gd.bg_free_blocks_count |= (read_u16(44) as u32) << 16;
            gd.bg_free_inodes_count |= (read_u16(46) as u32) << 16;
            gd.bg_used_dirs_count |= (read_u16(48) as u32) << 16;
        }
        gd
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_group_desc_ignores_following_descriptor_bytes() {
        let mut buf = [0u8; 64];
        buf[0..4].copy_from_slice(&17u32.to_le_bytes());
        buf[4..8].copy_from_slice(&19u32.to_le_bytes());
        buf[8..12].copy_from_slice(&21u32.to_le_bytes());
        buf[32..36].copy_from_slice(&0x800u32.to_le_bytes());
        buf[36..40].copy_from_slice(&0x814u32.to_le_bytes());
        buf[40..44].copy_from_slice(&0x815u32.to_le_bytes());

        let gd = Ext4GroupDesc::parse(&buf, EXT4_MIN_DESC_SIZE);

        assert_eq!(gd.bg_block_bitmap, 17);
        assert_eq!(gd.bg_inode_bitmap, 19);
        assert_eq!(gd.bg_inode_table, 21);
    }

    #[test]
    fn sixty_four_bit_group_desc_combines_high_fields() {
        let mut buf = [0u8; 64];
        buf[0..4].copy_from_slice(&17u32.to_le_bytes());
        buf[4..8].copy_from_slice(&19u32.to_le_bytes());
        buf[8..12].copy_from_slice(&21u32.to_le_bytes());
        buf[32..36].copy_from_slice(&1u32.to_le_bytes());
        buf[36..40].copy_from_slice(&2u32.to_le_bytes());
        buf[40..44].copy_from_slice(&3u32.to_le_bytes());

        let gd = Ext4GroupDesc::parse(&buf, EXT4_MIN_DESC_SIZE_64BIT);

        assert_eq!(gd.bg_block_bitmap, (1u64 << 32) | 17);
        assert_eq!(gd.bg_inode_bitmap, (2u64 << 32) | 19);
        assert_eq!(gd.bg_inode_table, (3u64 << 32) | 21);
    }
}

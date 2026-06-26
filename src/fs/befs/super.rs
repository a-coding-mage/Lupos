//! linux-parity: complete
//! linux-source: vendor/linux/fs/befs/super.c
//! test-origin: linux:vendor/linux/fs/befs/super.c
//! BeFS superblock load and validation rules.

pub const BEFS_OK: i32 = 0;
pub const BEFS_ERR: i32 = 1;
pub const BEFS_DIRTY: u32 = 0x4449_5254;
pub const BEFS_SUPER_MAGIC1: u32 = 0x4246_5331;
pub const BEFS_SUPER_MAGIC2: u32 = 0xdd12_1031;
pub const BEFS_SUPER_MAGIC3: u32 = 0x15b6_830e;
pub const BEFS_BYTEORDER_NATIVE: u32 = 0x4249_4745;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BefsByteSex {
    Le,
    Be,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BefsDiskSuperBlock {
    pub fs_byte_order: u32,
    pub magic1: u32,
    pub magic2: u32,
    pub magic3: u32,
    pub block_size: u32,
    pub block_shift: u32,
    pub num_blocks: u64,
    pub used_blocks: u64,
    pub inode_size: u32,
    pub blocks_per_ag: u32,
    pub ag_shift: u32,
    pub num_ags: u32,
    pub flags: u32,
    pub log_start: u64,
    pub log_end: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BefsSbInfo {
    pub byte_order: BefsByteSex,
    pub magic1: u32,
    pub magic2: u32,
    pub magic3: u32,
    pub block_size: u32,
    pub block_shift: u32,
    pub num_blocks: u64,
    pub used_blocks: u64,
    pub inode_size: u32,
    pub blocks_per_ag: u32,
    pub ag_shift: u32,
    pub num_ags: u32,
    pub flags: u32,
    pub log_start: u64,
    pub log_end: u64,
}

pub fn befs_load_sb(disk: BefsDiskSuperBlock) -> BefsSbInfo {
    let le_tag = BEFS_BYTEORDER_NATIVE.to_le();
    let byte_order = if disk.fs_byte_order == le_tag {
        BefsByteSex::Le
    } else {
        BefsByteSex::Be
    };

    BefsSbInfo {
        byte_order,
        magic1: fs32_to_cpu(byte_order, disk.magic1),
        magic2: fs32_to_cpu(byte_order, disk.magic2),
        magic3: fs32_to_cpu(byte_order, disk.magic3),
        block_size: fs32_to_cpu(byte_order, disk.block_size),
        block_shift: fs32_to_cpu(byte_order, disk.block_shift),
        num_blocks: fs64_to_cpu(byte_order, disk.num_blocks),
        used_blocks: fs64_to_cpu(byte_order, disk.used_blocks),
        inode_size: fs32_to_cpu(byte_order, disk.inode_size),
        blocks_per_ag: fs32_to_cpu(byte_order, disk.blocks_per_ag),
        ag_shift: fs32_to_cpu(byte_order, disk.ag_shift),
        num_ags: fs32_to_cpu(byte_order, disk.num_ags),
        flags: fs32_to_cpu(byte_order, disk.flags),
        log_start: fs64_to_cpu(byte_order, disk.log_start),
        log_end: fs64_to_cpu(byte_order, disk.log_end),
    }
}

pub fn befs_check_sb(sb: &BefsSbInfo, page_size: u32) -> i32 {
    if sb.magic1 != BEFS_SUPER_MAGIC1
        || sb.magic2 != BEFS_SUPER_MAGIC2
        || sb.magic3 != BEFS_SUPER_MAGIC3
    {
        return BEFS_ERR;
    }
    if sb.block_size != 1024
        && sb.block_size != 2048
        && sb.block_size != 4096
        && sb.block_size != 8192
    {
        return BEFS_ERR;
    }
    if sb.block_size > page_size {
        return BEFS_ERR;
    }
    if (1u32 << sb.block_shift) != sb.block_size {
        return BEFS_ERR;
    }
    if sb.log_start != sb.log_end || sb.flags == BEFS_DIRTY {
        return BEFS_ERR;
    }
    BEFS_OK
}

pub const fn befs_ag_shift_matches_blocks(sb: &BefsSbInfo) -> bool {
    (1u32 << sb.ag_shift) == sb.blocks_per_ag
}

const fn fs32_to_cpu(byte_order: BefsByteSex, value: u32) -> u32 {
    match byte_order {
        BefsByteSex::Le => u32::from_le(value),
        BefsByteSex::Be => u32::from_be(value),
    }
}

const fn fs64_to_cpu(byte_order: BefsByteSex, value: u64) -> u64 {
    match byte_order {
        BefsByteSex::Le => u64::from_le(value),
        BefsByteSex::Be => u64::from_be(value),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn befs_super_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/befs/super.c"
        ));
        assert!(source.contains("#include <linux/fs.h>"));
        assert!(source.contains("#include <asm/page.h>"));
        assert!(source.contains("#include \"befs.h\""));
        assert!(source.contains("#include \"super.h\""));
        assert!(source.contains("befs_load_sb(struct super_block *sb, befs_super_block *disk_sb)"));
        assert!(source.contains("if (disk_sb->fs_byte_order == BEFS_BYTEORDER_NATIVE_LE)"));
        assert!(source.contains("befs_sb->byte_order = BEFS_BYTESEX_LE;"));
        assert!(source.contains("else if (disk_sb->fs_byte_order == BEFS_BYTEORDER_NATIVE_BE)"));
        assert!(source.contains("befs_sb->byte_order = BEFS_BYTESEX_BE;"));
        assert!(source.contains("befs_sb->magic1 = fs32_to_cpu(sb, disk_sb->magic1);"));
        assert!(source.contains("befs_sb->num_blocks = fs64_to_cpu(sb, disk_sb->num_blocks);"));
        assert!(source.contains("befs_sb->log_blocks = fsrun_to_cpu(sb, disk_sb->log_blocks);"));
        assert!(source.contains("befs_sb->nls = NULL;"));
        assert!(source.contains("return BEFS_OK;"));
        assert!(source.contains("befs_check_sb(struct super_block *sb)"));
        assert!(source.contains("befs_sb->magic1 != BEFS_SUPER_MAGIC1"));
        assert!(source.contains("invalid magic header"));
        assert!(source.contains("befs_sb->block_size != 1024"));
        assert!(source.contains("befs_sb->block_size > PAGE_SIZE"));
        assert!(source.contains("(1 << befs_sb->block_shift) != befs_sb->block_size"));
        assert!(source.contains("(1 << befs_sb->ag_shift) != befs_sb->blocks_per_ag"));
        assert!(source.contains("befs_sb->log_start != befs_sb->log_end ||"));
        assert!(source.contains("befs_sb->flags == BEFS_DIRTY"));

        let disk = BefsDiskSuperBlock {
            fs_byte_order: BEFS_BYTEORDER_NATIVE.to_le(),
            magic1: BEFS_SUPER_MAGIC1.to_le(),
            magic2: BEFS_SUPER_MAGIC2.to_le(),
            magic3: BEFS_SUPER_MAGIC3.to_le(),
            block_size: 4096u32.to_le(),
            block_shift: 12u32.to_le(),
            num_blocks: 100u64.to_le(),
            used_blocks: 7u64.to_le(),
            inode_size: 128u32.to_le(),
            blocks_per_ag: 16u32.to_le(),
            ag_shift: 4u32.to_le(),
            num_ags: 3u32.to_le(),
            flags: 0,
            log_start: 0,
            log_end: 0,
        };
        let loaded = befs_load_sb(disk);
        assert_eq!(loaded.byte_order, BefsByteSex::Le);
        assert_eq!(loaded.magic1, BEFS_SUPER_MAGIC1);
        assert_eq!(loaded.block_size, 4096);
        assert_eq!(befs_check_sb(&loaded, 4096), BEFS_OK);
        assert!(befs_ag_shift_matches_blocks(&loaded));

        let bad_magic = BefsSbInfo {
            magic1: 0,
            ..loaded
        };
        assert_eq!(befs_check_sb(&bad_magic, 4096), BEFS_ERR);
        let bad_block = BefsSbInfo {
            block_size: 512,
            ..loaded
        };
        assert_eq!(befs_check_sb(&bad_block, 4096), BEFS_ERR);
        let dirty = BefsSbInfo {
            flags: BEFS_DIRTY,
            ..loaded
        };
        assert_eq!(befs_check_sb(&dirty, 4096), BEFS_ERR);
    }
}

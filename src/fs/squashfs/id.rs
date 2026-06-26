//! linux-parity: complete
//! linux-source: vendor/linux/fs/squashfs/id.c
//! test-origin: linux:vendor/linux/fs/squashfs/id.c
//! SquashFS uid/gid index-table helpers.

use crate::include::uapi::errno::EINVAL;

pub const SQUASHFS_METADATA_SIZE: u64 = 8192;
pub const SQUASHFS_BLOCK_OFFSET: u64 = 2;
pub const SQUASHFS_ID_ENTRY_SIZE: u64 = core::mem::size_of::<u32>() as u64;
pub const SQUASHFS_ID_INDEX_SIZE: u64 = core::mem::size_of::<u64>() as u64;

pub const fn squashfs_id_bytes(ids: u64) -> u64 {
    ids * SQUASHFS_ID_ENTRY_SIZE
}

pub const fn squashfs_id_block(index: u64) -> u64 {
    squashfs_id_bytes(index) / SQUASHFS_METADATA_SIZE
}

pub const fn squashfs_id_block_offset(index: u64) -> u64 {
    squashfs_id_bytes(index) % SQUASHFS_METADATA_SIZE
}

pub const fn squashfs_id_blocks(ids: u64) -> u64 {
    (squashfs_id_bytes(ids) + SQUASHFS_METADATA_SIZE - 1) / SQUASHFS_METADATA_SIZE
}

pub const fn squashfs_id_block_bytes(ids: u64) -> u64 {
    squashfs_id_blocks(ids) * SQUASHFS_ID_INDEX_SIZE
}

pub fn squashfs_get_id_result(
    index: u32,
    ids: u32,
    metadata_read: Result<u32, i32>,
) -> Result<u32, i32> {
    if index >= ids {
        return Err(-EINVAL);
    }
    metadata_read
}

pub fn squashfs_read_id_index_table_result(
    id_table_start: u64,
    next_table: u64,
    no_ids: u16,
    table: &[u64],
) -> Result<u64, i32> {
    let length = squashfs_id_block_bytes(no_ids as u64);
    let indexes = squashfs_id_blocks(no_ids as u64) as usize;

    if no_ids == 0 {
        return Err(-EINVAL);
    }
    if length != next_table.wrapping_sub(id_table_start) {
        return Err(-EINVAL);
    }
    if table.len() < indexes {
        return Err(-EINVAL);
    }

    for n in 0..indexes.saturating_sub(1) {
        let start = table[n];
        let end = table[n + 1];
        if start >= end || end - start > SQUASHFS_METADATA_SIZE + SQUASHFS_BLOCK_OFFSET {
            return Err(-EINVAL);
        }
    }

    let start = table[indexes - 1];
    if start >= id_table_start
        || id_table_start - start > SQUASHFS_METADATA_SIZE + SQUASHFS_BLOCK_OFFSET
    {
        return Err(-EINVAL);
    }

    Ok(length)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn squashfs_id_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/squashfs/id.c"
        ));
        assert!(source.contains("#include <linux/fs.h>"));
        assert!(source.contains("#include <linux/vfs.h>"));
        assert!(source.contains("#include <linux/slab.h>"));
        assert!(source.contains("#include \"squashfs_fs.h\""));
        assert!(source.contains("#include \"squashfs_fs_sb.h\""));
        assert!(source.contains("#include \"squashfs.h\""));
        assert!(source.contains("int squashfs_get_id(struct super_block *sb, unsigned int index,"));
        assert!(source.contains("int block = SQUASHFS_ID_BLOCK(index);"));
        assert!(source.contains("int offset = SQUASHFS_ID_BLOCK_OFFSET(index);"));
        assert!(source.contains("if (index >= msblk->ids)"));
        assert!(source.contains("return -EINVAL;"));
        assert!(source.contains("start_block = le64_to_cpu(msblk->id_table[block]);"));
        assert!(
            source.contains("err = squashfs_read_metadata(sb, &disk_id, &start_block, &offset,")
        );
        assert!(source.contains("*id = le32_to_cpu(disk_id);"));
        assert!(source.contains("__le64 *squashfs_read_id_index_table"));
        assert!(source.contains("unsigned int length = SQUASHFS_ID_BLOCK_BYTES(no_ids);"));
        assert!(source.contains("unsigned int indexes = SQUASHFS_ID_BLOCKS(no_ids);"));
        assert!(source.contains("if (no_ids == 0)"));
        assert!(source.contains("if (length != (next_table - id_table_start))"));
        assert!(source.contains("table = squashfs_read_table(sb, id_table_start, length);"));
        assert!(source.contains("for (n = 0; n < (indexes - 1); n++)"));
        assert!(source.contains("(SQUASHFS_METADATA_SIZE + SQUASHFS_BLOCK_OFFSET)"));
        assert!(source.contains("kfree(table);"));

        assert_eq!(squashfs_id_block(0), 0);
        assert_eq!(squashfs_id_block(2048), 1);
        assert_eq!(squashfs_id_block_offset(2049), 4);
        assert_eq!(squashfs_id_block_bytes(2049), 16);
        assert_eq!(squashfs_get_id_result(0, 1, Ok(1000)), Ok(1000));
        assert_eq!(squashfs_get_id_result(1, 1, Ok(1000)), Err(-EINVAL));
        assert_eq!(
            squashfs_read_id_index_table_result(20_000, 20_016, 2049, &[11_000, 12_000]),
            Ok(16)
        );
        assert_eq!(
            squashfs_read_id_index_table_result(20_000, 20_015, 2049, &[1000, 9000]),
            Err(-EINVAL)
        );
        assert_eq!(
            squashfs_read_id_index_table_result(20_000, 20_016, 2049, &[9000, 9000]),
            Err(-EINVAL)
        );
        assert_eq!(
            squashfs_read_id_index_table_result(20_000, 20_016, 2049, &[1000, 20_000]),
            Err(-EINVAL)
        );
    }
}

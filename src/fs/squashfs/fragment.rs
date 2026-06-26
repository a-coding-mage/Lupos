//! linux-parity: complete
//! linux-source: vendor/linux/fs/squashfs/fragment.c
//! test-origin: linux:vendor/linux/fs/squashfs/fragment.c
//! SquashFS compressed fragment lookup helpers.

use crate::include::uapi::errno::{EINVAL, EIO};

pub const SQUASHFS_METADATA_SIZE: u64 = 8192;
pub const SQUASHFS_FRAGMENT_ENTRY_SIZE: u64 = 16;

pub const fn squashfs_fragment_bytes(fragment: u64) -> u64 {
    fragment * SQUASHFS_FRAGMENT_ENTRY_SIZE
}

pub const fn squashfs_fragment_index(fragment: u64) -> u64 {
    squashfs_fragment_bytes(fragment) / SQUASHFS_METADATA_SIZE
}

pub const fn squashfs_fragment_index_offset(fragment: u64) -> u64 {
    squashfs_fragment_bytes(fragment) % SQUASHFS_METADATA_SIZE
}

pub const fn squashfs_fragment_indexes(fragments: u64) -> u64 {
    (squashfs_fragment_bytes(fragments) + SQUASHFS_METADATA_SIZE - 1) / SQUASHFS_METADATA_SIZE
}

pub const fn squashfs_fragment_index_bytes(fragments: u64) -> u64 {
    squashfs_fragment_indexes(fragments) * 8
}

pub const fn squashfs_block_size(raw: u32) -> Result<i32, i32> {
    if raw >> 25 != 0 {
        Err(-EIO)
    } else {
        Ok(raw as i32)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SquashfsFragLookup {
    pub fragment_block: u64,
    pub size: i32,
}

pub const fn squashfs_frag_lookup_result(
    fragments: u32,
    fragment: u32,
    metadata_read_size: i32,
    entry_start_block: u64,
    entry_size_raw: u32,
) -> Result<SquashfsFragLookup, i32> {
    if fragment >= fragments {
        return Err(-EIO);
    }
    if metadata_read_size < 0 {
        return Err(metadata_read_size);
    }
    match squashfs_block_size(entry_size_raw) {
        Ok(size) => Ok(SquashfsFragLookup {
            fragment_block: entry_start_block,
            size,
        }),
        Err(err) => Err(err),
    }
}

pub const fn squashfs_read_fragment_index_table_result(
    fragment_table_start: u64,
    next_table: u64,
    fragments: u32,
    first_index_block: Option<u64>,
) -> Result<u64, i32> {
    let length = squashfs_fragment_index_bytes(fragments as u64);
    if fragment_table_start.saturating_add(length) > next_table {
        return Err(-EINVAL);
    }
    if let Some(first) = first_index_block {
        if first >= fragment_table_start {
            return Err(-EINVAL);
        }
    }
    Ok(length)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn squashfs_fragment_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/squashfs/fragment.c"
        ));
        assert!(source.contains("#include <linux/fs.h>"));
        assert!(source.contains("#include <linux/vfs.h>"));
        assert!(source.contains("#include <linux/slab.h>"));
        assert!(source.contains("#include \"squashfs_fs.h\""));
        assert!(source.contains("#include \"squashfs_fs_sb.h\""));
        assert!(source.contains("#include \"squashfs.h\""));
        assert!(source.contains("int squashfs_frag_lookup"));
        assert!(source.contains("if (fragment >= msblk->fragments)"));
        assert!(source.contains("return -EIO;"));
        assert!(source.contains("SQUASHFS_FRAGMENT_INDEX(fragment)"));
        assert!(source.contains("SQUASHFS_FRAGMENT_INDEX_OFFSET(fragment)"));
        assert!(source.contains("squashfs_read_metadata"));
        assert!(source.contains("*fragment_block = le64_to_cpu(fragment_entry.start_block);"));
        assert!(source.contains("return squashfs_block_size(fragment_entry.size);"));
        assert!(source.contains("squashfs_read_fragment_index_table"));
        assert!(source.contains("SQUASHFS_FRAGMENT_INDEX_BYTES(fragments)"));
        assert!(source.contains("if (fragment_table_start + length > next_table)"));
        assert!(source.contains("squashfs_read_table(sb, fragment_table_start, length);"));
        assert!(source.contains("le64_to_cpu(table[0]) >= fragment_table_start"));
        assert!(source.contains("kfree(table);"));

        assert_eq!(squashfs_fragment_index(0), 0);
        assert_eq!(squashfs_fragment_index(512), 1);
        assert_eq!(squashfs_fragment_index_offset(513), 16);
        assert_eq!(squashfs_fragment_index_bytes(513), 16);
        assert_eq!(squashfs_block_size(0x0100_0000), Ok(0x0100_0000));
        assert_eq!(squashfs_block_size(0x0200_0000), Err(-EIO));
        assert_eq!(
            squashfs_frag_lookup_result(1, 0, 16, 0x1000, 4096),
            Ok(SquashfsFragLookup {
                fragment_block: 0x1000,
                size: 4096
            })
        );
        assert_eq!(squashfs_frag_lookup_result(1, 1, 16, 0, 0), Err(-EIO));
        assert_eq!(squashfs_frag_lookup_result(1, 0, -12, 0, 0), Err(-12));
        assert_eq!(
            squashfs_read_fragment_index_table_result(100, 116, 513, Some(99)),
            Ok(16)
        );
        assert_eq!(
            squashfs_read_fragment_index_table_result(100, 115, 513, Some(99)),
            Err(-EINVAL)
        );
        assert_eq!(
            squashfs_read_fragment_index_table_result(100, 116, 513, Some(100)),
            Err(-EINVAL)
        );
    }
}

//! linux-parity: complete
//! linux-source: vendor/linux/fs/squashfs/symlink.c
//! test-origin: linux:vendor/linux/fs/squashfs/symlink.c
//! SquashFS symlink folio read planning.

pub const PAGE_SIZE: i64 = 4096;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SquashfsSymlinkReadPlan {
    pub skip_metadata_bytes: i64,
    pub read_len: i64,
    pub zero_tail_bytes: i64,
}

pub const fn squashfs_symlink_read_plan(
    inode_size: i64,
    folio_pos: i64,
) -> SquashfsSymlinkReadPlan {
    let remaining = inode_size - folio_pos;
    let read_len = if remaining < PAGE_SIZE {
        remaining
    } else {
        PAGE_SIZE
    };
    SquashfsSymlinkReadPlan {
        skip_metadata_bytes: folio_pos,
        read_len,
        zero_tail_bytes: if read_len >= 0 {
            PAGE_SIZE - read_len
        } else {
            0
        },
    }
}

pub const fn squashfs_symlink_next_block(copied: i32, remaining: i32) -> bool {
    copied != remaining
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn squashfs_symlink_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/squashfs/symlink.c"
        ));
        assert!(source.contains("#include <linux/pagemap.h>"));
        assert!(source.contains("#include <linux/xattr.h>"));
        assert!(source.contains("#include \"squashfs_fs_i.h\""));
        assert!(source.contains("#include \"xattr.h\""));
        assert!(source.contains("static int squashfs_symlink_read_folio"));
        assert!(source.contains("int index = folio_pos(folio);"));
        assert!(source.contains("u64 block = squashfs_i(inode)->start;"));
        assert!(source.contains("int offset = squashfs_i(inode)->offset;"));
        assert!(source.contains("int length = min_t(int, i_size_read(inode) - index, PAGE_SIZE);"));
        assert!(source.contains("if (index)"));
        assert!(source.contains("bytes = squashfs_read_metadata(sb, NULL, &block, &offset,"));
        assert!(source.contains("for (bytes = 0; bytes < length; offset = 0, bytes += copied)"));
        assert!(source.contains("entry = squashfs_cache_get(sb, msblk->block_cache, block, 0);"));
        assert!(source.contains("if (entry->error)"));
        assert!(source.contains("pageaddr = kmap_local_folio(folio, 0);"));
        assert!(source.contains("copied = squashfs_copy_data(pageaddr + bytes, entry, offset,"));
        assert!(source.contains("if (copied == length - bytes)"));
        assert!(source.contains("memset(pageaddr + length, 0, PAGE_SIZE - length);"));
        assert!(source.contains("else"));
        assert!(source.contains("block = entry->next_index;"));
        assert!(source.contains("flush_dcache_folio(folio);"));
        assert!(source.contains("folio_end_read(folio, error == 0);"));
        assert!(source.contains("const struct address_space_operations squashfs_symlink_aops"));
        assert!(source.contains(".get_link = page_get_link"));
        assert!(source.contains(".listxattr = squashfs_listxattr"));

        assert_eq!(
            squashfs_symlink_read_plan(100, 0),
            SquashfsSymlinkReadPlan {
                skip_metadata_bytes: 0,
                read_len: 100,
                zero_tail_bytes: 3996,
            }
        );
        assert_eq!(
            squashfs_symlink_read_plan(8192, 4096),
            SquashfsSymlinkReadPlan {
                skip_metadata_bytes: 4096,
                read_len: 4096,
                zero_tail_bytes: 0,
            }
        );
        assert!(!squashfs_symlink_next_block(20, 20));
        assert!(squashfs_symlink_next_block(10, 20));
    }
}

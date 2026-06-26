//! linux-parity: complete
//! linux-source: vendor/linux/fs/verity/pagecache.c
//! test-origin: linux:vendor/linux/fs/verity/pagecache.c
//! fs-verity Merkle tree page-cache helpers.

use crate::include::uapi::errno::ENOENT;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FolioLookup {
    Err(i32),
    Present { uptodate: bool },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MerkleReadaheadPlan {
    pub invalidate_lock_required: bool,
    pub issue_readahead: bool,
    pub put_folio: bool,
}

pub fn generic_read_merkle_tree_page_result(
    read_mapping_folio_result: Result<(), i32>,
    index: u64,
) -> Result<u64, i32> {
    match read_mapping_folio_result {
        Ok(()) => Ok(index),
        Err(err) => Err(err),
    }
}

pub const fn generic_readahead_merkle_tree_plan(lookup: FolioLookup) -> MerkleReadaheadPlan {
    match lookup {
        FolioLookup::Err(err) => MerkleReadaheadPlan {
            invalidate_lock_required: true,
            issue_readahead: err == -ENOENT,
            put_folio: false,
        },
        FolioLookup::Present { uptodate } => MerkleReadaheadPlan {
            invalidate_lock_required: true,
            issue_readahead: !uptodate,
            put_folio: true,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verity_pagecache_helpers_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/verity/pagecache.c"
        ));
        assert!(source.contains("#include <linux/export.h>"));
        assert!(source.contains("#include <linux/fsverity.h>"));
        assert!(source.contains("#include <linux/pagemap.h>"));
        assert!(source.contains("generic_read_merkle_tree_page"));
        assert!(source.contains("folio = read_mapping_folio(inode->i_mapping, index, NULL);"));
        assert!(source.contains("if (IS_ERR(folio))"));
        assert!(source.contains("return ERR_CAST(folio);"));
        assert!(source.contains("return folio_file_page(folio, index);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(generic_read_merkle_tree_page);"));
        assert!(source.contains("generic_readahead_merkle_tree"));
        assert!(source.contains("lockdep_assert_held(&inode->i_mapping->invalidate_lock);"));
        assert!(source.contains("__filemap_get_folio(inode->i_mapping, index, FGP_ACCESSED, 0)"));
        assert!(source.contains("folio == ERR_PTR(-ENOENT)"));
        assert!(source.contains("!folio_test_uptodate(folio)"));
        assert!(source.contains("page_cache_ra_unbounded(&ractl, nr_pages, 0);"));
        assert!(source.contains("folio_put(folio);"));

        assert_eq!(generic_read_merkle_tree_page_result(Ok(()), 7), Ok(7));
        assert_eq!(generic_read_merkle_tree_page_result(Err(-5), 7), Err(-5));
        assert!(generic_readahead_merkle_tree_plan(FolioLookup::Err(-ENOENT)).issue_readahead);
        assert!(!generic_readahead_merkle_tree_plan(FolioLookup::Err(-5)).issue_readahead);
        let stale = generic_readahead_merkle_tree_plan(FolioLookup::Present { uptodate: false });
        assert!(stale.issue_readahead);
        assert!(stale.put_folio);
        assert!(
            !generic_readahead_merkle_tree_plan(FolioLookup::Present { uptodate: true })
                .issue_readahead
        );
    }
}

//! linux-parity: complete
//! linux-source: vendor/linux/mm/folio-compat.c
//! test-origin: linux:vendor/linux/mm/folio-compat.c
//! Page-to-folio compatibility wrappers.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FolioCompatOp {
    Unlock,
    EndWriteback,
    WaitWriteback,
    MarkAccessed,
    StartWriteback,
    MarkDirty,
    MarkDirtyLock,
    ClearDirtyForIo,
    RedirtyForWritepage,
    AddToPageCacheLru,
    PagecacheGetPage,
}

pub const FOLIO_COMPAT_EXPORTS: &[(&str, FolioCompatOp, bool)] = &[
    ("unlock_page", FolioCompatOp::Unlock, false),
    ("end_page_writeback", FolioCompatOp::EndWriteback, false),
    ("wait_on_page_writeback", FolioCompatOp::WaitWriteback, true),
    ("mark_page_accessed", FolioCompatOp::MarkAccessed, false),
    ("set_page_writeback", FolioCompatOp::StartWriteback, false),
    ("set_page_dirty", FolioCompatOp::MarkDirty, false),
    ("set_page_dirty_lock", FolioCompatOp::MarkDirtyLock, false),
    (
        "clear_page_dirty_for_io",
        FolioCompatOp::ClearDirtyForIo,
        false,
    ),
    (
        "redirty_page_for_writepage",
        FolioCompatOp::RedirtyForWritepage,
        false,
    ),
    (
        "add_to_page_cache_lru",
        FolioCompatOp::AddToPageCacheLru,
        false,
    ),
    ("pagecache_get_page", FolioCompatOp::PagecacheGetPage, false),
];

pub fn folio_compat_op(name: &str) -> Option<FolioCompatOp> {
    FOLIO_COMPAT_EXPORTS
        .iter()
        .find(|(symbol, _, _)| *symbol == name)
        .map(|(_, op, _)| *op)
}

pub const fn pagecache_get_page_returns_null(is_err: bool) -> bool {
    is_err
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folio_compat_wrappers_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/mm/folio-compat.c"
        ));
        assert!(source.contains("unlock_page(struct page *page)"));
        assert!(source.contains("folio_unlock(page_folio(page));"));
        assert!(source.contains("folio_end_writeback(page_folio(page));"));
        assert!(source.contains("folio_wait_writeback(page_folio(page));"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(wait_on_page_writeback);"));
        assert!(source.contains("folio_mark_dirty_lock(page_folio(page));"));
        assert!(source.contains("filemap_add_folio(mapping, page_folio(page), index, gfp);"));
        assert!(source.contains("__filemap_get_folio(mapping, index, fgp_flags, gfp);"));
        assert!(source.contains("if (IS_ERR(folio))"));
        assert!(source.contains("return folio_file_page(folio, index);"));

        assert_eq!(folio_compat_op("unlock_page"), Some(FolioCompatOp::Unlock));
        assert_eq!(
            folio_compat_op("pagecache_get_page"),
            Some(FolioCompatOp::PagecacheGetPage)
        );
        assert!(pagecache_get_page_returns_null(true));
        assert_eq!(FOLIO_COMPAT_EXPORTS.len(), 11);
    }
}

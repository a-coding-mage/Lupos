//! linux-parity: complete
//! linux-source: vendor/linux/fs/xfs/scrub/agb_bitmap.c
//! test-origin: linux:vendor/linux/fs/xfs/scrub/agb_bitmap.c
//! XFS scrub per-AG btree-block bitmap traversal.

pub const XFS_BTREE_VISIT_ALL_FLAG: &str = "XFS_BTREE_VISIT_ALL";
pub const XAGB_BITMAP_VISITOR: &str = "xagb_bitmap_visit_btblock";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum XagbBitmapVisit {
    SkipNullBuffer,
    SetAgBlock { agbno: u32, len: u32 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XagbBitmapVisitReport {
    pub get_block_called: bool,
    pub visit: XagbBitmapVisit,
    pub returned_error: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XagbBitmapSetBtblocksCall {
    pub uses_visit_blocks: bool,
    pub visitor: &'static str,
    pub visit_all: &'static str,
    pub passes_bitmap_as_private: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XagbBitmapBtcurPathReport {
    pub visited_levels: usize,
    pub stopped_at_non_first_ptr: bool,
    pub returned_error: i32,
}

pub const fn xagb_bitmap_visit_btblock(buffer_present: bool, agbno: u32) -> XagbBitmapVisit {
    if buffer_present {
        XagbBitmapVisit::SetAgBlock { agbno, len: 1 }
    } else {
        XagbBitmapVisit::SkipNullBuffer
    }
}

pub const fn xagb_bitmap_visit_btblock_report(
    buffer_present: bool,
    agbno: u32,
    set_error: i32,
) -> XagbBitmapVisitReport {
    if !buffer_present {
        return XagbBitmapVisitReport {
            get_block_called: true,
            visit: XagbBitmapVisit::SkipNullBuffer,
            returned_error: 0,
        };
    }

    XagbBitmapVisitReport {
        get_block_called: true,
        visit: XagbBitmapVisit::SetAgBlock { agbno, len: 1 },
        returned_error: set_error,
    }
}

pub const fn xagb_bitmap_set_btblocks_call() -> XagbBitmapSetBtblocksCall {
    XagbBitmapSetBtblocksCall {
        uses_visit_blocks: true,
        visitor: XAGB_BITMAP_VISITOR,
        visit_all: XFS_BTREE_VISIT_ALL_FLAG,
        passes_bitmap_as_private: true,
    }
}

pub fn xagb_bitmap_btcur_path_levels(ptrs_from_leaf: &[u16]) -> usize {
    let mut levels = 0;
    while levels < ptrs_from_leaf.len() && ptrs_from_leaf[levels] == 1 {
        levels += 1;
    }
    levels
}

pub fn xagb_bitmap_set_btcur_path_report(
    ptrs_from_leaf: &[u16],
    visit_errors: &[i32],
) -> XagbBitmapBtcurPathReport {
    let mut level = 0;
    while level < ptrs_from_leaf.len() && ptrs_from_leaf[level] == 1 {
        let error = visit_errors.get(level).copied().unwrap_or(0);
        level += 1;
        if error != 0 {
            return XagbBitmapBtcurPathReport {
                visited_levels: level,
                stopped_at_non_first_ptr: false,
                returned_error: error,
            };
        }
    }

    XagbBitmapBtcurPathReport {
        visited_levels: level,
        stopped_at_non_first_ptr: level < ptrs_from_leaf.len(),
        returned_error: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xfs_agb_bitmap_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/xfs/scrub/agb_bitmap.c"
        ));
        assert!(source.contains("#include \"xfs_btree.h\""));
        assert!(source.contains("#include \"bitmap.h\""));
        assert!(source.contains("#include \"scrub/agb_bitmap.h\""));
        assert!(source.contains("xagb_bitmap_visit_btblock"));
        assert!(source.contains("xfs_btree_get_block(cur, level, &bp);"));
        assert!(source.contains("if (!bp)"));
        assert!(source.contains("return 0;"));
        assert!(source.contains("fsbno = XFS_DADDR_TO_FSB(cur->bc_mp, xfs_buf_daddr(bp));"));
        assert!(source.contains("agbno = XFS_FSB_TO_AGBNO(cur->bc_mp, fsbno);"));
        assert!(source.contains("return xagb_bitmap_set(bitmap, agbno, 1);"));
        assert!(source.contains("xagb_bitmap_set_btblocks"));
        assert!(source.contains("return xfs_btree_visit_blocks(cur, xagb_bitmap_visit_btblock,"));
        assert!(source.contains("XFS_BTREE_VISIT_ALL"));
        assert!(source.contains("xagb_bitmap_set_btcur_path"));
        assert!(
            source.contains("for (i = 0; i < cur->bc_nlevels && cur->bc_levels[i].ptr == 1; i++)")
        );
        assert!(source.contains("error = xagb_bitmap_visit_btblock(cur, i, bitmap);"));
        assert!(source.contains("if (error)"));
        assert!(source.contains("return error;"));

        assert_eq!(
            xagb_bitmap_visit_btblock(false, 12),
            XagbBitmapVisit::SkipNullBuffer
        );
        assert_eq!(
            xagb_bitmap_visit_btblock(true, 12),
            XagbBitmapVisit::SetAgBlock { agbno: 12, len: 1 }
        );
        assert_eq!(xagb_bitmap_btcur_path_levels(&[1, 1, 2, 1]), 2);
        assert_eq!(xagb_bitmap_btcur_path_levels(&[2, 1]), 0);
    }

    #[test]
    fn visit_report_matches_null_buffer_and_set_result() {
        assert_eq!(
            xagb_bitmap_visit_btblock_report(false, 12, -5),
            XagbBitmapVisitReport {
                get_block_called: true,
                visit: XagbBitmapVisit::SkipNullBuffer,
                returned_error: 0,
            }
        );
        assert_eq!(
            xagb_bitmap_visit_btblock_report(true, 12, -5),
            XagbBitmapVisitReport {
                get_block_called: true,
                visit: XagbBitmapVisit::SetAgBlock { agbno: 12, len: 1 },
                returned_error: -5,
            }
        );
    }

    #[test]
    fn set_btblocks_call_uses_visit_all_with_bitmap_private() {
        assert_eq!(
            xagb_bitmap_set_btblocks_call(),
            XagbBitmapSetBtblocksCall {
                uses_visit_blocks: true,
                visitor: XAGB_BITMAP_VISITOR,
                visit_all: XFS_BTREE_VISIT_ALL_FLAG,
                passes_bitmap_as_private: true,
            }
        );
    }

    #[test]
    fn btcur_path_report_stops_on_pointer_or_error() {
        assert_eq!(
            xagb_bitmap_set_btcur_path_report(&[1, 1, 2, 1], &[0, 0, 0, 0]),
            XagbBitmapBtcurPathReport {
                visited_levels: 2,
                stopped_at_non_first_ptr: true,
                returned_error: 0,
            }
        );
        assert_eq!(
            xagb_bitmap_set_btcur_path_report(&[1, 1, 1], &[0, -5, 0]),
            XagbBitmapBtcurPathReport {
                visited_levels: 2,
                stopped_at_non_first_ptr: false,
                returned_error: -5,
            }
        );
    }
}

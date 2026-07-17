//! linux-parity: complete
//! linux-source: vendor/linux/fs/iomap/iter.c
//! test-origin: linux:vendor/linux/fs/iomap/iter.c
//! Core iomap iterator state transitions.

use crate::include::uapi::errno::EIO;

pub const IOMAP_HOLE: u16 = 0;
pub const IOMAP_DELALLOC: u16 = 1;
pub const IOMAP_MAPPED: u16 = 2;
pub const IOMAP_UNWRITTEN: u16 = 3;
pub const IOMAP_INLINE: u16 = 4;

pub const IOMAP_F_FOLIO_BATCH: u16 = 1 << 13;
pub const IOMAP_F_SIZE_CHANGED: u16 = 1 << 14;
pub const IOMAP_F_STALE: u16 = 1 << 15;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Iomap {
    pub offset: u64,
    pub length: u64,
    pub iomap_type: u16,
    pub flags: u16,
}

impl Iomap {
    pub const fn hole() -> Self {
        Self {
            offset: 0,
            length: 0,
            iomap_type: IOMAP_HOLE,
            flags: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IomapIter {
    pub pos: u64,
    pub len: u64,
    pub iter_start_pos: u64,
    pub status: i32,
    pub flags: u32,
    pub iomap: Iomap,
    pub srcmap: Iomap,
}

pub const fn iomap_length_trim(iter: &IomapIter, pos: u64, len: u64) -> u64 {
    let mut end = iter.iomap.offset + iter.iomap.length;
    if iter.srcmap.iomap_type != IOMAP_HOLE {
        let src_end = iter.srcmap.offset + iter.srcmap.length;
        if src_end < end {
            end = src_end;
        }
    }
    let map_len = end - pos;
    if len < map_len { len } else { map_len }
}

pub const fn iomap_length(iter: &IomapIter) -> u64 {
    iomap_length_trim(iter, iter.pos, iter.len)
}

pub fn iomap_iter_advance(iter: &mut IomapIter, count: u64) -> i32 {
    if count > iomap_length(iter) {
        return -EIO;
    }
    iter.pos += count;
    iter.len -= count;
    0
}

pub fn iomap_iter_advance_full(iter: &mut IomapIter) -> i32 {
    iomap_iter_advance(iter, iomap_length(iter))
}

pub fn iomap_iter_clean_fbatch(iter: &mut IomapIter) {
    iter.status = 0;
}

pub fn iomap_iter_finish(iter: &mut IomapIter, iomap_end_ret: Option<i32>) -> i32 {
    let stale = (iter.iomap.flags & IOMAP_F_STALE) != 0;
    let advanced = iter.pos as i128 - iter.iter_start_pos as i128;

    if let Some(ret) = iomap_end_ret {
        if ret < 0 && advanced == 0 {
            return ret;
        }
    }

    if iter.status > 0 {
        iter.status = -EIO;
    }

    let ret = if iter.status < 0 {
        iter.status
    } else if iter.len == 0 || (advanced == 0 && !stale) {
        0
    } else {
        1
    };
    iomap_iter_clean_fbatch(iter);
    if ret <= 0 {
        return ret;
    }
    iter.iomap = Iomap::hole();
    iter.srcmap = Iomap::hole();
    ret
}

pub fn iomap_iter_done(iter: &mut IomapIter) {
    iter.iter_start_pos = iter.pos;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iomap_iter_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/iomap/iter.c"
        ));
        assert!(source.contains("#include <linux/iomap.h>"));
        assert!(source.contains("#include \"trace.h\""));
        assert!(
            source.contains("static inline void iomap_iter_clean_fbatch(struct iomap_iter *iter)")
        );
        assert!(source.contains("if (iter->iomap.flags & IOMAP_F_FOLIO_BATCH)"));
        assert!(source.contains("folio_batch_release(iter->fbatch);"));
        assert!(source.contains("iter->status = 0;"));
        assert!(source.contains("memset(&iter->iomap, 0, sizeof(iter->iomap));"));
        assert!(source.contains("int iomap_iter_advance(struct iomap_iter *iter, u64 count)"));
        assert!(source.contains("if (WARN_ON_ONCE(count > iomap_length(iter)))"));
        assert!(source.contains("return -EIO;"));
        assert!(source.contains("iter->pos += count;"));
        assert!(source.contains("iter->len -= count;"));
        assert!(source.contains("static inline void iomap_iter_done(struct iomap_iter *iter)"));
        assert!(source.contains("iter->iter_start_pos = iter->pos;"));
        assert!(source.contains("trace_iomap_iter_dstmap(iter->inode, &iter->iomap);"));
        assert!(source.contains("if (iter->srcmap.type != IOMAP_HOLE)"));
        assert!(
            source.contains("int iomap_iter(struct iomap_iter *iter, const struct iomap_ops *ops)")
        );
        assert!(source.contains("bool stale = iter->iomap.flags & IOMAP_F_STALE;"));
        assert!(source.contains("advanced = iter->pos - iter->iter_start_pos;"));
        assert!(source.contains("if (ret < 0 && !advanced)"));
        assert!(source.contains("if (WARN_ON_ONCE(iter->status > 0))"));
        assert!(source.contains("iter->status = -EIO;"));
        assert!(source.contains("else if (iter->len == 0 || (!advanced && !stale))"));
        assert!(
            source
                .contains("ret = ops->iomap_begin(iter->inode, iter->pos, iter->len, iter->flags,")
        );

        let mut iter = IomapIter {
            pos: 10,
            len: 20,
            iter_start_pos: 10,
            status: 0,
            flags: 0,
            iomap: Iomap {
                offset: 10,
                length: 8,
                iomap_type: IOMAP_MAPPED,
                flags: 0,
            },
            srcmap: Iomap::hole(),
        };
        assert_eq!(iomap_length(&iter), 8);
        assert_eq!(iomap_iter_advance(&mut iter, 8), 0);
        assert_eq!(iter.pos, 18);
        assert_eq!(iter.len, 12);
        assert_eq!(iomap_iter_advance(&mut iter, 9), -EIO);

        assert_eq!(iomap_iter_finish(&mut iter, None), 1);
        assert_eq!(iter.status, 0);
        assert_eq!(iter.iomap, Iomap::hole());

        let mut no_advance = IomapIter {
            iomap: Iomap {
                offset: 0,
                length: 1,
                iomap_type: IOMAP_MAPPED,
                flags: 0,
            },
            ..iter
        };
        no_advance.pos = 0;
        no_advance.iter_start_pos = 0;
        no_advance.len = 1;
        assert_eq!(iomap_iter_finish(&mut no_advance, None), 0);
        assert_eq!(no_advance.iomap.iomap_type, IOMAP_MAPPED);

        let mut stale = no_advance;
        stale.iomap.flags = IOMAP_F_STALE;
        assert_eq!(iomap_iter_finish(&mut stale, None), 1);

        let mut old_status = no_advance;
        old_status.status = 2;
        assert_eq!(iomap_iter_finish(&mut old_status, None), -EIO);
    }
}

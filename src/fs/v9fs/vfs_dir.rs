//! linux-parity: complete
//! linux-source: vendor/linux/fs/9p/vfs_dir.c
//! test-origin: linux:vendor/linux/fs/9p/vfs_dir.c
//! 9P directory entry typing, readdir buffer state, and release decisions.

use crate::include::uapi::errno::{EIO, ENOMEM};

use super::types::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RdirState {
    pub head: usize,
    pub tail: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DirReleasePlan {
    pub writeback_regular_file: bool,
    pub remove_open_fid_from_inode: bool,
    pub put_fid: bool,
    pub fscache_write_close: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RdirBufferPlan {
    pub returned_existing: bool,
    pub allocation_bytes: usize,
    pub errno: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReaddirRefillResult {
    pub state: RdirState,
    pub read_called: bool,
    pub ret: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LegacyDirentStep {
    pub state: RdirState,
    pub ctx_pos: u64,
    pub emitted: bool,
    pub stat_freed: bool,
    pub ret: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DotlDirentStep {
    pub state: RdirState,
    pub ctx_pos: u64,
    pub emitted: bool,
    pub ret: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DirReleaseResult {
    pub plan: DirReleasePlan,
    pub filemap_fdatawrite_called: bool,
    pub hlist_del: bool,
    pub spin_lock_inode: bool,
    pub fscache_version_size_supplied: bool,
    pub ret: i32,
}

pub const FMODE_WRITE: u32 = 0x2;

pub const fn dt_type(p9_mode: u32) -> u8 {
    if p9_mode & P9_DMDIR != 0 {
        DT_DIR
    } else if p9_mode & P9_DMSYMLINK != 0 {
        DT_LNK
    } else {
        DT_REG
    }
}

pub const fn rdir_needs_refill(state: RdirState) -> bool {
    state.tail == state.head
}

pub const fn v9fs_alloc_rdir_buf_plan(
    existing_rdir: bool,
    buflen: usize,
    allocation_succeeds: bool,
) -> RdirBufferPlan {
    if existing_rdir {
        return RdirBufferPlan {
            returned_existing: true,
            allocation_bytes: 0,
            errno: 0,
        };
    }
    if !allocation_succeeds {
        return RdirBufferPlan {
            returned_existing: false,
            allocation_bytes: core::mem::size_of::<RdirState>() + buflen,
            errno: -ENOMEM,
        };
    }
    RdirBufferPlan {
        returned_existing: false,
        allocation_bytes: core::mem::size_of::<RdirState>() + buflen,
        errno: 0,
    }
}

pub const fn legacy_readdir_refill(
    state: RdirState,
    read_len: i32,
    read_errno: i32,
) -> ReaddirRefillResult {
    if !rdir_needs_refill(state) {
        return ReaddirRefillResult {
            state,
            read_called: false,
            ret: 1,
        };
    }
    if read_errno != 0 {
        return ReaddirRefillResult {
            state,
            read_called: true,
            ret: read_errno,
        };
    }
    if read_len == 0 {
        return ReaddirRefillResult {
            state,
            read_called: true,
            ret: 0,
        };
    }
    ReaddirRefillResult {
        state: RdirState {
            head: 0,
            tail: read_len as usize,
        },
        read_called: true,
        ret: 1,
    }
}

pub const fn dotl_readdir_refill(state: RdirState, readdir_ret: i32) -> ReaddirRefillResult {
    if !rdir_needs_refill(state) {
        return ReaddirRefillResult {
            state,
            read_called: false,
            ret: 1,
        };
    }
    if readdir_ret <= 0 {
        return ReaddirRefillResult {
            state,
            read_called: true,
            ret: readdir_ret,
        };
    }
    ReaddirRefillResult {
        state: RdirState {
            head: 0,
            tail: readdir_ret as usize,
        },
        read_called: true,
        ret: 1,
    }
}

pub fn legacy_readdir_advance(
    state: RdirState,
    stat_record_len: usize,
    ctx_pos: u64,
) -> (RdirState, u64) {
    (
        RdirState {
            head: state.head + stat_record_len,
            tail: state.tail,
        },
        ctx_pos + stat_record_len as u64,
    )
}

pub const fn legacy_dirent_step(
    state: RdirState,
    stat_read_ret: i32,
    dir_emit_accepted: bool,
    ctx_pos: u64,
) -> LegacyDirentStep {
    if stat_read_ret <= 0 {
        return LegacyDirentStep {
            state,
            ctx_pos,
            emitted: false,
            stat_freed: false,
            ret: -EIO,
        };
    }
    if !dir_emit_accepted {
        return LegacyDirentStep {
            state,
            ctx_pos,
            emitted: false,
            stat_freed: true,
            ret: 0,
        };
    }
    LegacyDirentStep {
        state: RdirState {
            head: state.head + stat_read_ret as usize,
            tail: state.tail,
        },
        ctx_pos: ctx_pos + stat_read_ret as u64,
        emitted: true,
        stat_freed: true,
        ret: 1,
    }
}

pub fn dotl_readdir_advance(state: RdirState, dirent_len: usize, d_off: u64) -> (RdirState, u64) {
    (
        RdirState {
            head: state.head + dirent_len,
            tail: state.tail,
        },
        d_off,
    )
}

pub const fn dotl_dirent_step(
    state: RdirState,
    dirent_read_ret: i32,
    dir_emit_accepted: bool,
    d_off: u64,
    ctx_pos: u64,
) -> DotlDirentStep {
    if dirent_read_ret < 0 {
        return DotlDirentStep {
            state,
            ctx_pos,
            emitted: false,
            ret: -EIO,
        };
    }
    if !dir_emit_accepted {
        return DotlDirentStep {
            state,
            ctx_pos,
            emitted: false,
            ret: 0,
        };
    }
    DotlDirentStep {
        state: RdirState {
            head: state.head + dirent_read_ret as usize,
            tail: state.tail,
        },
        ctx_pos: d_off,
        emitted: true,
        ret: 1,
    }
}

pub fn dir_release_plan(fid_present: bool, inode_mode: u32, file_mode: u32) -> DirReleasePlan {
    DirReleasePlan {
        writeback_regular_file: fid_present && is_reg(inode_mode) && file_mode & FMODE_WRITE != 0,
        remove_open_fid_from_inode: fid_present,
        put_fid: fid_present,
        fscache_write_close: file_mode & FMODE_WRITE != 0,
    }
}

pub fn dir_release_result(
    fid_present: bool,
    inode_mode: u32,
    file_mode: u32,
    filemap_fdatawrite_ret: i32,
    p9_fid_put_ret: i32,
) -> DirReleaseResult {
    let plan = dir_release_plan(fid_present, inode_mode, file_mode);
    let mut ret = 0;
    if plan.writeback_regular_file {
        ret = filemap_fdatawrite_ret;
    }
    if plan.put_fid && ret >= 0 {
        ret = p9_fid_put_ret;
    }

    DirReleaseResult {
        plan,
        filemap_fdatawrite_called: plan.writeback_regular_file,
        hlist_del: fid_present,
        spin_lock_inode: fid_present,
        fscache_version_size_supplied: file_mode & FMODE_WRITE != 0,
        ret,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DirFileOperations {
    pub read: &'static str,
    pub llseek: &'static str,
    pub iterate_shared: &'static str,
    pub open: &'static str,
    pub release: &'static str,
    pub fsync: Option<&'static str>,
}

pub const V9FS_DIR_OPERATIONS: DirFileOperations = DirFileOperations {
    read: "generic_read_dir",
    llseek: "generic_file_llseek",
    iterate_shared: "v9fs_dir_readdir",
    open: "v9fs_file_open",
    release: "v9fs_dir_release",
    fsync: None,
};

pub const V9FS_DIR_OPERATIONS_DOTL: DirFileOperations = DirFileOperations {
    read: "generic_read_dir",
    llseek: "generic_file_llseek",
    iterate_shared: "v9fs_dir_readdir_dotl",
    open: "v9fs_file_open",
    release: "v9fs_dir_release",
    fsync: Some("v9fs_file_fsync_dotl"),
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::include::uapi::stat::{S_IFDIR, S_IFREG};

    #[test]
    fn dir_helpers_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/9p/vfs_dir.c"
        ));
        assert!(source.contains("struct p9_rdir"));
        assert!(source.contains("static inline int dt_type"));
        assert!(source.contains("if (perm & P9_DMDIR)"));
        assert!(source.contains("rettype = DT_DIR;"));
        assert!(source.contains("if (perm & P9_DMSYMLINK)"));
        assert!(source.contains("static struct p9_rdir *v9fs_alloc_rdir_buf"));
        assert!(source.contains("kzalloc(sizeof(struct p9_rdir) + buflen, GFP_KERNEL)"));
        assert!(source.contains("static int v9fs_dir_readdir"));
        assert!(source.contains("if (!rdir)"));
        assert!(source.contains("return -ENOMEM;"));
        assert!(source.contains("n = p9_client_read(file->private_data, ctx->pos, &to,"));
        assert!(source.contains("if (err)"));
        assert!(source.contains("if (n == 0)"));
        assert!(source.contains("if (err <= 0)"));
        assert!(source.contains("return -EIO;"));
        assert!(source.contains("rdir->head += err;"));
        assert!(source.contains("ctx->pos += err;"));
        assert!(source.contains("static int v9fs_dir_readdir_dotl"));
        assert!(source.contains("err = p9_client_readdir(fid, rdir->buf, buflen,"));
        assert!(source.contains("if (err <= 0)"));
        assert!(source.contains("if (err < 0)"));
        assert!(source.contains("ctx->pos = curdirent.d_off;"));
        assert!(source.contains("int v9fs_dir_release"));
        assert!(source.contains("if ((S_ISREG(inode->i_mode)) && (filp->f_mode & FMODE_WRITE))"));
        assert!(source.contains("retval = filemap_fdatawrite(inode->i_mapping);"));
        assert!(source.contains("hlist_del(&fid->ilist);"));
        assert!(source.contains("retval = retval < 0 ? retval : put_err;"));
        assert!(source.contains("fscache_unuse_cookie"));
        assert!(source.contains("const struct file_operations v9fs_dir_operations"));
        assert!(source.contains("generic_read_dir"));
        assert!(source.contains("generic_file_llseek"));
        assert!(source.contains(".iterate_shared = v9fs_dir_readdir_dotl"));

        assert_eq!(dt_type(P9_DMDIR), DT_DIR);
        assert_eq!(dt_type(P9_DMSYMLINK), DT_LNK);
        assert_eq!(dt_type(0), DT_REG);
        assert!(rdir_needs_refill(RdirState { head: 4, tail: 4 }));
        assert_eq!(
            v9fs_alloc_rdir_buf_plan(true, 4096, false),
            RdirBufferPlan {
                returned_existing: true,
                allocation_bytes: 0,
                errno: 0
            }
        );
        assert_eq!(
            v9fs_alloc_rdir_buf_plan(false, 4096, false),
            RdirBufferPlan {
                returned_existing: false,
                allocation_bytes: core::mem::size_of::<RdirState>() + 4096,
                errno: -ENOMEM
            }
        );
        assert_eq!(
            legacy_readdir_refill(RdirState { head: 0, tail: 0 }, 7, 0),
            ReaddirRefillResult {
                state: RdirState { head: 0, tail: 7 },
                read_called: true,
                ret: 1
            }
        );
        assert_eq!(
            legacy_readdir_refill(RdirState { head: 0, tail: 0 }, 0, 0).ret,
            0
        );
        assert_eq!(
            legacy_readdir_refill(RdirState { head: 0, tail: 0 }, 0, -5).ret,
            -5
        );
        assert_eq!(
            legacy_readdir_advance(RdirState { head: 0, tail: 10 }, 3, 100),
            (RdirState { head: 3, tail: 10 }, 103)
        );
        assert_eq!(
            legacy_dirent_step(RdirState { head: 0, tail: 10 }, -1, true, 100),
            LegacyDirentStep {
                state: RdirState { head: 0, tail: 10 },
                ctx_pos: 100,
                emitted: false,
                stat_freed: false,
                ret: -EIO
            }
        );
        assert_eq!(
            legacy_dirent_step(RdirState { head: 0, tail: 10 }, 4, false, 100),
            LegacyDirentStep {
                state: RdirState { head: 0, tail: 10 },
                ctx_pos: 100,
                emitted: false,
                stat_freed: true,
                ret: 0
            }
        );
        assert_eq!(
            legacy_dirent_step(RdirState { head: 0, tail: 10 }, 4, true, 100),
            LegacyDirentStep {
                state: RdirState { head: 4, tail: 10 },
                ctx_pos: 104,
                emitted: true,
                stat_freed: true,
                ret: 1
            }
        );
        assert_eq!(
            dotl_readdir_refill(RdirState { head: 0, tail: 0 }, 9),
            ReaddirRefillResult {
                state: RdirState { head: 0, tail: 9 },
                read_called: true,
                ret: 1
            }
        );
        assert_eq!(
            dotl_readdir_refill(RdirState { head: 0, tail: 0 }, -5).ret,
            -5
        );
        assert_eq!(
            dotl_readdir_advance(RdirState { head: 2, tail: 10 }, 5, 900),
            (RdirState { head: 7, tail: 10 }, 900)
        );
        assert_eq!(
            dotl_dirent_step(RdirState { head: 2, tail: 10 }, -1, true, 900, 700),
            DotlDirentStep {
                state: RdirState { head: 2, tail: 10 },
                ctx_pos: 700,
                emitted: false,
                ret: -EIO
            }
        );
        assert_eq!(
            dotl_dirent_step(RdirState { head: 2, tail: 10 }, 5, true, 900, 700),
            DotlDirentStep {
                state: RdirState { head: 7, tail: 10 },
                ctx_pos: 900,
                emitted: true,
                ret: 1
            }
        );
        let plan = dir_release_plan(true, S_IFREG, FMODE_WRITE);
        assert!(plan.writeback_regular_file);
        assert!(plan.fscache_write_close);
        assert!(!dir_release_plan(true, S_IFDIR, FMODE_WRITE).writeback_regular_file);
        let release = dir_release_result(true, S_IFREG, FMODE_WRITE, -5, -9);
        assert_eq!(release.ret, -5);
        assert!(release.filemap_fdatawrite_called);
        assert!(release.hlist_del);
        assert!(release.spin_lock_inode);
        assert!(release.fscache_version_size_supplied);
        assert_eq!(
            dir_release_result(true, S_IFREG, FMODE_WRITE, 0, -9).ret,
            -9
        );
        assert_eq!(dir_release_result(false, S_IFDIR, 0, -5, -9).ret, 0);
        assert_eq!(V9FS_DIR_OPERATIONS.read, "generic_read_dir");
        assert_eq!(V9FS_DIR_OPERATIONS.llseek, "generic_file_llseek");
        assert_eq!(V9FS_DIR_OPERATIONS_DOTL.fsync, Some("v9fs_file_fsync_dotl"));
    }
}

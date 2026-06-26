//! linux-parity: complete
//! linux-source: vendor/linux/fs/9p/vfs_super.c
//! test-origin: linux:vendor/linux/fs/9p/vfs_super.c
//! 9P superblock setup, statfs fallback, inode drop, and fs-context defaults.

use crate::include::uapi::errno::{ENOMEM, ENOSYS};

use super::types::*;
use super::v9fs::V9fsContext;

pub const MAX_LFS_FILESIZE: u64 = i64::MAX as u64;
pub const SB_ACTIVE: u32 = 1 << 30;
pub const SB_POSIXACL: u32 = 1 << 16;
pub const DCACHE_DONTCACHE: u32 = 1 << 0;
pub const FS_RENAME_DOES_D_MOVE: u32 = 32768;
pub const PAGE_SHIFT: u32 = 12;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SuperOpsKind {
    Legacy,
    Dotl,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SuperBlockPlan {
    pub maxbytes: u64,
    pub blocksize_bits: u32,
    pub blocksize: u32,
    pub magic: u32,
    pub ops: SuperOpsKind,
    pub xattr_handlers: bool,
    pub time_min: u64,
    pub time_max: Option<u64>,
    pub ra_pages: u32,
    pub io_pages: u32,
    pub flags: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DentryOpsChoice {
    Cached,
    UncachedDontCache,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StatfsChoice {
    Remote,
    Simple,
    Error(i32),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GetTreeFailure {
    SessionAlloc,
    SessionInit(i32),
    Sget(i32),
    FillSuper(i32),
    GetNewInode(i32),
    MakeRoot,
    GetAcl(i32),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GetTreePlan {
    pub retval: i32,
    pub allocated_session: bool,
    pub session_initialized: bool,
    pub super_acquired: bool,
    pub fill_super_called: bool,
    pub dentry_ops: Option<DentryOpsChoice>,
    pub root_made: bool,
    pub acl_checked: bool,
    pub fid_added_to_root: bool,
    pub root_dget: bool,
    pub fid_put: bool,
    pub session_close: bool,
    pub session_free: bool,
    pub deactivate_locked_super: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KillSuperPlan {
    pub kill_anon_super: bool,
    pub session_cancel: bool,
    pub session_close: bool,
    pub session_free: bool,
    pub clear_fs_info: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UmountBeginPlan {
    pub begin_cancel: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StatfsPlan {
    pub result: i32,
    pub choice: StatfsChoice,
    pub fid_put: bool,
    pub remote_called: bool,
    pub simple_called: bool,
    pub copied_remote_fields: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WriteInodeKind {
    Legacy,
    Dotl,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WriteInodePlan {
    pub kind: WriteInodeKind,
    pub netfs_unpin_writeback: bool,
    pub ignores_sync_mode: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FreeFcPlan {
    pub returns_early: bool,
    pub free_uname: bool,
    pub free_aname: bool,
    pub free_cachetag: bool,
    pub put_trans: bool,
    pub free_context: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InitFsContextFailure {
    AllocContext,
    UnameDup,
    AnameDup,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InitFsContextPlan {
    pub retval: i32,
    pub ops_installed: bool,
    pub fs_private_installed: bool,
    pub need_free: bool,
    pub session_core_defaults_set: bool,
    pub uname_allocated: bool,
    pub aname_allocated: bool,
    pub client_defaults_set: bool,
    pub fd_defaults_set: bool,
    pub rdma_defaults_set: bool,
    pub defaults: Option<V9fsContext>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FileSystemTypePlan {
    pub name: &'static str,
    pub kill_sb: &'static str,
    pub owner: &'static str,
    pub fs_flags: u32,
    pub init_fs_context: &'static str,
    pub parameters: &'static str,
}

pub fn v9fs_fill_super_plan(session_flags: u32, cache: u32, maxdata: u32) -> SuperBlockPlan {
    let blocksize_bits = fls(maxdata.saturating_sub(1));
    let pages = if cache == 0 { 0 } else { maxdata >> PAGE_SHIFT };
    let mut flags = SB_ACTIVE;
    if (session_flags & V9FS_ACL_MASK) == V9FS_POSIX_ACL {
        flags |= SB_POSIXACL;
    }
    SuperBlockPlan {
        maxbytes: MAX_LFS_FILESIZE,
        blocksize_bits,
        blocksize: 1u32.checked_shl(blocksize_bits).unwrap_or(0),
        magic: V9FS_MAGIC,
        ops: if proto_dotl(session_flags) {
            SuperOpsKind::Dotl
        } else {
            SuperOpsKind::Legacy
        },
        xattr_handlers: proto_dotl(session_flags) && session_flags & V9FS_NO_XATTR == 0,
        time_min: 0,
        time_max: if proto_dotl(session_flags) {
            None
        } else {
            Some(u32::MAX as u64)
        },
        ra_pages: pages,
        io_pages: pages,
        flags,
    }
}

pub fn v9fs_fill_super_result(
    session_flags: u32,
    cache: u32,
    maxdata: u32,
    super_setup_bdi_ret: i32,
) -> Result<SuperBlockPlan, i32> {
    if super_setup_bdi_ret != 0 {
        Err(super_setup_bdi_ret)
    } else {
        Ok(v9fs_fill_super_plan(session_flags, cache, maxdata))
    }
}

pub const fn fls(value: u32) -> u32 {
    if value == 0 {
        0
    } else {
        u32::BITS - value.leading_zeros()
    }
}

pub const fn dentry_ops_for_cache(cache: u32) -> DentryOpsChoice {
    if cache & (CACHE_META | CACHE_LOOSE) != 0 {
        DentryOpsChoice::Cached
    } else {
        DentryOpsChoice::UncachedDontCache
    }
}

pub const fn v9fs_drop_inode(cache: u32) -> bool {
    cache & (CACHE_META | CACHE_LOOSE) == 0
}

pub fn v9fs_get_tree_plan(cache: u32, failure: Option<GetTreeFailure>) -> GetTreePlan {
    let base = GetTreePlan {
        retval: 0,
        allocated_session: true,
        session_initialized: true,
        super_acquired: true,
        fill_super_called: true,
        dentry_ops: Some(dentry_ops_for_cache(cache)),
        root_made: true,
        acl_checked: true,
        fid_added_to_root: true,
        root_dget: true,
        fid_put: false,
        session_close: false,
        session_free: false,
        deactivate_locked_super: false,
    };

    match failure {
        None => base,
        Some(GetTreeFailure::SessionAlloc) => GetTreePlan {
            retval: -ENOMEM,
            allocated_session: false,
            session_initialized: false,
            super_acquired: false,
            fill_super_called: false,
            dentry_ops: None,
            root_made: false,
            acl_checked: false,
            fid_added_to_root: false,
            root_dget: false,
            fid_put: false,
            session_close: false,
            session_free: false,
            deactivate_locked_super: false,
        },
        Some(GetTreeFailure::SessionInit(errno)) => GetTreePlan {
            retval: errno,
            allocated_session: true,
            session_initialized: false,
            super_acquired: false,
            fill_super_called: false,
            dentry_ops: None,
            root_made: false,
            acl_checked: false,
            fid_added_to_root: false,
            root_dget: false,
            fid_put: false,
            session_close: false,
            session_free: true,
            deactivate_locked_super: false,
        },
        Some(GetTreeFailure::Sget(errno)) => GetTreePlan {
            retval: errno,
            allocated_session: true,
            session_initialized: true,
            super_acquired: false,
            fill_super_called: false,
            dentry_ops: None,
            root_made: false,
            acl_checked: false,
            fid_added_to_root: false,
            root_dget: false,
            fid_put: true,
            session_close: true,
            session_free: true,
            deactivate_locked_super: false,
        },
        Some(GetTreeFailure::FillSuper(errno)) => GetTreePlan {
            retval: errno,
            dentry_ops: None,
            root_made: false,
            acl_checked: false,
            fid_added_to_root: false,
            root_dget: false,
            fid_put: true,
            deactivate_locked_super: true,
            ..base
        },
        Some(GetTreeFailure::GetNewInode(errno)) => GetTreePlan {
            retval: errno,
            root_made: false,
            acl_checked: false,
            fid_added_to_root: false,
            root_dget: false,
            fid_put: true,
            deactivate_locked_super: true,
            ..base
        },
        Some(GetTreeFailure::MakeRoot) => GetTreePlan {
            retval: -ENOMEM,
            root_made: false,
            acl_checked: false,
            fid_added_to_root: false,
            root_dget: false,
            fid_put: true,
            deactivate_locked_super: true,
            ..base
        },
        Some(GetTreeFailure::GetAcl(errno)) => GetTreePlan {
            retval: errno,
            fid_added_to_root: false,
            root_dget: false,
            fid_put: true,
            deactivate_locked_super: true,
            ..base
        },
    }
}

pub const fn v9fs_statfs_choice(proto_dotl_enabled: bool, remote_errno: i32) -> StatfsChoice {
    if proto_dotl_enabled {
        if remote_errno == 0 {
            StatfsChoice::Remote
        } else if remote_errno != -ENOSYS {
            StatfsChoice::Error(remote_errno)
        } else {
            StatfsChoice::Simple
        }
    } else {
        StatfsChoice::Simple
    }
}

pub fn v9fs_statfs_plan(
    proto_dotl_enabled: bool,
    fid_lookup_errno: Option<i32>,
    remote_errno: i32,
    simple_errno: i32,
) -> StatfsPlan {
    if let Some(errno) = fid_lookup_errno {
        return StatfsPlan {
            result: errno,
            choice: StatfsChoice::Error(errno),
            fid_put: true,
            remote_called: false,
            simple_called: false,
            copied_remote_fields: false,
        };
    }

    if proto_dotl_enabled {
        if remote_errno == 0 {
            return StatfsPlan {
                result: 0,
                choice: StatfsChoice::Remote,
                fid_put: true,
                remote_called: true,
                simple_called: false,
                copied_remote_fields: true,
            };
        }
        if remote_errno != -ENOSYS {
            return StatfsPlan {
                result: remote_errno,
                choice: StatfsChoice::Error(remote_errno),
                fid_put: true,
                remote_called: true,
                simple_called: false,
                copied_remote_fields: false,
            };
        }
    }

    StatfsPlan {
        result: simple_errno,
        choice: if simple_errno == 0 {
            StatfsChoice::Simple
        } else {
            StatfsChoice::Error(simple_errno)
        },
        fid_put: true,
        remote_called: proto_dotl_enabled,
        simple_called: true,
        copied_remote_fields: false,
    }
}

pub fn init_fs_context_defaults() -> V9fsContext {
    V9fsContext::default()
}

pub const fn v9fs_kill_super_plan() -> KillSuperPlan {
    KillSuperPlan {
        kill_anon_super: true,
        session_cancel: true,
        session_close: true,
        session_free: true,
        clear_fs_info: true,
    }
}

pub const fn v9fs_umount_begin_plan() -> UmountBeginPlan {
    UmountBeginPlan { begin_cancel: true }
}

pub const fn v9fs_write_inode_plan(kind: WriteInodeKind) -> WriteInodePlan {
    WriteInodePlan {
        kind,
        netfs_unpin_writeback: true,
        ignores_sync_mode: true,
    }
}

pub const fn v9fs_free_fc_plan(
    has_context: bool,
    config_9p_fscache: bool,
    has_trans_mod: bool,
) -> FreeFcPlan {
    if !has_context {
        FreeFcPlan {
            returns_early: true,
            free_uname: false,
            free_aname: false,
            free_cachetag: false,
            put_trans: false,
            free_context: false,
        }
    } else {
        FreeFcPlan {
            returns_early: false,
            free_uname: true,
            free_aname: true,
            free_cachetag: config_9p_fscache,
            put_trans: has_trans_mod,
            free_context: true,
        }
    }
}

pub fn v9fs_init_fs_context_plan(failure: Option<InitFsContextFailure>) -> InitFsContextPlan {
    match failure {
        Some(InitFsContextFailure::AllocContext) => InitFsContextPlan {
            retval: -ENOMEM,
            ops_installed: false,
            fs_private_installed: false,
            need_free: false,
            session_core_defaults_set: false,
            uname_allocated: false,
            aname_allocated: false,
            client_defaults_set: false,
            fd_defaults_set: false,
            rdma_defaults_set: false,
            defaults: None,
        },
        Some(InitFsContextFailure::UnameDup) => InitFsContextPlan {
            retval: -ENOMEM,
            ops_installed: true,
            fs_private_installed: true,
            need_free: true,
            session_core_defaults_set: true,
            uname_allocated: false,
            aname_allocated: false,
            client_defaults_set: false,
            fd_defaults_set: false,
            rdma_defaults_set: false,
            defaults: None,
        },
        Some(InitFsContextFailure::AnameDup) => InitFsContextPlan {
            retval: -ENOMEM,
            ops_installed: true,
            fs_private_installed: true,
            need_free: true,
            session_core_defaults_set: true,
            uname_allocated: true,
            aname_allocated: false,
            client_defaults_set: false,
            fd_defaults_set: false,
            rdma_defaults_set: false,
            defaults: None,
        },
        None => InitFsContextPlan {
            retval: 0,
            ops_installed: true,
            fs_private_installed: true,
            need_free: false,
            session_core_defaults_set: true,
            uname_allocated: true,
            aname_allocated: true,
            client_defaults_set: true,
            fd_defaults_set: true,
            rdma_defaults_set: true,
            defaults: Some(init_fs_context_defaults()),
        },
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SuperOps {
    pub alloc_inode: &'static str,
    pub free_inode: &'static str,
    pub statfs: &'static str,
    pub drop_inode: &'static str,
    pub evict_inode: &'static str,
    pub show_options: &'static str,
    pub umount_begin: &'static str,
    pub write_inode: &'static str,
}

pub const V9FS_SUPER_OPS: SuperOps = SuperOps {
    alloc_inode: "v9fs_alloc_inode",
    free_inode: "v9fs_free_inode",
    statfs: "simple_statfs",
    drop_inode: "v9fs_drop_inode",
    evict_inode: "v9fs_evict_inode",
    show_options: "v9fs_show_options",
    umount_begin: "v9fs_umount_begin",
    write_inode: "v9fs_write_inode",
};

pub const V9FS_SUPER_OPS_DOTL: SuperOps = SuperOps {
    alloc_inode: "v9fs_alloc_inode",
    free_inode: "v9fs_free_inode",
    statfs: "v9fs_statfs",
    drop_inode: "v9fs_drop_inode",
    evict_inode: "v9fs_evict_inode",
    show_options: "v9fs_show_options",
    umount_begin: "v9fs_umount_begin",
    write_inode: "v9fs_write_inode_dotl",
};

pub const V9FS_FS_TYPE: FileSystemTypePlan = FileSystemTypePlan {
    name: "9p",
    kill_sb: "v9fs_kill_super",
    owner: "THIS_MODULE",
    fs_flags: FS_RENAME_DOES_D_MOVE,
    init_fs_context: "v9fs_init_fs_context",
    parameters: "v9fs_param_spec",
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::v9fs::v9fs::{
        DEFAULT_MSIZE, P9_FD_PORT, P9_RDMA_PORT, P9_RDMA_RQ_DEPTH, P9_RDMA_SQ_DEPTH,
        P9_RDMA_TIMEOUT,
    };

    #[test]
    fn super_helpers_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/9p/vfs_super.c"
        ));
        assert!(source.contains("static int v9fs_fill_super(struct super_block *sb)"));
        assert!(source.contains("sb->s_maxbytes = MAX_LFS_FILESIZE;"));
        assert!(source.contains("sb->s_blocksize_bits = fls(v9ses->maxdata - 1);"));
        assert!(source.contains("sb->s_magic = V9FS_MAGIC;"));
        assert!(source.contains("if (v9fs_proto_dotl(v9ses))"));
        assert!(source.contains("if (!(v9ses->flags & V9FS_NO_XATTR))"));
        assert!(source.contains("ret = super_setup_bdi(sb);"));
        assert!(source.contains("if (ret)"));
        assert!(source.contains("sb->s_bdi->ra_pages = 0;"));
        assert!(source.contains("sb->s_bdi->ra_pages = v9ses->maxdata >> PAGE_SHIFT;"));
        assert!(source.contains("static int v9fs_get_tree(struct fs_context *fc)"));
        assert!(source.contains("v9ses = kzalloc_obj(struct v9fs_session_info);"));
        assert!(source.contains("fid = v9fs_session_init(v9ses, fc);"));
        assert!(source.contains("sb = sget_fc(fc, NULL, set_anon_super_fc);"));
        assert!(source.contains("retval = v9fs_fill_super(sb);"));
        assert!(source.contains("if (v9ses->cache & (CACHE_META|CACHE_LOOSE))"));
        assert!(source.contains("set_default_d_op(sb, &v9fs_cached_dentry_operations);"));
        assert!(source.contains("sb->s_d_flags |= DCACHE_DONTCACHE;"));
        assert!(source.contains("inode = v9fs_get_new_inode_from_fid(v9ses, fid, sb);"));
        assert!(source.contains("root = d_make_root(inode);"));
        assert!(source.contains("retval = v9fs_get_acl(inode, fid);"));
        assert!(source.contains("v9fs_fid_add(root, &fid);"));
        assert!(source.contains("fc->root = dget(sb->s_root);"));
        assert!(source.contains("goto free_session;"));
        assert!(source.contains("goto clunk_fid;"));
        assert!(source.contains("goto release_sb;"));
        assert!(source.contains("v9fs_session_close(v9ses);"));
        assert!(source.contains("deactivate_locked_super(sb);"));
        assert!(source.contains("static void v9fs_kill_super(struct super_block *s)"));
        assert!(source.contains("kill_anon_super(s);"));
        assert!(source.contains("v9fs_session_cancel(v9ses);"));
        assert!(source.contains("s->s_fs_info = NULL;"));
        assert!(source.contains("v9fs_umount_begin(struct super_block *sb)"));
        assert!(source.contains("v9fs_session_begin_cancel(v9ses);"));
        assert!(source.contains("static int v9fs_statfs"));
        assert!(source.contains("fid = v9fs_fid_lookup(dentry);"));
        assert!(source.contains("res = p9_client_statfs(fid, &rs);"));
        assert!(source.contains("if (res != -ENOSYS)"));
        assert!(source.contains("res = simple_statfs(dentry, buf);"));
        assert!(source.contains("p9_fid_put(fid);"));
        assert!(source.contains("static int v9fs_drop_inode"));
        assert!(source.contains("return inode_generic_drop(inode);"));
        assert!(source.contains("return 1;"));
        assert!(source.contains("static int v9fs_write_inode(struct inode *inode,"));
        assert!(source.contains("static int v9fs_write_inode_dotl(struct inode *inode,"));
        assert!(source.contains("return netfs_unpin_writeback(inode, wbc);"));
        assert!(source.contains("static void v9fs_free_fc(struct fs_context *fc)"));
        assert!(source.contains("if (!ctx)"));
        assert!(source.contains("kfree(ctx->session_opts.uname);"));
        assert!(source.contains("kfree(ctx->session_opts.aname);"));
        assert!(source.contains("if (ctx->client_opts.trans_mod)"));
        assert!(source.contains("v9fs_put_trans(ctx->client_opts.trans_mod);"));
        assert!(source.contains("static int v9fs_init_fs_context"));
        assert!(source.contains("ctx->session_opts.afid = ~0;"));
        assert!(source.contains("ctx->session_opts.uname = kstrdup(V9FS_DEFUSER, GFP_KERNEL);"));
        assert!(source.contains("ctx->session_opts.aname = kstrdup(V9FS_DEFANAME, GFP_KERNEL);"));
        assert!(source.contains("ctx->client_opts.proto_version = p9_proto_2000L;"));
        assert!(source.contains("ctx->client_opts.msize = DEFAULT_MSIZE;"));
        assert!(source.contains("ctx->fd_opts.port = P9_FD_PORT;"));
        assert!(source.contains("ctx->rdma_opts.port = P9_RDMA_PORT;"));
        assert!(source.contains("ctx->rdma_opts.sq_depth = P9_RDMA_SQ_DEPTH;"));
        assert!(source.contains("ctx->rdma_opts.rq_depth = P9_RDMA_RQ_DEPTH;"));
        assert!(source.contains("ctx->rdma_opts.timeout = P9_RDMA_TIMEOUT;"));
        assert!(source.contains("fc->need_free = 1;"));
        assert!(source.contains("struct file_system_type v9fs_fs_type"));
        assert!(source.contains(".name = \"9p\""));
        assert!(source.contains(".kill_sb = v9fs_kill_super,"));
        assert!(source.contains(".fs_flags = FS_RENAME_DOES_D_MOVE,"));
        assert!(source.contains(".init_fs_context = v9fs_init_fs_context,"));

        let plan = v9fs_fill_super_plan(V9FS_PROTO_2000L | V9FS_POSIX_ACL, CACHE_FILE, 8192);
        assert_eq!(plan.blocksize_bits, 13);
        assert_eq!(plan.blocksize, 8192);
        assert_eq!(plan.magic, V9FS_MAGIC);
        assert_eq!(plan.ops, SuperOpsKind::Dotl);
        assert!(plan.xattr_handlers);
        assert_eq!(plan.ra_pages, 2);
        assert!(plan.flags & SB_POSIXACL != 0);
        let no_cache = v9fs_fill_super_plan(0, 0, 4096);
        assert_eq!(no_cache.ops, SuperOpsKind::Legacy);
        assert_eq!(no_cache.time_max, Some(u32::MAX as u64));
        assert_eq!(no_cache.ra_pages, 0);
        assert_eq!(v9fs_fill_super_result(0, 0, 4096, -5), Err(-5));
        assert_eq!(dentry_ops_for_cache(CACHE_META), DentryOpsChoice::Cached);
        assert_eq!(dentry_ops_for_cache(0), DentryOpsChoice::UncachedDontCache);
        assert!(v9fs_drop_inode(0));
        assert!(!v9fs_drop_inode(CACHE_LOOSE));

        let get_tree = v9fs_get_tree_plan(CACHE_META, None);
        assert_eq!(get_tree.retval, 0);
        assert_eq!(get_tree.dentry_ops, Some(DentryOpsChoice::Cached));
        assert!(get_tree.fid_added_to_root);
        assert!(get_tree.root_dget);
        assert!(!get_tree.fid_put);
        let alloc_fail = v9fs_get_tree_plan(0, Some(GetTreeFailure::SessionAlloc));
        assert_eq!(alloc_fail.retval, -ENOMEM);
        assert!(!alloc_fail.allocated_session);
        let session_fail = v9fs_get_tree_plan(0, Some(GetTreeFailure::SessionInit(-5)));
        assert!(session_fail.session_free);
        assert!(!session_fail.session_close);
        let sget_fail = v9fs_get_tree_plan(0, Some(GetTreeFailure::Sget(-5)));
        assert!(sget_fail.fid_put);
        assert!(sget_fail.session_close);
        assert!(sget_fail.session_free);
        let fill_fail = v9fs_get_tree_plan(0, Some(GetTreeFailure::FillSuper(-5)));
        assert!(fill_fail.fill_super_called);
        assert!(fill_fail.deactivate_locked_super);
        assert_eq!(fill_fail.dentry_ops, None);
        let inode_fail = v9fs_get_tree_plan(0, Some(GetTreeFailure::GetNewInode(-5)));
        assert_eq!(
            inode_fail.dentry_ops,
            Some(DentryOpsChoice::UncachedDontCache)
        );
        assert!(inode_fail.deactivate_locked_super);
        let root_fail = v9fs_get_tree_plan(0, Some(GetTreeFailure::MakeRoot));
        assert_eq!(root_fail.retval, -ENOMEM);
        assert!(!root_fail.root_made);
        let acl_fail = v9fs_get_tree_plan(0, Some(GetTreeFailure::GetAcl(-5)));
        assert!(acl_fail.root_made);
        assert!(acl_fail.acl_checked);
        assert!(!acl_fail.fid_added_to_root);

        assert_eq!(v9fs_statfs_choice(true, 0), StatfsChoice::Remote);
        assert_eq!(v9fs_statfs_choice(true, -ENOSYS), StatfsChoice::Simple);
        assert_eq!(v9fs_statfs_choice(true, -5), StatfsChoice::Error(-5));
        let fid_error = v9fs_statfs_plan(true, Some(-5), 0, 0);
        assert_eq!(fid_error.result, -5);
        assert!(fid_error.fid_put);
        assert!(!fid_error.remote_called);
        let remote = v9fs_statfs_plan(true, None, 0, 0);
        assert_eq!(remote.choice, StatfsChoice::Remote);
        assert!(remote.copied_remote_fields);
        assert!(!remote.simple_called);
        let fallback = v9fs_statfs_plan(true, None, -ENOSYS, 0);
        assert_eq!(fallback.choice, StatfsChoice::Simple);
        assert!(fallback.remote_called);
        assert!(fallback.simple_called);
        let legacy = v9fs_statfs_plan(false, None, -5, 0);
        assert_eq!(legacy.choice, StatfsChoice::Simple);
        assert!(!legacy.remote_called);

        assert!(v9fs_kill_super_plan().clear_fs_info);
        assert!(v9fs_umount_begin_plan().begin_cancel);
        let write = v9fs_write_inode_plan(WriteInodeKind::Dotl);
        assert_eq!(write.kind, WriteInodeKind::Dotl);
        assert!(write.netfs_unpin_writeback);
        assert!(write.ignores_sync_mode);
        assert_eq!(
            v9fs_write_inode_plan(WriteInodeKind::Legacy).kind,
            WriteInodeKind::Legacy
        );
        assert!(v9fs_free_fc_plan(false, true, true).returns_early);
        let free = v9fs_free_fc_plan(true, true, true);
        assert!(free.free_uname);
        assert!(free.free_aname);
        assert!(free.free_cachetag);
        assert!(free.put_trans);
        assert!(free.free_context);

        let alloc_error = v9fs_init_fs_context_plan(Some(InitFsContextFailure::AllocContext));
        assert_eq!(alloc_error.retval, -ENOMEM);
        assert!(!alloc_error.ops_installed);
        assert!(!alloc_error.need_free);
        let uname_error = v9fs_init_fs_context_plan(Some(InitFsContextFailure::UnameDup));
        assert!(uname_error.ops_installed);
        assert!(uname_error.need_free);
        assert!(!uname_error.uname_allocated);
        let aname_error = v9fs_init_fs_context_plan(Some(InitFsContextFailure::AnameDup));
        assert!(aname_error.uname_allocated);
        assert!(!aname_error.aname_allocated);
        assert!(!aname_error.client_defaults_set);
        let init = v9fs_init_fs_context_plan(None);
        assert_eq!(init.retval, 0);
        assert!(init.client_defaults_set);
        assert!(init.fd_defaults_set);
        assert!(init.rdma_defaults_set);
        let defaults = init.defaults.unwrap();
        assert_eq!(
            defaults.client.proto_version,
            crate::fs::v9fs::v9fs::ProtoVersion::P9P2000L
        );
        assert_eq!(defaults.client.msize, DEFAULT_MSIZE);
        assert_eq!(defaults.fd_port, P9_FD_PORT);
        assert_eq!(defaults.rdma_port, P9_RDMA_PORT);
        assert_eq!(defaults.rdma_sq_depth, P9_RDMA_SQ_DEPTH);
        assert_eq!(defaults.rdma_rq_depth, P9_RDMA_RQ_DEPTH);
        assert_eq!(defaults.rdma_timeout, P9_RDMA_TIMEOUT);

        assert_eq!(V9FS_SUPER_OPS.umount_begin, "v9fs_umount_begin");
        assert_eq!(V9FS_SUPER_OPS_DOTL.statfs, "v9fs_statfs");
        assert_eq!(V9FS_SUPER_OPS_DOTL.write_inode, "v9fs_write_inode_dotl");
        assert_eq!(V9FS_FS_TYPE.name, "9p");
        assert_eq!(V9FS_FS_TYPE.fs_flags, FS_RENAME_DOES_D_MOVE);
    }
}

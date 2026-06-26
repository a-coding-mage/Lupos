//! linux-parity: complete
//! linux-source: vendor/linux/fs/9p/vfs_addr.c
//! test-origin: linux:vendor/linux/fs/9p/vfs_addr.c
//! Netfs read/write request sizing and subrequest completion decisions.

use crate::include::uapi::errno::EINVAL;

use super::types::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NetfsOrigin {
    Writeback,
    ReadForWrite,
    Writethrough,
    UnbufferedWrite,
    DioWrite,
    UnbufferedRead,
    DioRead,
    BufferedRead,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RequestInitPlan {
    pub needs_fid_now: bool,
    pub writing: bool,
    pub wsize: Option<u32>,
    pub errno: i32,
    pub fid_get: bool,
    pub used_file_private_data: bool,
    pub used_inode_lookup: bool,
    pub warn_read_for_write_without_ordwr: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReadSubreqResult {
    pub transferred: u32,
    pub error: i32,
    pub made_progress: bool,
    pub clear_tail: bool,
    pub hit_eof: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WriteSubreqResult {
    pub terminated_with: i32,
    pub made_progress: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BeginWritebackPlan {
    pub found_fid: bool,
    pub warn_missing_fid: bool,
    pub wsize: Option<u32>,
    pub netfs_priv_set: bool,
    pub stream0_available: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FreeRequestPlan {
    pub fid_put: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NetfsRequestOps {
    pub init_request: &'static str,
    pub free_request: &'static str,
    pub issue_read: &'static str,
    pub begin_writeback: &'static str,
    pub issue_write: &'static str,
}

pub const V9FS_REQ_OPS: NetfsRequestOps = NetfsRequestOps {
    init_request: "v9fs_init_request",
    free_request: "v9fs_free_request",
    issue_read: "v9fs_issue_read",
    begin_writeback: "v9fs_begin_writeback",
    issue_write: "v9fs_issue_write",
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AddressSpaceOperations {
    pub read_folio: &'static str,
    pub readahead: &'static str,
    pub dirty_folio: &'static str,
    pub release_folio: &'static str,
    pub invalidate_folio: &'static str,
    pub direct_io: &'static str,
    pub writepages: &'static str,
    pub migrate_folio: &'static str,
}

pub const V9FS_ADDR_OPERATIONS: AddressSpaceOperations = AddressSpaceOperations {
    read_folio: "netfs_read_folio",
    readahead: "netfs_readahead",
    dirty_folio: "netfs_dirty_folio",
    release_folio: "netfs_release_folio",
    invalidate_folio: "netfs_invalidate_folio",
    direct_io: "noop_direct_IO",
    writepages: "netfs_writepages",
    migrate_folio: "filemap_migrate_folio",
};

pub const fn v9fs_io_size(msize: u32, iounit: u32) -> u32 {
    let base = msize.saturating_sub(P9_IOHDRSZ);
    if iounit != 0 && iounit < base {
        iounit
    } else {
        base
    }
}

pub const fn request_origin_is_writing(origin: NetfsOrigin) -> bool {
    matches!(
        origin,
        NetfsOrigin::ReadForWrite
            | NetfsOrigin::Writethrough
            | NetfsOrigin::UnbufferedWrite
            | NetfsOrigin::DioWrite
    )
}

pub fn v9fs_begin_writeback_plan(
    inode_has_write_fid: bool,
    msize: u32,
    iounit: u32,
) -> BeginWritebackPlan {
    if !inode_has_write_fid {
        return BeginWritebackPlan {
            found_fid: false,
            warn_missing_fid: true,
            wsize: None,
            netfs_priv_set: false,
            stream0_available: false,
        };
    }

    BeginWritebackPlan {
        found_fid: true,
        warn_missing_fid: false,
        wsize: Some(v9fs_io_size(msize, iounit)),
        netfs_priv_set: true,
        stream0_available: true,
    }
}

pub fn v9fs_init_request_plan(
    origin: NetfsOrigin,
    file_has_fid: bool,
    inode_has_fid: bool,
    msize: u32,
    iounit: u32,
) -> RequestInitPlan {
    v9fs_init_request_plan_with_file(
        origin,
        file_has_fid,
        file_has_fid,
        inode_has_fid,
        false,
        msize,
        iounit,
    )
}

pub fn v9fs_init_request_plan_with_file(
    origin: NetfsOrigin,
    file_present: bool,
    file_has_fid: bool,
    inode_has_fid: bool,
    fid_mode_ordwr: bool,
    msize: u32,
    iounit: u32,
) -> RequestInitPlan {
    let writing = request_origin_is_writing(origin);
    if origin == NetfsOrigin::Writeback {
        return RequestInitPlan {
            needs_fid_now: false,
            writing: false,
            wsize: None,
            errno: 0,
            fid_get: false,
            used_file_private_data: false,
            used_inode_lookup: false,
            warn_read_for_write_without_ordwr: false,
        };
    }

    let (has_fid, used_file_private_data, used_inode_lookup) = if file_present {
        (file_has_fid, file_has_fid, false)
    } else {
        (inode_has_fid, false, inode_has_fid)
    };

    if !has_fid {
        return RequestInitPlan {
            needs_fid_now: true,
            writing,
            wsize: None,
            errno: -EINVAL,
            fid_get: false,
            used_file_private_data,
            used_inode_lookup,
            warn_read_for_write_without_ordwr: false,
        };
    }

    RequestInitPlan {
        needs_fid_now: true,
        writing,
        wsize: Some(v9fs_io_size(msize, iounit)),
        errno: 0,
        fid_get: true,
        used_file_private_data,
        used_inode_lookup,
        warn_read_for_write_without_ordwr: origin == NetfsOrigin::ReadForWrite && !fid_mode_ordwr,
    }
}

pub fn v9fs_issue_read_result(
    origin: NetfsOrigin,
    start: u64,
    prior_transferred: u32,
    read_total: u32,
    read_errno: i32,
    inode_size: u64,
) -> ReadSubreqResult {
    let pos = start + prior_transferred as u64;
    let made_progress = read_errno == 0 && read_total != 0;
    ReadSubreqResult {
        transferred: if made_progress {
            prior_transferred + read_total
        } else {
            prior_transferred
        },
        error: read_errno,
        made_progress,
        clear_tail: origin != NetfsOrigin::UnbufferedRead && origin != NetfsOrigin::DioRead,
        hit_eof: pos + read_total as u64 >= inode_size,
    }
}

pub const fn v9fs_issue_write_result(write_len: i32, write_errno: i32) -> WriteSubreqResult {
    WriteSubreqResult {
        terminated_with: if write_len != 0 {
            write_len
        } else {
            write_errno
        },
        made_progress: write_len > 0,
    }
}

pub const fn v9fs_free_request_plan(has_netfs_priv: bool) -> FreeRequestPlan {
    FreeRequestPlan {
        fid_put: has_netfs_priv,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vfs_addr_helpers_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/9p/vfs_addr.c"
        ));
        assert!(source.contains("static void v9fs_begin_writeback"));
        assert!(source.contains("wreq->wsize = fid->clnt->msize - P9_IOHDRSZ;"));
        assert!(source.contains("if (fid->iounit)"));
        assert!(source.contains("wreq->wsize = min(wreq->wsize, fid->iounit);"));
        assert!(source.contains("static void v9fs_issue_write"));
        assert!(source.contains("__set_bit(NETFS_SREQ_MADE_PROGRESS"));
        assert!(source.contains("static void v9fs_issue_read"));
        assert!(source.contains("NETFS_SREQ_CLEAR_TAIL"));
        assert!(source.contains("NETFS_SREQ_HIT_EOF"));
        assert!(source.contains("static int v9fs_init_request"));
        assert!(source.contains("if (rreq->origin == NETFS_WRITEBACK)"));
        assert!(source.contains("return -EINVAL;"));
        assert!(source.contains("static void v9fs_free_request"));
        assert!(source.contains("p9_fid_put(fid);"));
        assert!(source.contains("const struct netfs_request_ops v9fs_req_ops"));
        assert!(source.contains("const struct address_space_operations v9fs_addr_operations"));
        assert!(source.contains(".read_folio\t\t= netfs_read_folio"));
        assert!(source.contains(".direct_IO\t\t= noop_direct_IO"));

        assert_eq!(v9fs_io_size(8192, 0), 8192 - P9_IOHDRSZ);
        assert_eq!(v9fs_io_size(8192, 1024), 1024);
        assert!(request_origin_is_writing(NetfsOrigin::DioWrite));
        assert!(!request_origin_is_writing(NetfsOrigin::BufferedRead));
        assert_eq!(V9FS_REQ_OPS.init_request, "v9fs_init_request");
        assert_eq!(V9FS_REQ_OPS.begin_writeback, "v9fs_begin_writeback");
        assert_eq!(V9FS_ADDR_OPERATIONS.writepages, "netfs_writepages");
        assert_eq!(V9FS_ADDR_OPERATIONS.migrate_folio, "filemap_migrate_folio");
        assert_eq!(
            v9fs_begin_writeback_plan(false, 8192, 0),
            BeginWritebackPlan {
                found_fid: false,
                warn_missing_fid: true,
                wsize: None,
                netfs_priv_set: false,
                stream0_available: false,
            }
        );
        assert_eq!(
            v9fs_begin_writeback_plan(true, 8192, 1024),
            BeginWritebackPlan {
                found_fid: true,
                warn_missing_fid: false,
                wsize: Some(1024),
                netfs_priv_set: true,
                stream0_available: true,
            }
        );
        assert_eq!(
            v9fs_init_request_plan(NetfsOrigin::Writeback, false, false, 8192, 0),
            RequestInitPlan {
                needs_fid_now: false,
                writing: false,
                wsize: None,
                errno: 0,
                fid_get: false,
                used_file_private_data: false,
                used_inode_lookup: false,
                warn_read_for_write_without_ordwr: false,
            }
        );
        assert_eq!(
            v9fs_init_request_plan(NetfsOrigin::BufferedRead, false, false, 8192, 0).errno,
            -EINVAL
        );
        assert_eq!(
            v9fs_init_request_plan_with_file(
                NetfsOrigin::BufferedRead,
                true,
                false,
                true,
                false,
                8192,
                0
            )
            .errno,
            -EINVAL
        );
        assert_eq!(
            v9fs_init_request_plan_with_file(
                NetfsOrigin::ReadForWrite,
                false,
                false,
                true,
                false,
                8192,
                0
            ),
            RequestInitPlan {
                needs_fid_now: true,
                writing: true,
                wsize: Some(8192 - P9_IOHDRSZ),
                errno: 0,
                fid_get: true,
                used_file_private_data: false,
                used_inode_lookup: true,
                warn_read_for_write_without_ordwr: true,
            }
        );
        assert_eq!(
            v9fs_issue_read_result(NetfsOrigin::BufferedRead, 10, 5, 7, 0, 21),
            ReadSubreqResult {
                transferred: 12,
                error: 0,
                made_progress: true,
                clear_tail: true,
                hit_eof: true
            }
        );
        assert!(!v9fs_issue_read_result(NetfsOrigin::DioRead, 0, 0, 1, 0, 9).clear_tail);
        assert_eq!(
            v9fs_issue_write_result(0, -5),
            WriteSubreqResult {
                terminated_with: -5,
                made_progress: false
            }
        );
        assert_eq!(
            v9fs_free_request_plan(true),
            FreeRequestPlan { fid_put: true }
        );
        assert_eq!(
            v9fs_free_request_plan(false),
            FreeRequestPlan { fid_put: false }
        );
    }
}

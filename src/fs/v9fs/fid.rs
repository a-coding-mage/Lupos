//! linux-parity: complete
//! linux-source: vendor/linux/fs/9p/fid.c
//! test-origin: linux:vendor/linux/fs/9p/fid.c
//! 9P fid selection, path-walk batching, and cache-mode augmentation.

extern crate alloc;

use alloc::vec::Vec;

use crate::include::uapi::errno::{ENOENT, ENOMEM, EPERM};
use crate::include::uapi::fcntl::{O_DIRECT, O_DSYNC};

use super::types::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FidSnapshot {
    pub uid: u32,
    pub mode: u32,
    pub qid_version: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FidLookupIdentity {
    pub uid: u32,
    pub any: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FidLocation {
    Dentry(usize),
    Inode(usize),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FidAddTarget {
    Dentry,
    Inode,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FidAddPlan {
    pub target: FidAddTarget,
    pub fid_added: bool,
    pub pfid_nulled: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FidLookupInputs {
    pub is_root: bool,
    pub dentry_unhashed_after_walk: bool,
    pub parent_walk_errno: i32,
    pub attach_errno: i32,
    pub build_path_errno: i32,
    pub walk_error_at_batch: Option<(usize, i32)>,
}

impl FidLookupInputs {
    pub const fn success() -> Self {
        Self {
            is_root: false,
            dentry_unhashed_after_walk: false,
            parent_walk_errno: 0,
            attach_errno: 0,
            build_path_errno: 0,
            walk_error_at_batch: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FidLookupStep {
    UseExisting(FidLocation),
    LockRename,
    UnlockRename,
    WalkFromParent {
        location: FidLocation,
        clone: bool,
    },
    AttachRoot {
        uname_is_null: bool,
    },
    AddRootFid,
    ReturnRootFid,
    WalkBatch {
        offset: usize,
        len: usize,
        clone: bool,
    },
    AddDentryFid,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FidLookupPlan {
    pub uid: u32,
    pub any: bool,
    pub steps: Vec<FidLookupStep>,
}

pub const INVALID_UID: u32 = u32::MAX;

pub const fn v9fs_is_writeable(mode: u32) -> bool {
    mode & (P9_OWRITE | P9_ORDWR) != 0
}

pub fn v9fs_fid_find_inode(
    fids: &[FidSnapshot],
    want_writeable: bool,
    uid: u32,
    any: bool,
) -> Option<usize> {
    fids.iter()
        .position(|fid| (any || fid.uid == uid) && (!want_writeable || v9fs_is_writeable(fid.mode)))
}

pub fn v9fs_fid_add_plan(fid_present: bool) -> FidAddPlan {
    FidAddPlan {
        target: FidAddTarget::Dentry,
        fid_added: fid_present,
        pfid_nulled: fid_present,
    }
}

pub fn v9fs_open_fid_add_plan(fid_present: bool) -> FidAddPlan {
    FidAddPlan {
        target: FidAddTarget::Inode,
        fid_added: fid_present,
        pfid_nulled: fid_present,
    }
}

pub fn v9fs_fid_find(
    dentry_fids: &[FidSnapshot],
    inode_fids: &[FidSnapshot],
    uid: u32,
    any: bool,
) -> Option<FidLocation> {
    dentry_fids
        .iter()
        .position(|fid| any || fid.uid == uid)
        .map(FidLocation::Dentry)
        .or_else(|| v9fs_fid_find_inode(inode_fids, false, uid, any).map(FidLocation::Inode))
}

pub fn build_path_from_dentry<'a>(leaf_to_root_names: &[&'a str]) -> Vec<&'a str> {
    leaf_to_root_names.iter().rev().copied().collect()
}

pub fn walk_batch_lengths(path_len: usize) -> Vec<usize> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < path_len {
        let len = core::cmp::min(path_len - i, P9_MAXWELEM);
        out.push(len);
        i += len;
    }
    out
}

pub fn v9fs_lookup_identity(
    session_flags: u32,
    session_uid: u32,
    current_fsuid: u32,
) -> FidLookupIdentity {
    match session_flags & V9FS_ACCESS_MASK {
        V9FS_ACCESS_SINGLE | V9FS_ACCESS_USER | V9FS_ACCESS_CLIENT => FidLookupIdentity {
            uid: current_fsuid,
            any: false,
        },
        V9FS_ACCESS_ANY => FidLookupIdentity {
            uid: session_uid,
            any: true,
        },
        _ => FidLookupIdentity {
            uid: INVALID_UID,
            any: false,
        },
    }
}

pub fn v9fs_fid_lookup_with_uid_plan(
    dentry_fids: &[FidSnapshot],
    inode_fids: &[FidSnapshot],
    parent_dentry_fids: &[FidSnapshot],
    parent_inode_fids: &[FidSnapshot],
    root_dentry_fids: &[FidSnapshot],
    root_inode_fids: &[FidSnapshot],
    leaf_to_root_names: &[&str],
    session_flags: u32,
    uid: u32,
    any: bool,
    inputs: FidLookupInputs,
) -> Result<FidLookupPlan, i32> {
    let mut steps = Vec::new();

    if let Some(location) = v9fs_fid_find(dentry_fids, inode_fids, uid, any) {
        steps.push(FidLookupStep::UseExisting(location));
        return Ok(FidLookupPlan { uid, any, steps });
    }

    steps.push(FidLookupStep::LockRename);
    if let Some(location) = v9fs_fid_find(parent_dentry_fids, parent_inode_fids, uid, any) {
        if inputs.parent_walk_errno != 0 {
            steps.push(FidLookupStep::UnlockRename);
            return Err(inputs.parent_walk_errno);
        }
        steps.push(FidLookupStep::WalkFromParent {
            location,
            clone: true,
        });
        return finish_walked_fid(uid, any, steps, inputs.dentry_unhashed_after_walk);
    }
    steps.push(FidLookupStep::UnlockRename);

    let root_fid = v9fs_fid_find(root_dentry_fids, root_inode_fids, uid, any);
    if root_fid.is_none() {
        if session_flags & V9FS_ACCESS_MASK == V9FS_ACCESS_SINGLE {
            return Err(-EPERM);
        }
        if inputs.attach_errno != 0 {
            return Err(inputs.attach_errno);
        }
        steps.push(FidLookupStep::AttachRoot {
            uname_is_null: proto_dotu(session_flags) || proto_dotl(session_flags),
        });
        steps.push(FidLookupStep::AddRootFid);
    }

    if inputs.is_root {
        steps.push(FidLookupStep::ReturnRootFid);
        return Ok(FidLookupPlan { uid, any, steps });
    }

    steps.push(FidLookupStep::LockRename);
    if inputs.build_path_errno != 0 {
        steps.push(FidLookupStep::UnlockRename);
        return Err(inputs.build_path_errno);
    }
    let path = build_path_from_dentry(leaf_to_root_names);
    let batches = walk_batch_lengths(path.len());
    for (batch_index, len) in batches.iter().copied().enumerate() {
        if let Some((error_batch, errno)) = inputs.walk_error_at_batch
            && error_batch == batch_index
        {
            steps.push(FidLookupStep::UnlockRename);
            return Err(errno);
        }
        let offset = batches[..batch_index].iter().sum();
        steps.push(FidLookupStep::WalkBatch {
            offset,
            len,
            clone: batch_index == 0,
        });
    }

    finish_walked_fid(uid, any, steps, inputs.dentry_unhashed_after_walk)
}

pub fn v9fs_fid_lookup_plan(
    dentry_fids: &[FidSnapshot],
    inode_fids: &[FidSnapshot],
    parent_dentry_fids: &[FidSnapshot],
    parent_inode_fids: &[FidSnapshot],
    root_dentry_fids: &[FidSnapshot],
    root_inode_fids: &[FidSnapshot],
    leaf_to_root_names: &[&str],
    session_flags: u32,
    session_uid: u32,
    current_fsuid: u32,
    inputs: FidLookupInputs,
) -> Result<FidLookupPlan, i32> {
    let identity = v9fs_lookup_identity(session_flags, session_uid, current_fsuid);
    v9fs_fid_lookup_with_uid_plan(
        dentry_fids,
        inode_fids,
        parent_dentry_fids,
        parent_inode_fids,
        root_dentry_fids,
        root_inode_fids,
        leaf_to_root_names,
        session_flags,
        identity.uid,
        identity.any,
        inputs,
    )
}

fn finish_walked_fid(
    uid: u32,
    any: bool,
    mut steps: Vec<FidLookupStep>,
    dentry_unhashed: bool,
) -> Result<FidLookupPlan, i32> {
    if dentry_unhashed {
        steps.push(FidLookupStep::UnlockRename);
        return Err(-ENOENT);
    }
    steps.push(FidLookupStep::AddDentryFid);
    steps.push(FidLookupStep::UnlockRename);
    Ok(FidLookupPlan { uid, any, steps })
}

pub fn v9fs_fid_add_modes(
    fid_mode: u32,
    qid_version: u32,
    session_flags: u32,
    session_cache: u32,
    file_flags: u32,
) -> u32 {
    let mut mode = fid_mode;
    if session_cache == 0
        || (qid_version == 0 && session_flags & V9FS_IGNORE_QV == 0)
        || session_flags & V9FS_DIRECT_IO != 0
        || file_flags & O_DIRECT != 0
    {
        mode |= P9L_DIRECT;
    } else if session_cache & CACHE_WRITEBACK == 0
        || file_flags & O_DSYNC != 0
        || session_flags & V9FS_SYNC != 0
    {
        mode |= P9L_NOWRITECACHE;
    }
    mode
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::include::uapi::fcntl::O_WRONLY;

    const fn fid(uid: u32, mode: u32) -> FidSnapshot {
        FidSnapshot {
            uid,
            mode,
            qid_version: 1,
        }
    }

    #[test]
    fn fid_helpers_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/9p/fid.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/9p/fid.h"
        ));
        assert!(source.contains("static inline void __add_fid"));
        assert!(source.contains("void v9fs_fid_add"));
        assert!(source.contains("void v9fs_open_fid_add"));
        assert!(source.contains("static bool v9fs_is_writeable"));
        assert!(source.contains("if (mode & (P9_OWRITE|P9_ORDWR))"));
        assert!(source.contains("struct p9_fid *v9fs_fid_find_inode"));
        assert!(source.contains("if (want_writeable && !v9fs_is_writeable(fid->mode))"));
        assert!(source.contains("static struct p9_fid *v9fs_fid_find"));
        assert!(source.contains("if (dentry->d_fsdata)"));
        assert!(source.contains("if (!ret && dentry->d_inode)"));
        assert!(source.contains("static int build_path_from_dentry"));
        assert!(source.contains("for (ds = dentry, i = (n-1); i >= 0; i--"));
        assert!(source.contains("static struct p9_fid *v9fs_fid_lookup_with_uid"));
        assert!(source.contains("fid = v9fs_fid_find(dentry, uid, any);"));
        assert!(source.contains("fid = v9fs_fid_find(ds, uid, any);"));
        assert!(source.contains("fid = p9_client_walk(old_fid, 1, &dentry->d_name.name, 1);"));
        assert!(source.contains("if (access == V9FS_ACCESS_SINGLE)"));
        assert!(source.contains("fid = p9_client_attach(v9ses->clnt, NULL, uname, uid,"));
        assert!(source.contains("if (dentry->d_sb->s_root == dentry)"));
        assert!(source.contains("l = min(n - i, P9_MAXWELEM);"));
        assert!(source.contains("old_fid == root_fid /* clone */"));
        assert!(source.contains("if (d_unhashed(dentry))"));
        assert!(source.contains("struct p9_fid *v9fs_fid_lookup(struct dentry *dentry)"));
        assert!(source.contains("case V9FS_ACCESS_ANY:"));
        assert!(header.contains("static inline struct p9_fid *v9fs_parent_fid"));
        assert!(header.contains("static inline struct p9_fid *clone_fid"));
        assert!(header.contains("static inline void v9fs_fid_add_modes"));
        assert!(header.contains("fid->mode |= P9L_DIRECT"));
        assert!(header.contains("fid->mode |= P9L_NOWRITECACHE"));

        assert!(v9fs_is_writeable(P9_OWRITE));
        assert!(v9fs_is_writeable(P9_ORDWR));
        assert!(!v9fs_is_writeable(P9_OREAD));

        let fids = [
            FidSnapshot {
                uid: 10,
                mode: P9_OREAD,
                qid_version: 1,
            },
            FidSnapshot {
                uid: 11,
                mode: P9_OWRITE,
                qid_version: 1,
            },
        ];
        assert_eq!(v9fs_fid_find_inode(&fids, false, 10, false), Some(0));
        assert_eq!(v9fs_fid_find_inode(&fids, true, 10, false), None);
        assert_eq!(v9fs_fid_find_inode(&fids, true, 0, true), Some(1));
        assert_eq!(
            v9fs_fid_add_plan(true),
            FidAddPlan {
                target: FidAddTarget::Dentry,
                fid_added: true,
                pfid_nulled: true,
            }
        );
        assert_eq!(
            v9fs_open_fid_add_plan(false),
            FidAddPlan {
                target: FidAddTarget::Inode,
                fid_added: false,
                pfid_nulled: false,
            }
        );
        assert_eq!(
            v9fs_fid_find(&fids[..1], &fids[1..], 11, false),
            Some(FidLocation::Inode(0))
        );
        assert_eq!(
            v9fs_fid_find(&fids[..1], &fids[1..], 0, true),
            Some(FidLocation::Dentry(0))
        );
        assert_eq!(
            build_path_from_dentry(&["leaf", "mid", "root"]),
            ["root", "mid", "leaf"]
        );
        assert_eq!(walk_batch_lengths(34), [16, 16, 2]);
        assert_eq!(
            v9fs_lookup_identity(V9FS_ACCESS_ANY, 55, 66),
            FidLookupIdentity { uid: 55, any: true }
        );
        assert_eq!(
            v9fs_lookup_identity(V9FS_ACCESS_USER, 55, 66),
            FidLookupIdentity {
                uid: 66,
                any: false
            }
        );
        assert_eq!(v9fs_fid_add_modes(0, 0, 0, CACHE_FILE, 0), P9L_DIRECT);
        assert_eq!(
            v9fs_fid_add_modes(0, 0, V9FS_IGNORE_QV, CACHE_FILE, 0),
            P9L_NOWRITECACHE
        );
        assert_eq!(v9fs_fid_add_modes(0, 1, 0, CACHE_WRITEBACK, O_WRONLY), 0);
    }

    #[test]
    fn fid_lookup_plan_matches_existing_parent_root_and_walk_paths() {
        let existing = v9fs_fid_lookup_plan(
            &[fid(44, P9_OREAD)],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            V9FS_ACCESS_USER,
            7,
            44,
            FidLookupInputs::success(),
        )
        .unwrap();
        assert_eq!(existing.uid, 44);
        assert_eq!(existing.any, false);
        assert_eq!(
            existing.steps,
            [FidLookupStep::UseExisting(FidLocation::Dentry(0))]
        );

        let from_parent = v9fs_fid_lookup_plan(
            &[],
            &[],
            &[],
            &[fid(44, P9_OREAD)],
            &[],
            &[],
            &["leaf"],
            V9FS_ACCESS_USER,
            7,
            44,
            FidLookupInputs::success(),
        )
        .unwrap();
        assert_eq!(
            from_parent.steps,
            [
                FidLookupStep::LockRename,
                FidLookupStep::WalkFromParent {
                    location: FidLocation::Inode(0),
                    clone: true,
                },
                FidLookupStep::AddDentryFid,
                FidLookupStep::UnlockRename,
            ]
        );

        let root = v9fs_fid_lookup_plan(
            &[],
            &[],
            &[],
            &[],
            &[fid(44, P9_OREAD)],
            &[],
            &[],
            V9FS_ACCESS_USER,
            7,
            44,
            FidLookupInputs {
                is_root: true,
                ..FidLookupInputs::success()
            },
        )
        .unwrap();
        assert_eq!(
            root.steps,
            [
                FidLookupStep::LockRename,
                FidLookupStep::UnlockRename,
                FidLookupStep::ReturnRootFid,
            ]
        );

        let names = [
            "leaf", "15", "14", "13", "12", "11", "10", "9", "8", "7", "6", "5", "4", "3", "2",
            "1", "0",
        ];
        let walked = v9fs_fid_lookup_plan(
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &names,
            V9FS_PROTO_2000L | V9FS_ACCESS_ANY,
            7,
            44,
            FidLookupInputs::success(),
        )
        .unwrap();
        assert_eq!(walked.uid, 7);
        assert_eq!(walked.any, true);
        assert_eq!(
            walked.steps,
            [
                FidLookupStep::LockRename,
                FidLookupStep::UnlockRename,
                FidLookupStep::AttachRoot {
                    uname_is_null: true,
                },
                FidLookupStep::AddRootFid,
                FidLookupStep::LockRename,
                FidLookupStep::WalkBatch {
                    offset: 0,
                    len: P9_MAXWELEM,
                    clone: true,
                },
                FidLookupStep::WalkBatch {
                    offset: P9_MAXWELEM,
                    len: 1,
                    clone: false,
                },
                FidLookupStep::AddDentryFid,
                FidLookupStep::UnlockRename,
            ]
        );
    }

    #[test]
    fn fid_lookup_plan_preserves_linux_error_edges() {
        let single_access = v9fs_fid_lookup_plan(
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &["leaf"],
            V9FS_ACCESS_SINGLE,
            7,
            44,
            FidLookupInputs::success(),
        );
        assert_eq!(single_access, Err(-EPERM));

        let attach_failure = v9fs_fid_lookup_with_uid_plan(
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &["leaf"],
            V9FS_ACCESS_USER,
            44,
            false,
            FidLookupInputs {
                attach_errno: -ENOMEM,
                ..FidLookupInputs::success()
            },
        );
        assert_eq!(attach_failure, Err(-ENOMEM));

        let parent_walk_failure = v9fs_fid_lookup_with_uid_plan(
            &[],
            &[],
            &[fid(44, P9_OREAD)],
            &[],
            &[],
            &[],
            &["leaf"],
            V9FS_ACCESS_USER,
            44,
            false,
            FidLookupInputs {
                parent_walk_errno: -ENOENT,
                ..FidLookupInputs::success()
            },
        );
        assert_eq!(parent_walk_failure, Err(-ENOENT));

        let build_path_failure = v9fs_fid_lookup_with_uid_plan(
            &[],
            &[],
            &[],
            &[],
            &[fid(44, P9_OREAD)],
            &[],
            &["leaf"],
            V9FS_ACCESS_USER,
            44,
            false,
            FidLookupInputs {
                build_path_errno: -ENOMEM,
                ..FidLookupInputs::success()
            },
        );
        assert_eq!(build_path_failure, Err(-ENOMEM));

        let walk_failure = v9fs_fid_lookup_with_uid_plan(
            &[],
            &[],
            &[],
            &[],
            &[fid(44, P9_OREAD)],
            &[],
            &["leaf"],
            V9FS_ACCESS_USER,
            44,
            false,
            FidLookupInputs {
                walk_error_at_batch: Some((0, -ENOENT)),
                ..FidLookupInputs::success()
            },
        );
        assert_eq!(walk_failure, Err(-ENOENT));

        let unhashed = v9fs_fid_lookup_with_uid_plan(
            &[],
            &[],
            &[],
            &[],
            &[fid(44, P9_OREAD)],
            &[],
            &["leaf"],
            V9FS_ACCESS_USER,
            44,
            false,
            FidLookupInputs {
                dentry_unhashed_after_walk: true,
                ..FidLookupInputs::success()
            },
        );
        assert_eq!(unhashed, Err(-ENOENT));
    }
}

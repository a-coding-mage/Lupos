// SPDX-License-Identifier: GPL-2.0 WITH Linux-syscall-note
//
// File: linux/nfsacl.h
//
// (C) 2003 Andreas Gruenbacher <agruen@suse.de>
//! linux-source: include/uapi/linux/nfsacl.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016315

// The Rust module boundary provides the source header's once-only inclusion
// semantics; this unit marker retains the guard's name and empty value.
#[doc(hidden)]
pub const _UAPI__LINUX_NFSACL_H: () = ();

pub const NFS_ACL_PROGRAM: u32 = 100227;

pub const ACLPROC2_NULL: u32 = 0;
pub const ACLPROC2_GETACL: u32 = 1;
pub const ACLPROC2_SETACL: u32 = 2;
pub const ACLPROC2_GETATTR: u32 = 3;
pub const ACLPROC2_ACCESS: u32 = 4;

pub const ACLPROC3_NULL: u32 = 0;
pub const ACLPROC3_GETACL: u32 = 1;
pub const ACLPROC3_SETACL: u32 = 2;

// Flags for the getacl/setacl mode.
pub const NFS_ACL: u32 = 0x0001;
pub const NFS_ACLCNT: u32 = 0x0002;
pub const NFS_DFACL: u32 = 0x0004;
pub const NFS_DFACLCNT: u32 = 0x0008;
pub const NFS_ACL_MASK: u32 = 0x000f;

// Flag for Default ACL entries.
pub const NFS_ACL_DEFAULT: u32 = 0x1000;

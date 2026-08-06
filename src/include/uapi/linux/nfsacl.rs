// SPDX-License-Identifier: GPL-2.0 WITH Linux-syscall-note
/*
 * (C) 2003 Andreas Gruenbacher <agruen@suse.de>
 */
//! linux-source: include/uapi/linux/nfsacl.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016315

// NFS ACL RPC program number.
pub const NFS_ACL_PROGRAM: i32 = 100227;

// NFSv2 ACL RPC procedure numbers.
pub const ACLPROC2_NULL: i32 = 0;
pub const ACLPROC2_GETACL: i32 = 1;
pub const ACLPROC2_SETACL: i32 = 2;
pub const ACLPROC2_GETATTR: i32 = 3;
pub const ACLPROC2_ACCESS: i32 = 4;

// NFSv3 ACL RPC procedure numbers.
pub const ACLPROC3_NULL: i32 = 0;
pub const ACLPROC3_GETACL: i32 = 1;
pub const ACLPROC3_SETACL: i32 = 2;

// Flags for the getacl/setacl mode.
pub const NFS_ACL: i32 = 0x0001;
pub const NFS_ACLCNT: i32 = 0x0002;
pub const NFS_DFACL: i32 = 0x0004;
pub const NFS_DFACLCNT: i32 = 0x0008;
pub const NFS_ACL_MASK: i32 = 0x000f;

// Flag for default ACL entries.
pub const NFS_ACL_DEFAULT: i32 = 0x1000;

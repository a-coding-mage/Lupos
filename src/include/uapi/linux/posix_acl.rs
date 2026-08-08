// SPDX-License-Identifier: LGPL-2.1+ WITH Linux-syscall-note
//! linux-source: include/uapi/linux/posix_acl.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016334

/*
 * The C include guard __UAPI_POSIX_ACL_H has no Rust analogue.  These public
 * constants preserve the values of the UAPI macros from the guarded header.
 */

/// Undefined user or group identifier.
pub const ACL_UNDEFINED_ID: i32 = -1;

/// Values of the `a_type` field in `acl_user_posix_entry_t`.
pub const ACL_TYPE_ACCESS: i32 = 0x8000;
pub const ACL_TYPE_DEFAULT: i32 = 0x4000;

/// Values of the `e_tag` field in `struct posix_acl_entry`.
pub const ACL_USER_OBJ: i32 = 0x01;
pub const ACL_USER: i32 = 0x02;
pub const ACL_GROUP_OBJ: i32 = 0x04;
pub const ACL_GROUP: i32 = 0x08;
pub const ACL_MASK: i32 = 0x10;
pub const ACL_OTHER: i32 = 0x20;

/// Permission bits in the `e_perm` field.
pub const ACL_READ: i32 = 0x04;
pub const ACL_WRITE: i32 = 0x02;
pub const ACL_EXECUTE: i32 = 0x01;

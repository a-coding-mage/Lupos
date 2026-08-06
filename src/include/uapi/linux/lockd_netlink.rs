// SPDX-License-Identifier: ((GPL-2.0 WITH Linux-syscall-note) OR BSD-3-Clause)
//! linux-source: include/uapi/linux/lockd_netlink.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016228

// Do not edit directly, auto-generated from:
// 	Documentation/netlink/specs/lockd.yaml
// YNL-GEN uapi header
// To regenerate run: tools/net/ynl/ynl-regen.sh

use core::ffi::c_int;

// The C macro expands to a `char[6]` string literal, including its trailing
// NUL. The selected use initializes `struct genl_family.name`, an inline
// `char[GENL_NAMSIZ]` aggregate; it does not perform pointer decay. Preserve
// the literal's stable unsigned-byte storage under the frozen `-funsigned-char`
// commands. A Rust pointer-consuming use corresponding to a C expansion makes
// that conversion explicitly at its corresponding source expression.
pub static LOCKD_FAMILY_NAME: [u8; 6] = *b"lockd\0";
pub const LOCKD_FAMILY_VERSION: c_int = 1;

// Anonymous C enum at source line 13.  The frozen GNU11 configurations do
// not enable `-fshort-enums`, so these enumerator integer constants use C
// `int` representation on both selected architectures.
pub const LOCKD_A_SERVER_GRACETIME: c_int = 1;
pub const LOCKD_A_SERVER_TCP_PORT: c_int = 2;
pub const LOCKD_A_SERVER_UDP_PORT: c_int = 3;
pub const __LOCKD_A_SERVER_MAX: c_int = 4;
pub const LOCKD_A_SERVER_MAX: c_int = __LOCKD_A_SERVER_MAX - 1;

// Anonymous C enum at source line 22.
pub const LOCKD_CMD_SERVER_SET: c_int = 1;
pub const LOCKD_CMD_SERVER_GET: c_int = 2;
pub const __LOCKD_CMD_MAX: c_int = 3;
pub const LOCKD_CMD_MAX: c_int = __LOCKD_CMD_MAX - 1;

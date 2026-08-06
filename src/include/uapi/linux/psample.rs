// SPDX-License-Identifier: GPL-2.0 WITH Linux-syscall-note
//! linux-source: include/uapi/linux/psample.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016342

/*
 * UAPI generic-netlink command and attribute values for packet sampling.
 *
 * C enumerators are `int` constant expressions. The two named C enum tags
 * are separate transparent C-`int` wrappers for typed ABI positions; their
 * public tuple fields retain every C enum object representation. The
 * unscoped enumerators themselves remain integer constants, as in the UAPI
 * header.
 */

pub const PSAMPLE_ATTR_IIFINDEX: i32 = 0;
pub const PSAMPLE_ATTR_OIFINDEX: i32 = 1;
pub const PSAMPLE_ATTR_ORIGSIZE: i32 = 2;
pub const PSAMPLE_ATTR_SAMPLE_GROUP: i32 = 3;
pub const PSAMPLE_ATTR_GROUP_SEQ: i32 = 4;
/* u32 ratio between observed and sampled packets, or scaled probability when
 * PSAMPLE_ATTR_SAMPLE_PROBABILITY is set. */
pub const PSAMPLE_ATTR_SAMPLE_RATE: i32 = 5;
pub const PSAMPLE_ATTR_DATA: i32 = 6;
pub const PSAMPLE_ATTR_GROUP_REFCOUNT: i32 = 7;
pub const PSAMPLE_ATTR_TUNNEL: i32 = 8;
pub const PSAMPLE_ATTR_PAD: i32 = 9;
/* u16 */
pub const PSAMPLE_ATTR_OUT_TC: i32 = 10;
/* u64 bytes */
pub const PSAMPLE_ATTR_OUT_TC_OCC: i32 = 11;
/* u64 nanoseconds */
pub const PSAMPLE_ATTR_LATENCY: i32 = 12;
/* u64 nanoseconds */
pub const PSAMPLE_ATTR_TIMESTAMP: i32 = 13;
/* u16 */
pub const PSAMPLE_ATTR_PROTO: i32 = 14;
/* Binary, user-provided data. */
pub const PSAMPLE_ATTR_USER_COOKIE: i32 = 15;
/* No argument; interpret PSAMPLE_ATTR_SAMPLE_RATE as a probability scaled
 * from 0 through U32_MAX. */
pub const PSAMPLE_ATTR_SAMPLE_PROBABILITY: i32 = 16;
pub const __PSAMPLE_ATTR_MAX: i32 = 17;

/*
 * `enum psample_command` is a separately named C enum type. The frozen clang
 * commands for both targets omit `-fshort-enums`, so this transparent C-`int`
 * representation preserves its 32-bit ABI without introducing Rust
 * enum-discriminant validity restrictions on an object received from C.
 */
#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct psample_command(pub i32);

pub const PSAMPLE_CMD_SAMPLE: i32 = 0;
pub const PSAMPLE_CMD_GET_GROUP: i32 = 1;
pub const PSAMPLE_CMD_NEW_GROUP: i32 = 2;
pub const PSAMPLE_CMD_DEL_GROUP: i32 = 3;

/* `enum psample_tunnel_key_attr` has the same frozen-target C-`int` ABI but
 * remains a distinct public tag from `psample_command`. */
#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct psample_tunnel_key_attr(pub i32);

/* be64 tunnel ID */
pub const PSAMPLE_TUNNEL_KEY_ATTR_ID: i32 = 0;
/* be32 source IP address */
pub const PSAMPLE_TUNNEL_KEY_ATTR_IPV4_SRC: i32 = 1;
/* be32 destination IP address */
pub const PSAMPLE_TUNNEL_KEY_ATTR_IPV4_DST: i32 = 2;
/* u8 tunnel IP ToS */
pub const PSAMPLE_TUNNEL_KEY_ATTR_TOS: i32 = 3;
/* u8 tunnel IP TTL */
pub const PSAMPLE_TUNNEL_KEY_ATTR_TTL: i32 = 4;
/* No argument: set DF. */
pub const PSAMPLE_TUNNEL_KEY_ATTR_DONT_FRAGMENT: i32 = 5;
/* No argument: checksum packet. */
pub const PSAMPLE_TUNNEL_KEY_ATTR_CSUM: i32 = 6;
/* No argument: OAM frame. */
pub const PSAMPLE_TUNNEL_KEY_ATTR_OAM: i32 = 7;
/* Array of Geneve options. */
pub const PSAMPLE_TUNNEL_KEY_ATTR_GENEVE_OPTS: i32 = 8;
/* be16 source transport port */
pub const PSAMPLE_TUNNEL_KEY_ATTR_TP_SRC: i32 = 9;
/* be16 destination transport port */
pub const PSAMPLE_TUNNEL_KEY_ATTR_TP_DST: i32 = 10;
/* Nested VXLAN options. */
pub const PSAMPLE_TUNNEL_KEY_ATTR_VXLAN_OPTS: i32 = 11;
/* struct in6_addr source IPv6 address */
pub const PSAMPLE_TUNNEL_KEY_ATTR_IPV6_SRC: i32 = 12;
/* struct in6_addr destination IPv6 address */
pub const PSAMPLE_TUNNEL_KEY_ATTR_IPV6_DST: i32 = 13;
pub const PSAMPLE_TUNNEL_KEY_ATTR_PAD: i32 = 14;
/* struct erspan_metadata */
pub const PSAMPLE_TUNNEL_KEY_ATTR_ERSPAN_OPTS: i32 = 15;
/* No argument: IPV4_INFO_BRIDGE mode. */
pub const PSAMPLE_TUNNEL_KEY_ATTR_IPV4_INFO_BRIDGE: i32 = 16;
pub const __PSAMPLE_TUNNEL_KEY_ATTR_MAX: i32 = 17;

/* May be overridden at runtime by a module option. */
pub const PSAMPLE_ATTR_MAX: i32 = __PSAMPLE_ATTR_MAX - 1;

/*
 * C string-literal macro expansions: array values with their terminating NUL.
 * `-funsigned-char` is present in both frozen commands, so `u8` preserves the
 * C `char` element representation. A translated aggregate initializer uses
 * the value directly; a C-pointer context uses `.as_ptr()` at that use site.
 */
pub const PSAMPLE_NL_MCGRP_CONFIG_NAME: [u8; 7] = *b"config\0";
pub const PSAMPLE_NL_MCGRP_SAMPLE_NAME: [u8; 8] = *b"packets\0";
pub const PSAMPLE_GENL_NAME: [u8; 8] = *b"psample\0";
pub const PSAMPLE_GENL_VERSION: i32 = 1;

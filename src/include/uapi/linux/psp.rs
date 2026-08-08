// SPDX-License-Identifier: ((GPL-2.0 WITH Linux-syscall-note) OR BSD-3-Clause)
//! linux-source: include/uapi/linux/psp.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016344

pub const PSP_FAMILY_NAME: &str = "psp";
pub const PSP_FAMILY_VERSION: core::ffi::c_int = 1;

#[repr(C)]
pub enum psp_version {
    PSP_VERSION_HDR0_AES_GCM_128 = 0,
    PSP_VERSION_HDR0_AES_GCM_256,
    PSP_VERSION_HDR0_AES_GMAC_128,
    PSP_VERSION_HDR0_AES_GMAC_256,
}

pub const PSP_A_ASSOC_DEV_INFO_IFINDEX: core::ffi::c_int = 1;
pub const PSP_A_ASSOC_DEV_INFO_NSID: core::ffi::c_int = 2;
pub const __PSP_A_ASSOC_DEV_INFO_MAX: core::ffi::c_int = 3;
pub const PSP_A_ASSOC_DEV_INFO_MAX: core::ffi::c_int = __PSP_A_ASSOC_DEV_INFO_MAX - 1;

pub const PSP_A_DEV_ID: core::ffi::c_int = 1;
pub const PSP_A_DEV_IFINDEX: core::ffi::c_int = 2;
pub const PSP_A_DEV_PSP_VERSIONS_CAP: core::ffi::c_int = 3;
pub const PSP_A_DEV_PSP_VERSIONS_ENA: core::ffi::c_int = 4;
pub const PSP_A_DEV_ASSOC_LIST: core::ffi::c_int = 5;
pub const PSP_A_DEV_NSID: core::ffi::c_int = 6;
pub const PSP_A_DEV_BY_ASSOCIATION: core::ffi::c_int = 7;
pub const __PSP_A_DEV_MAX: core::ffi::c_int = 8;
pub const PSP_A_DEV_MAX: core::ffi::c_int = __PSP_A_DEV_MAX - 1;

pub const PSP_A_ASSOC_DEV_ID: core::ffi::c_int = 1;
pub const PSP_A_ASSOC_VERSION: core::ffi::c_int = 2;
pub const PSP_A_ASSOC_RX_KEY: core::ffi::c_int = 3;
pub const PSP_A_ASSOC_TX_KEY: core::ffi::c_int = 4;
pub const PSP_A_ASSOC_SOCK_FD: core::ffi::c_int = 5;
pub const __PSP_A_ASSOC_MAX: core::ffi::c_int = 6;
pub const PSP_A_ASSOC_MAX: core::ffi::c_int = __PSP_A_ASSOC_MAX - 1;

pub const PSP_A_KEYS_KEY: core::ffi::c_int = 1;
pub const PSP_A_KEYS_SPI: core::ffi::c_int = 2;
pub const __PSP_A_KEYS_MAX: core::ffi::c_int = 3;
pub const PSP_A_KEYS_MAX: core::ffi::c_int = __PSP_A_KEYS_MAX - 1;

pub const PSP_A_STATS_DEV_ID: core::ffi::c_int = 1;
pub const PSP_A_STATS_KEY_ROTATIONS: core::ffi::c_int = 2;
pub const PSP_A_STATS_STALE_EVENTS: core::ffi::c_int = 3;
pub const PSP_A_STATS_RX_PACKETS: core::ffi::c_int = 4;
pub const PSP_A_STATS_RX_BYTES: core::ffi::c_int = 5;
pub const PSP_A_STATS_RX_AUTH_FAIL: core::ffi::c_int = 6;
pub const PSP_A_STATS_RX_ERROR: core::ffi::c_int = 7;
pub const PSP_A_STATS_RX_BAD: core::ffi::c_int = 8;
pub const PSP_A_STATS_TX_PACKETS: core::ffi::c_int = 9;
pub const PSP_A_STATS_TX_BYTES: core::ffi::c_int = 10;
pub const PSP_A_STATS_TX_ERROR: core::ffi::c_int = 11;
pub const __PSP_A_STATS_MAX: core::ffi::c_int = 12;
pub const PSP_A_STATS_MAX: core::ffi::c_int = __PSP_A_STATS_MAX - 1;

pub const PSP_CMD_DEV_GET: core::ffi::c_int = 1;
pub const PSP_CMD_DEV_ADD_NTF: core::ffi::c_int = 2;
pub const PSP_CMD_DEV_DEL_NTF: core::ffi::c_int = 3;
pub const PSP_CMD_DEV_SET: core::ffi::c_int = 4;
pub const PSP_CMD_DEV_CHANGE_NTF: core::ffi::c_int = 5;
pub const PSP_CMD_KEY_ROTATE: core::ffi::c_int = 6;
pub const PSP_CMD_KEY_ROTATE_NTF: core::ffi::c_int = 7;
pub const PSP_CMD_RX_ASSOC: core::ffi::c_int = 8;
pub const PSP_CMD_TX_ASSOC: core::ffi::c_int = 9;
pub const PSP_CMD_GET_STATS: core::ffi::c_int = 10;
pub const PSP_CMD_DEV_ASSOC: core::ffi::c_int = 11;
pub const PSP_CMD_DEV_DISASSOC: core::ffi::c_int = 12;
pub const __PSP_CMD_MAX: core::ffi::c_int = 13;
pub const PSP_CMD_MAX: core::ffi::c_int = __PSP_CMD_MAX - 1;

pub const PSP_MCGRP_MGMT: &str = "mgmt";
pub const PSP_MCGRP_USE: &str = "use";

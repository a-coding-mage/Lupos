// SPDX-License-Identifier: GPL-2.0 WITH Linux-syscall-note
//! linux-source: include/uapi/linux/netfilter/nf_log.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016275

pub const NF_LOG_TCPSEQ: core::ffi::c_int = 0x01;
pub const NF_LOG_TCPOPT: core::ffi::c_int = 0x02;
pub const NF_LOG_IPOPT: core::ffi::c_int = 0x04;
pub const NF_LOG_UID: core::ffi::c_int = 0x08;
pub const NF_LOG_NFLOG: core::ffi::c_int = 0x10;
pub const NF_LOG_MACDECODE: core::ffi::c_int = 0x20;
pub const NF_LOG_MASK: core::ffi::c_int = 0x2f;

pub const NF_LOG_PREFIXLEN: core::ffi::c_int = 128;

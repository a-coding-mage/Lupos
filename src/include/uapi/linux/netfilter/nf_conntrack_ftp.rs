// SPDX-License-Identifier: GPL-2.0 WITH Linux-syscall-note
//! linux-source: include/uapi/linux/netfilter/nf_conntrack_ftp.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: x86_64
//! rewrite-task: S016271

#![allow(non_camel_case_types, non_upper_case_globals)]

use core::ffi::c_int;

/* FTP tracking. */

/* This enum is exposed to userspace.  Its C ABI is `int`; keeping it as a
 * transparent integer wrapper also permits values outside the named
 * enumerators, as C does. */
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct nf_ct_ftp_type(pub c_int);

/* PORT command from client */
pub const NF_CT_FTP_PORT: nf_ct_ftp_type = nf_ct_ftp_type(0);
/* PASV response from server */
pub const NF_CT_FTP_PASV: nf_ct_ftp_type = nf_ct_ftp_type(1);
/* EPRT command from client */
pub const NF_CT_FTP_EPRT: nf_ct_ftp_type = nf_ct_ftp_type(2);
/* EPSV response from server */
pub const NF_CT_FTP_EPSV: nf_ct_ftp_type = nf_ct_ftp_type(3);

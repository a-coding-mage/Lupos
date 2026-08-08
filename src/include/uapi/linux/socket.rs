// SPDX-License-Identifier: GPL-2.0 WITH Linux-syscall-note
//! linux-source: include/uapi/linux/socket.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016386

/// Implementation-specific maximum size of a socket address.
pub const _K_SS_MAXSIZE: usize = 128;

/// Address-family value carried by a kernel socket address.
pub type __kernel_sa_family_t = u16;

/// The address-family portion and implementation-specific storage of
/// `__kernel_sockaddr_storage`.
#[repr(C)]
pub struct __kernel_sockaddr_storage_data {
    pub ss_family: __kernel_sa_family_t,
    pub __data: [u8; _K_SS_MAXSIZE - core::mem::size_of::<u16>()],
}

/// The C anonymous union used to control the default alignment of
/// `__kernel_sockaddr_storage`.
#[repr(C)]
pub union __kernel_sockaddr_storage_union {
    pub __data: __kernel_sockaddr_storage_data,
    pub __align: *mut core::ffi::c_void,
}

/// Opaque, fixed-size storage for a socket address.
#[repr(C)]
pub struct __kernel_sockaddr_storage {
    pub __storage: __kernel_sockaddr_storage_union,
}

pub const SOCK_SNDBUF_LOCK: u32 = 1;
pub const SOCK_RCVBUF_LOCK: u32 = 2;
pub const SOCK_BUF_LOCK_MASK: u32 = SOCK_SNDBUF_LOCK | SOCK_RCVBUF_LOCK;

pub const SOCK_TXREHASH_DEFAULT: u32 = 255;
pub const SOCK_TXREHASH_DISABLED: u32 = 0;
pub const SOCK_TXREHASH_ENABLED: u32 = 1;

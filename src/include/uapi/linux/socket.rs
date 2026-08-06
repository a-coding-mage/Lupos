// SPDX-License-Identifier: GPL-2.0 WITH Linux-syscall-note
//! linux-source: include/uapi/linux/socket.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016386

/*
 * Desired design of maximum size and alignment (see RFC2553).
 *
 * `_K_SS_MAXSIZE` is an unsuffixed C integer literal macro.  It is a `usize`
 * here solely because Rust array lengths use `usize`; the resulting array
 * bound remains the C expression `128 - sizeof(unsigned short)`.
 */
pub const _K_SS_MAXSIZE: usize = 128;

pub type __kernel_sa_family_t = u16;

/*
 * C exposes the members of these anonymous aggregate types directly through
 * `__kernel_sockaddr_storage`.  Rust has no anonymous union or struct, so the
 * implementation-required aggregate types are named while preserving their
 * exact C representation, member types, size, and alignment.
 */
#[repr(C)]
#[derive(Copy, Clone)]
pub struct __kernel_sockaddr_storage__anonymous_struct {
    /* address family */
    pub ss_family: __kernel_sa_family_t,
    /* Following field(s) are implementation specific. */
    pub __data: [i8; _K_SS_MAXSIZE - core::mem::size_of::<u16>()],
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union __kernel_sockaddr_storage__anonymous_union {
    pub __anonymous_struct: __kernel_sockaddr_storage__anonymous_struct,
    /* implementation specific desired alignment */
    pub __align: *mut core::ffi::c_void,
}

/*
 * The definition uses an anonymous union and struct in order to control the
 * default alignment.
 */
#[repr(C)]
#[derive(Copy, Clone)]
pub struct __kernel_sockaddr_storage {
    pub __anonymous_union: __kernel_sockaddr_storage__anonymous_union,
}

/* The following object-like macros are unsuffixed C `int` expressions. */
pub const SOCK_SNDBUF_LOCK: i32 = 1;
pub const SOCK_RCVBUF_LOCK: i32 = 2;

pub const SOCK_BUF_LOCK_MASK: i32 = SOCK_SNDBUF_LOCK | SOCK_RCVBUF_LOCK;

pub const SOCK_TXREHASH_DEFAULT: i32 = 255;
pub const SOCK_TXREHASH_DISABLED: i32 = 0;
pub const SOCK_TXREHASH_ENABLED: i32 = 1;

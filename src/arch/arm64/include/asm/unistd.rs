// SPDX-License-Identifier: GPL-2.0-only
//! linux-source: arch/arm64/include/asm/unistd.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: aarch64
//! rewrite-task: S000188

/*
 * Copyright (C) 2012 ARM Ltd.
 */

#![allow(non_upper_case_globals)]

// CONFIG_COMPAT is enabled by the frozen AArch64 configuration.  These unit
// markers preserve the presence of the corresponding C selection macros.
pub const __ARCH_WANT_COMPAT_STAT: () = ();
pub const __ARCH_WANT_COMPAT_STAT64: () = ();
pub const __ARCH_WANT_SYS_GETHOSTNAME: () = ();
pub const __ARCH_WANT_SYS_PAUSE: () = ();
pub const __ARCH_WANT_SYS_GETPGRP: () = ();
pub const __ARCH_WANT_SYS_NICE: () = ();
pub const __ARCH_WANT_SYS_SIGPENDING: () = ();
pub const __ARCH_WANT_SYS_SIGPROCMASK: () = ();
pub const __ARCH_WANT_COMPAT_SYS_SENDFILE: () = ();
pub const __ARCH_WANT_SYS_UTIME32: () = ();
pub const __ARCH_WANT_SYS_FORK: () = ();
pub const __ARCH_WANT_SYS_VFORK: () = ();

/* The following SVCs are ARM private. */
pub const __ARM_NR_COMPAT_BASE: i32 = 0x0f0000;
pub const __ARM_NR_compat_cacheflush: i32 = __ARM_NR_COMPAT_BASE + 2;
pub const __ARM_NR_compat_set_tls: i32 = __ARM_NR_COMPAT_BASE + 5;
pub const __ARM_NR_COMPAT_END: i32 = __ARM_NR_COMPAT_BASE + 0x800;

pub const __ARCH_WANT_SYS_CLONE: () = ();
pub const __ARCH_WANT_NEW_STAT: () = ();

// The included generated asm/unistd_64.h is BUILD_METADATA (S012326); under
// the frozen configuration it defines __NR_syscalls as 472.
pub const NR_syscalls: usize = 472;

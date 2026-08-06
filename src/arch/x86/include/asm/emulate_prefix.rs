// SPDX-License-Identifier: GPL-2.0-only
//! linux-source: arch/x86/include/asm/emulate_prefix.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: x86_64
//! rewrite-task: S000520

/*
 * Virtualization escape sequences that force instruction emulation.  Each
 * sequence is UD2 followed by its three-byte virtualization signature.
 */

/// `ud2; .ascii "xen"`.
pub const __XEN_EMULATE_PREFIX: [u8; 5] = [0x0f, 0x0b, 0x78, 0x65, 0x6e];

/// `ud2; .ascii "kvm"`.
pub const __KVM_EMULATE_PREFIX: [u8; 5] = [0x0f, 0x0b, 0x6b, 0x76, 0x6d];

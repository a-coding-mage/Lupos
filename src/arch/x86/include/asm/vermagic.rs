// SPDX-License-Identifier: GPL-2.0
//! linux-source: arch/x86/include/asm/vermagic.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: x86_64
//! rewrite-task: S000749

/// Linux's x86_64 configuration does not define `MODULE_PROC_FAMILY`.
///
/// Under `CONFIG_X86_64`, the upstream header selects no processor-family
/// token.  Since the frozen task architecture is x86_64, the subsequent
/// `CONFIG_X86_32` branch is false and `MODULE_ARCH_VERMAGIC` is the empty
/// string.
pub const MODULE_ARCH_VERMAGIC: &str = "";

// SPDX-License-Identifier: GPL-2.0-only
//! linux-source: arch/x86/include/asm/vermagic.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: x86_64
//! rewrite-task: S000749

/// The architecture contribution to a module's compile-time version magic.
///
/// Under the frozen x86_64 configuration, `CONFIG_X86_64` selects the Linux
/// header branch in which `MODULE_PROC_FAMILY` is intentionally absent and
/// `MODULE_ARCH_VERMAGIC` expands to this empty string literal.  Keeping it as
/// a macro lets its consumer remain a compile-time string-token composition.
#[cfg(target_arch = "x86_64")]
#[macro_export]
macro_rules! MODULE_ARCH_VERMAGIC {
    () => {
        ""
    };
}

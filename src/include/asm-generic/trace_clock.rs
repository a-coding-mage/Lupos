// SPDX-License-Identifier: GPL-2.0-only
//! linux-source: include/asm-generic/trace_clock.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: aarch64
//! rewrite-task: S012594

//! Default architecture trace-clock contribution.  The AArch64 generic include
//! path has no architecture-specific override, so this expansion contributes
//! no entries to the tracing clock array.

/// Rust mapping of the empty `ARCH_TRACE_CLOCKS` replacement list.
///
/// This is token-level behavior: an invocation expands to no tokens, as does
/// the selected C fallback macro.
#[macro_export]
macro_rules! ARCH_TRACE_CLOCKS {
    () => {};
}

// SPDX-License-Identifier: GPL-2.0-only
//! linux-source: arch/arm64/include/asm/tlbbatch.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: aarch64
//! rewrite-task: S000182

/// ARM64 carries no per-batch state: hardware broadcasts the TLB invalidation.
///
/// This remains an empty C-layout type because it is embedded in the generic
/// `tlbflush_unmap_batch` only when batched unmap TLB flushing is enabled.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct arch_tlbflush_unmap_batch {}

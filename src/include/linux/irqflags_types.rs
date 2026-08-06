// SPDX-License-Identifier: GPL-2.0-only
//! linux-source: include/linux/irqflags_types.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S014144

/*
 * The only declaration in the upstream header is guarded by
 * CONFIG_TRACE_IRQFLAGS.  That symbol is absent from both frozen
 * configurations; TRACE_IRQFLAGS_SUPPORT and TRACE_IRQFLAGS_NMI_SUPPORT do
 * not select it.  Therefore this header contributes no declarations to the
 * approved configuration union.
 */

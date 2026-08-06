// SPDX-License-Identifier: GPL-2.0-or-later
//! linux-source: arch/x86/include/asm/inat_types.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: x86_64
//! rewrite-task: S000555

// x86 instruction attributes.
// Written by Masami Hiramatsu <mhiramat@redhat.com>

/// Instruction attribute word.
#[allow(non_camel_case_types)]
pub type insn_attr_t = u32;

#[allow(non_camel_case_types)]
pub type insn_byte_t = u8;

#[allow(non_camel_case_types)]
pub type insn_value_t = i32;

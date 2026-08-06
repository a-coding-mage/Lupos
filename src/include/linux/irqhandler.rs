// SPDX-License-Identifier: GPL-2.0
//! linux-source: include/linux/irqhandler.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S014145

//! Interrupt flow-handler declarations.
//!
//! Linux keeps this typedef in a dedicated header so users can name a flow
//! handler without recursively including the complete interrupt descriptor.

/// C `struct irq_desc`, intentionally incomplete at this header boundary.
///
/// This declaration must not be replaced by an import of the later completed
/// descriptor: `irq.h` includes this header before `irqdesc.h`, and `irqdesc.h`
/// itself contains an `irq_flow_handler_t` field.
#[allow(non_camel_case_types)]
pub enum irq_desc {}

/// C `irq_flow_handler_t`: a nullable C-ABI flow-handler pointer.
///
/// The descriptor remains incomplete here and is used only through its raw
/// address. `Option` preserves the null function-pointer value permitted by
/// the C typedef.
#[allow(non_camel_case_types)]
pub type irq_flow_handler_t = Option<unsafe extern "C" fn(desc: *mut irq_desc)>;

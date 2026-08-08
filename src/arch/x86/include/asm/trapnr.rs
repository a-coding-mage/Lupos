// SPDX-License-Identifier: GPL-2.0
//! linux-source: arch/x86/include/asm/trapnr.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: x86_64
//! rewrite-task: S000730

/*
 * Event type codes used by FRED, Intel VT-x and AMD SVM.
 *
 * These constants retain the C preprocessor names and the untyped integer
 * values of the source header; callers apply the same context-specific type
 * conversions as the original macros.
 */
pub const EVENT_TYPE_EXTINT: i32 = 0;
pub const EVENT_TYPE_RESERVED: i32 = 1;
pub const EVENT_TYPE_NMI: i32 = 2;
pub const EVENT_TYPE_HWEXC: i32 = 3;
pub const EVENT_TYPE_SWINT: i32 = 4;
pub const EVENT_TYPE_PRIV_SWEXC: i32 = 5;
pub const EVENT_TYPE_SWEXC: i32 = 6;
pub const EVENT_TYPE_OTHER: i32 = 7;

/* Interrupts/Exceptions */

pub const X86_TRAP_DE: i32 = 0;
pub const X86_TRAP_DB: i32 = 1;
pub const X86_TRAP_NMI: i32 = 2;
pub const X86_TRAP_BP: i32 = 3;
pub const X86_TRAP_OF: i32 = 4;
pub const X86_TRAP_BR: i32 = 5;
pub const X86_TRAP_UD: i32 = 6;
pub const X86_TRAP_NM: i32 = 7;
pub const X86_TRAP_DF: i32 = 8;
pub const X86_TRAP_OLD_MF: i32 = 9;
pub const X86_TRAP_TS: i32 = 10;
pub const X86_TRAP_NP: i32 = 11;
pub const X86_TRAP_SS: i32 = 12;
pub const X86_TRAP_GP: i32 = 13;
pub const X86_TRAP_PF: i32 = 14;
pub const X86_TRAP_SPURIOUS: i32 = 15;
pub const X86_TRAP_MF: i32 = 16;
pub const X86_TRAP_AC: i32 = 17;
pub const X86_TRAP_MC: i32 = 18;
pub const X86_TRAP_XF: i32 = 19;
pub const X86_TRAP_VE: i32 = 20;
pub const X86_TRAP_CP: i32 = 21;
pub const X86_TRAP_VC: i32 = 29;
pub const X86_TRAP_IRET: i32 = 32;

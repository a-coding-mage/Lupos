// SPDX-License-Identifier: GPL-2.0-only
//! linux-source: arch/x86/include/asm/trapnr.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: x86_64
//! rewrite-task: S000730

//! x86 interrupt/exception vector numbers and FRED/virtualization event types.
//!
//! The source macros use unsuffixed decimal integer literals.  On the frozen
//! x86_64 C target each expands as a signed `int`; these constants retain that
//! type so comparisons, switch discriminants, shifts, and assignments at their
//! translated use sites begin with the same integer width and signedness.

// Event type codes used by FRED, Intel VT-x and AMD SVM.
pub const EVENT_TYPE_EXTINT: i32 = 0; // External interrupt
pub const EVENT_TYPE_RESERVED: i32 = 1;
pub const EVENT_TYPE_NMI: i32 = 2; // NMI
pub const EVENT_TYPE_HWEXC: i32 = 3; // Hardware originated traps, exceptions
pub const EVENT_TYPE_SWINT: i32 = 4; // INT n
pub const EVENT_TYPE_PRIV_SWEXC: i32 = 5; // INT1
pub const EVENT_TYPE_SWEXC: i32 = 6; // INTO, INT3
pub const EVENT_TYPE_OTHER: i32 = 7; // FRED SYSCALL/SYSENTER, VT-x MTF

// Interrupts/Exceptions.
pub const X86_TRAP_DE: i32 = 0; // Divide-by-zero
pub const X86_TRAP_DB: i32 = 1; // Debug
pub const X86_TRAP_NMI: i32 = 2; // Non-maskable Interrupt
pub const X86_TRAP_BP: i32 = 3; // Breakpoint
pub const X86_TRAP_OF: i32 = 4; // Overflow
pub const X86_TRAP_BR: i32 = 5; // Bound Range Exceeded
pub const X86_TRAP_UD: i32 = 6; // Invalid Opcode
pub const X86_TRAP_NM: i32 = 7; // Device Not Available
pub const X86_TRAP_DF: i32 = 8; // Double Fault
pub const X86_TRAP_OLD_MF: i32 = 9; // Coprocessor Segment Overrun
pub const X86_TRAP_TS: i32 = 10; // Invalid TSS
pub const X86_TRAP_NP: i32 = 11; // Segment Not Present
pub const X86_TRAP_SS: i32 = 12; // Stack Segment Fault
pub const X86_TRAP_GP: i32 = 13; // General Protection Fault
pub const X86_TRAP_PF: i32 = 14; // Page Fault
pub const X86_TRAP_SPURIOUS: i32 = 15; // Spurious Interrupt
pub const X86_TRAP_MF: i32 = 16; // x87 Floating-Point Exception
pub const X86_TRAP_AC: i32 = 17; // Alignment Check
pub const X86_TRAP_MC: i32 = 18; // Machine Check
pub const X86_TRAP_XF: i32 = 19; // SIMD Floating-Point Exception
pub const X86_TRAP_VE: i32 = 20; // Virtualization Exception
pub const X86_TRAP_CP: i32 = 21; // Control Protection Exception
pub const X86_TRAP_VC: i32 = 29; // VMM Communication Exception
pub const X86_TRAP_IRET: i32 = 32; // IRET Exception

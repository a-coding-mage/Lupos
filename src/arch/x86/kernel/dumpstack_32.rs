//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/dumpstack_32.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/dumpstack_32.c
//! x86-32 stack classifier.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/dumpstack_32.c

use crate::arch::x86::kernel::dumpstack::{StackInfo, StackType};
use crate::arch::x86::mm::paging::PAGE_SIZE;

pub const THREAD_SIZE: u64 = PAGE_SIZE << 1;
pub const IRQ_STACK_SIZE: u64 = THREAD_SIZE;

pub const fn stack_type_name_32(typ: StackType) -> Option<&'static str> {
    if typ.0 == StackType::IRQ.0 {
        Some("IRQ")
    } else if typ.0 == StackType::SOFTIRQ.0 {
        Some("SOFTIRQ")
    } else if typ.0 == StackType::ENTRY.0 {
        Some("ENTRY_TRAMPOLINE")
    } else if typ.0 == StackType::EXCEPTION.0 {
        Some("#DF")
    } else {
        None
    }
}

pub const fn in_hardirq_stack(
    stack: u64,
    hardirq_stack_begin: u64,
    saved_next_sp: Option<u64>,
) -> Option<StackInfo> {
    in_software_stack(stack, hardirq_stack_begin, StackType::IRQ, saved_next_sp)
}

pub const fn in_softirq_stack(
    stack: u64,
    softirq_stack_begin: u64,
    saved_next_sp: Option<u64>,
) -> Option<StackInfo> {
    in_software_stack(
        stack,
        softirq_stack_begin,
        StackType::SOFTIRQ,
        saved_next_sp,
    )
}

pub const fn in_doublefault_stack(
    stack: u64,
    doublefault_stack_begin: u64,
    tss_sp: Option<u64>,
) -> Option<StackInfo> {
    let end = doublefault_stack_begin + PAGE_SIZE;
    if stack < doublefault_stack_begin || stack >= end {
        None
    } else {
        Some(StackInfo {
            typ: StackType::EXCEPTION,
            begin: doublefault_stack_begin,
            end,
            next_sp: tss_sp,
        })
    }
}

const fn in_software_stack(
    stack: u64,
    begin: u64,
    typ: StackType,
    saved_next_sp: Option<u64>,
) -> Option<StackInfo> {
    let end = begin + THREAD_SIZE;
    if stack < begin || stack > end {
        None
    } else {
        Some(StackInfo {
            typ,
            begin,
            end,
            next_sp: saved_next_sp,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stack_names_match_linux_32bit_table() {
        assert_eq!(stack_type_name_32(StackType::IRQ), Some("IRQ"));
        assert_eq!(stack_type_name_32(StackType::SOFTIRQ), Some("SOFTIRQ"));
        assert_eq!(
            stack_type_name_32(StackType::ENTRY),
            Some("ENTRY_TRAMPOLINE")
        );
        assert_eq!(stack_type_name_32(StackType::EXCEPTION), Some("#DF"));
        assert_eq!(stack_type_name_32(StackType::TASK), None);
    }

    #[test]
    fn software_irq_stacks_allow_empty_top_pointer() {
        let begin = 0x1000_0000;
        assert!(in_hardirq_stack(begin, begin, Some(0xaaaa)).is_some());
        let info = in_softirq_stack(begin + THREAD_SIZE, begin, Some(0xbbbb)).unwrap();
        assert_eq!(info.typ, StackType::SOFTIRQ);
        assert_eq!(info.next_sp, Some(0xbbbb));
        assert!(in_hardirq_stack(begin + THREAD_SIZE + 1, begin, None).is_none());
    }

    #[test]
    fn doublefault_stack_is_page_sized_and_half_open() {
        let begin = 0x2000_0000;
        let info = in_doublefault_stack(begin + PAGE_SIZE - 1, begin, Some(0x1234)).unwrap();
        assert_eq!(info.typ, StackType::EXCEPTION);
        assert_eq!(info.end, begin + PAGE_SIZE);
        assert!(in_doublefault_stack(begin + PAGE_SIZE, begin, None).is_none());
    }
}

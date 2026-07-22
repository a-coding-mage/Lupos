//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/dumpstack_64.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/dumpstack_64.c
//! x86-64 stack classifier.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/dumpstack_64.c

use crate::arch::x86::kernel::dumpstack::{StackInfo, StackType};
use crate::arch::x86::mm::paging::PAGE_SIZE;

pub const THREAD_SIZE_ORDER: u32 = crate::kernel::sched::KTHREAD_STACK_ORDER;
pub const THREAD_SIZE: u64 = PAGE_SIZE << THREAD_SIZE_ORDER;
pub const EXCEPTION_STACK_ORDER: u32 = 1;
pub const EXCEPTION_STKSZ: u64 = PAGE_SIZE << EXCEPTION_STACK_ORDER;
pub const IRQ_STACK_SIZE: u64 = PAGE_SIZE << 2;
pub const N_EXCEPTION_STACKS: u8 = 6;

pub const ESTACK_DF: u8 = 0;
pub const ESTACK_NMI: u8 = 1;
pub const ESTACK_DB: u8 = 2;
pub const ESTACK_MCE: u8 = 3;
pub const ESTACK_VC: u8 = 4;
pub const ESTACK_VC2: u8 = 5;

pub const EXCEPTION_STACK_NAMES: [&str; N_EXCEPTION_STACKS as usize] =
    ["#DF", "NMI", "#DB", "#MC", "#VC", "#VC2"];

pub const CEA_ESTACK_UNIT: u64 = PAGE_SIZE + EXCEPTION_STKSZ;
pub const CEA_ESTACK_PAGES: u64 =
    N_EXCEPTION_STACKS as u64 * (1 + (EXCEPTION_STKSZ / PAGE_SIZE)) + 1;

pub const fn exception_stack_name(index: u8) -> Option<&'static str> {
    match index {
        ESTACK_DF => Some("#DF"),
        ESTACK_NMI => Some("NMI"),
        ESTACK_DB => Some("#DB"),
        ESTACK_MCE => Some("#MC"),
        ESTACK_VC => Some("#VC"),
        ESTACK_VC2 => Some("#VC2"),
        _ => None,
    }
}

pub const fn stack_type_name(typ: StackType) -> Option<&'static str> {
    if typ.0 == StackType::TASK.0 {
        Some("TASK")
    } else if typ.0 == StackType::IRQ.0 {
        Some("IRQ")
    } else if typ.0 == StackType::SOFTIRQ.0 {
        Some("SOFTIRQ")
    } else if typ.0 == StackType::ENTRY.0 {
        Some("ENTRY_TRAMPOLINE")
    } else if typ.0 >= StackType::EXCEPTION.0 && typ.0 < StackType::EXCEPTION.0 + N_EXCEPTION_STACKS
    {
        exception_stack_name(typ.0 - StackType::EXCEPTION.0)
    } else {
        None
    }
}

pub const fn exception_stack_bounds(cea_estacks_base: u64, index: u8) -> Option<(u64, u64)> {
    if index >= N_EXCEPTION_STACKS {
        return None;
    }
    let begin = cea_estacks_base + index as u64 * CEA_ESTACK_UNIT + PAGE_SIZE;
    Some((begin, begin + EXCEPTION_STKSZ))
}

pub const fn in_exception_stack(
    stack: u64,
    cea_estacks_base: u64,
    saved_sp: Option<u64>,
) -> Option<StackInfo> {
    if cea_estacks_base == 0 {
        return None;
    }
    let mut idx = 0;
    while idx < N_EXCEPTION_STACKS {
        if let Some((begin, end)) = exception_stack_bounds(cea_estacks_base, idx) {
            if stack >= begin && stack < end {
                return Some(StackInfo {
                    typ: StackType::exception(idx),
                    begin,
                    end,
                    next_sp: saved_sp,
                });
            }
        }
        idx += 1;
    }
    None
}

pub const fn in_irq_stack(
    stack: u64,
    hardirq_stack_top_entry: u64,
    saved_next_sp: Option<u64>,
) -> Option<StackInfo> {
    let end = hardirq_stack_top_entry + core::mem::size_of::<u64>() as u64;
    let begin = end - IRQ_STACK_SIZE;
    if stack < begin || stack >= end {
        None
    } else {
        Some(StackInfo {
            typ: StackType::IRQ,
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
    fn stack_names_match_linux_64bit_table() {
        assert_eq!(N_EXCEPTION_STACKS, 6);
        assert_eq!(stack_type_name(StackType::TASK), Some("TASK"));
        assert_eq!(stack_type_name(StackType::IRQ), Some("IRQ"));
        assert_eq!(stack_type_name(StackType::ENTRY), Some("ENTRY_TRAMPOLINE"));
        assert_eq!(
            stack_type_name(StackType::exception(ESTACK_MCE)),
            Some("#MC")
        );
        assert_eq!(stack_type_name(StackType::UNKNOWN), None);
    }

    #[test]
    fn exception_stack_bounds_include_guard_page_before_stack() {
        let base = 0x1000_0000;
        assert_eq!(
            exception_stack_bounds(base, ESTACK_DF),
            Some((base + PAGE_SIZE, base + PAGE_SIZE + EXCEPTION_STKSZ))
        );
        assert_eq!(
            exception_stack_bounds(base, ESTACK_NMI),
            Some((
                base + CEA_ESTACK_UNIT + PAGE_SIZE,
                base + CEA_ESTACK_UNIT + PAGE_SIZE + EXCEPTION_STKSZ
            ))
        );
    }

    #[test]
    fn in_exception_stack_classifies_by_linux_order() {
        let base = 0x1000_0000;
        let stack = base + CEA_ESTACK_UNIT * ESTACK_DB as u64 + PAGE_SIZE + 16;
        let info = in_exception_stack(stack, base, Some(0xfeed)).unwrap();
        assert_eq!(info.typ, StackType::exception(ESTACK_DB));
        assert_eq!(info.next_sp, Some(0xfeed));
        assert!(in_exception_stack(base, base, None).is_none());
    }

    #[test]
    fn irq_stack_is_half_open_and_top_pointer_is_adjusted() {
        let top_entry = 0x8000_0000;
        let info = in_irq_stack(top_entry - 8, top_entry, Some(0x1234)).unwrap();
        assert_eq!(info.typ, StackType::IRQ);
        assert_eq!(info.end, top_entry + 8);
        assert_eq!(info.next_sp, Some(0x1234));
        assert!(in_irq_stack(top_entry + 8, top_entry, None).is_none());
    }
}

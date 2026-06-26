//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/dumpstack.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/dumpstack.c
//! Shared x86 stack-dump classification helpers.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/dumpstack.c

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StackType(pub u8);

impl StackType {
    pub const UNKNOWN: Self = Self(0);
    pub const TASK: Self = Self(1);
    pub const IRQ: Self = Self(2);
    pub const SOFTIRQ: Self = Self(3);
    pub const ENTRY: Self = Self(4);
    pub const EXCEPTION: Self = Self(5);

    pub const fn exception(index: u8) -> Self {
        Self(Self::EXCEPTION.0 + index)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StackInfo {
    pub typ: StackType,
    pub begin: u64,
    pub end: u64,
    pub next_sp: Option<u64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PtRegsSnapshot {
    pub ip: u64,
    pub cs: u64,
    pub sp: u64,
    pub ss: u64,
    pub flags: u64,
    pub bp: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IretFrame {
    pub ss: u64,
    pub sp: u64,
    pub flags: u64,
}

pub const PROLOGUE_SIZE: usize = 42;
pub const EPILOGUE_SIZE: usize = 21;
pub const OPCODE_BUFSIZE: usize = PROLOGUE_SIZE + 1 + EPILOGUE_SIZE;

pub const fn in_task_stack(stack: u64, task_begin: u64, thread_size: u64) -> Option<StackInfo> {
    let end = task_begin + thread_size;
    if stack < task_begin || stack >= end {
        None
    } else {
        Some(StackInfo {
            typ: StackType::TASK,
            begin: task_begin,
            end,
            next_sp: None,
        })
    }
}

pub const fn in_entry_stack(stack: u64, entry_begin: u64, entry_size: u64) -> Option<StackInfo> {
    let end = entry_begin + entry_size;
    if stack < entry_begin || stack >= end {
        None
    } else {
        Some(StackInfo {
            typ: StackType::ENTRY,
            begin: entry_begin,
            end,
            next_sp: None,
        })
    }
}

pub const fn on_stack(info: StackInfo, addr: u64, len: u64) -> bool {
    if info.typ.0 == StackType::UNKNOWN.0 || len == 0 {
        return false;
    }
    let end = match addr.checked_add(len) {
        Some(value) => value,
        None => return false,
    };
    addr >= info.begin && addr < info.end && end > info.begin && end <= info.end
}

pub const fn show_ip_prefix(is_64bit: bool) -> &'static str {
    if is_64bit { "RIP" } else { "EIP" }
}

pub const fn show_iret_regs(regs: PtRegsSnapshot) -> IretFrame {
    IretFrame {
        ss: regs.ss,
        sp: regs.sp,
        flags: regs.flags,
    }
}

pub const fn mark_stack_visited(visit_mask: u64, typ: StackType) -> Result<u64, ()> {
    let bit = 1u64 << typ.0;
    if (visit_mask & bit) != 0 {
        Err(())
    } else {
        Ok(visit_mask | bit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stack_type_values_match_linux_enum() {
        assert_eq!(StackType::UNKNOWN.0, 0);
        assert_eq!(StackType::TASK.0, 1);
        assert_eq!(StackType::EXCEPTION.0, 5);
        assert_eq!(StackType::exception(2).0, 7);
    }

    #[test]
    fn task_and_entry_stacks_are_half_open() {
        assert!(in_task_stack(0x1000, 0x1000, 0x100).is_some());
        assert!(in_task_stack(0x10ff, 0x1000, 0x100).is_some());
        assert!(in_task_stack(0x1100, 0x1000, 0x100).is_none());
        assert_eq!(
            in_entry_stack(0x2000, 0x2000, 0x80).unwrap().typ,
            StackType::ENTRY
        );
    }

    #[test]
    fn on_stack_requires_full_object_to_fit() {
        let info = StackInfo {
            typ: StackType::TASK,
            begin: 0x1000,
            end: 0x2000,
            next_sp: None,
        };
        assert!(on_stack(info, 0x1ff0, 0x10));
        assert!(!on_stack(info, 0x1ff0, 0x11));
    }

    #[test]
    fn show_helpers_preserve_arch_prefix_and_iret_frame() {
        assert_eq!(show_ip_prefix(true), "RIP");
        assert_eq!(show_ip_prefix(false), "EIP");
        assert_eq!(
            show_iret_regs(PtRegsSnapshot {
                ip: 1,
                cs: 2,
                sp: 3,
                ss: 4,
                flags: 5,
                bp: 6
            }),
            IretFrame {
                ss: 4,
                sp: 3,
                flags: 5
            }
        );
    }

    #[test]
    fn visit_mask_detects_stack_recursion() {
        let mask = mark_stack_visited(0, StackType::IRQ).unwrap();
        assert_eq!(mark_stack_visited(mask, StackType::IRQ), Err(()));
    }
}

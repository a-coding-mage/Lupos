//! linux-parity: complete
//! linux-source: vendor/linux/lib/syscall.c
//! test-origin: linux:vendor/linux/lib/syscall.c
//! Source-backed syscall snapshot collection model.

use crate::include::uapi::errno::EAGAIN;

pub const SYSCALL_MAX_ARGS: usize = 6;
pub const NO_SYSCALL: i64 = -1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SyscallData {
    pub nr: i64,
    pub args: [u64; SYSCALL_MAX_ARGS],
    pub instruction_pointer: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SyscallInfo {
    pub sp: u64,
    pub data: SyscallData,
}

impl Default for SyscallInfo {
    fn default() -> Self {
        Self {
            sp: 0,
            data: SyscallData {
                nr: NO_SYSCALL,
                args: [0; SYSCALL_MAX_ARGS],
                instruction_pointer: 0,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PtRegsSnapshot {
    pub user_sp: u64,
    pub instruction_pointer: u64,
    pub syscall_nr: i64,
    pub args: [u64; SYSCALL_MAX_ARGS],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TaskStackSnapshot {
    NoStack,
    MissingRegs,
    Regs(PtRegsSnapshot),
}

pub fn collect_syscall(stack: TaskStackSnapshot) -> Result<SyscallInfo, i32> {
    let regs = match stack {
        TaskStackSnapshot::NoStack => return Ok(SyscallInfo::default()),
        TaskStackSnapshot::MissingRegs => return Err(-EAGAIN),
        TaskStackSnapshot::Regs(regs) => regs,
    };
    let mut info = SyscallInfo {
        sp: regs.user_sp,
        data: SyscallData {
            nr: regs.syscall_nr,
            args: [0; SYSCALL_MAX_ARGS],
            instruction_pointer: regs.instruction_pointer,
        },
    };
    if regs.syscall_nr != NO_SYSCALL {
        info.data.args = regs.args;
    }
    Ok(info)
}

pub fn collect_syscall_snapshot(regs: Option<PtRegsSnapshot>) -> Result<SyscallInfo, i32> {
    collect_syscall(match regs {
        Some(regs) => TaskStackSnapshot::Regs(regs),
        None => TaskStackSnapshot::NoStack,
    })
}

pub fn task_current_syscall(
    target_is_current: bool,
    target_state: u32,
    first_inactive_switches: Option<u64>,
    second_inactive_switches: Option<u64>,
    stack: TaskStackSnapshot,
) -> Result<SyscallInfo, i32> {
    if target_is_current {
        return collect_syscall(stack);
    }
    if target_state == 0 {
        return Err(-EAGAIN);
    }
    let Some(first) = first_inactive_switches else {
        return Err(-EAGAIN);
    };
    let info = collect_syscall(stack).map_err(|_| -EAGAIN)?;
    if second_inactive_switches != Some(first) {
        return Err(-EAGAIN);
    }
    Ok(info)
}

pub fn task_current_syscall_snapshot(
    target_is_current: bool,
    target_state: u32,
    first_inactive_switches: Option<u64>,
    second_inactive_switches: Option<u64>,
    regs: Option<PtRegsSnapshot>,
) -> Result<SyscallInfo, i32> {
    task_current_syscall(
        target_is_current,
        target_state,
        first_inactive_switches,
        second_inactive_switches,
        match regs {
            Some(regs) => TaskStackSnapshot::Regs(regs),
            None => TaskStackSnapshot::NoStack,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn syscall_snapshot_matches_linux_collection_flow() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/syscall.c"
        ));
        assert!(source.contains("static int collect_syscall(struct task_struct *target"));
        assert!(source.contains("unsigned long args[6] = { };"));
        assert!(source.contains("if (!try_get_task_stack(target))"));
        assert!(source.contains("memset(info, 0, sizeof(*info));"));
        assert!(source.contains("info->data.nr = -1;"));
        assert!(source.contains("regs = task_pt_regs(target);"));
        assert!(source.contains("if (unlikely(!regs))"));
        assert!(source.contains("put_task_stack(target);"));
        assert!(source.contains("info->sp = user_stack_pointer(regs);"));
        assert!(source.contains("info->data.instruction_pointer = instruction_pointer(regs);"));
        assert!(source.contains("info->data.nr = syscall_get_nr(target, regs);"));
        assert!(source.contains("if (info->data.nr != -1L)"));
        assert!(source.contains("syscall_get_arguments(target, regs, args);"));
        assert!(source.contains("info->data.args[5] = args[5];"));
        assert!(source.contains("if (target == current)"));
        assert!(source.contains("state = READ_ONCE(target->__state);"));
        assert!(source.contains("ncsw = wait_task_inactive(target, state);"));
        assert!(source.contains("unlikely(collect_syscall(target, info))"));
        assert!(source.contains("wait_task_inactive(target, state) != ncsw"));
        assert!(source.contains("return -EAGAIN;"));

        let regs = PtRegsSnapshot {
            user_sp: 0x1000,
            instruction_pointer: 0x2000,
            syscall_nr: 60,
            args: [1, 2, 3, 4, 5, 6],
        };
        let info =
            task_current_syscall_snapshot(true, 0, None, None, Some(regs)).expect("current task");
        assert_eq!(info.sp, 0x1000);
        assert_eq!(info.data.nr, 60);
        assert_eq!(info.data.args, [1, 2, 3, 4, 5, 6]);

        let no_stack = collect_syscall(TaskStackSnapshot::NoStack).expect("no stack");
        assert_eq!(no_stack.data.nr, NO_SYSCALL);
        assert_eq!(
            collect_syscall(TaskStackSnapshot::MissingRegs),
            Err(-EAGAIN)
        );
        assert_eq!(
            task_current_syscall_snapshot(false, 0, Some(1), Some(1), Some(regs)),
            Err(-EAGAIN)
        );
        assert_eq!(
            task_current_syscall_snapshot(false, 1, Some(1), Some(2), Some(regs)),
            Err(-EAGAIN)
        );
        assert_eq!(
            task_current_syscall(false, 1, Some(1), Some(1), TaskStackSnapshot::MissingRegs,),
            Err(-EAGAIN)
        );
        let not_in_syscall = PtRegsSnapshot {
            syscall_nr: NO_SYSCALL,
            args: [9; SYSCALL_MAX_ARGS],
            ..regs
        };
        let info = collect_syscall(TaskStackSnapshot::Regs(not_in_syscall)).unwrap();
        assert_eq!(info.data.nr, NO_SYSCALL);
        assert_eq!(info.data.args, [0; SYSCALL_MAX_ARGS]);
    }
}

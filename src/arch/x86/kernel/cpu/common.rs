//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/kernel/cpu/common.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/cpu/common.c
//! x86 per-CPU ABI symbols exported to Linux-built modules.
//!
//! Also carries the single-CPU cpumask objects Linux modules reference from
//! `vendor/linux/kernel/cpu.c`.

use core::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

use crate::kernel::module::{export_symbol, find_symbol};

static LINUX_PREEMPT_COUNT: AtomicU32 = AtomicU32::new(0);
static LINUX_CONST_CURRENT_TASK: AtomicUsize = AtomicUsize::new(0);

/// `struct cpumask` - `vendor/linux/include/linux/cpumask_types.h`.
#[repr(C)]
pub struct LinuxCpuMask {
    pub bits: [usize; 1],
}

static LINUX_CPU_POSSIBLE_MASK: LinuxCpuMask = LinuxCpuMask { bits: [1] };

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once(
        "__preempt_count",
        core::ptr::addr_of!(LINUX_PREEMPT_COUNT) as usize,
        true,
    );
    export_symbol_once(
        "const_current_task",
        core::ptr::addr_of!(LINUX_CONST_CURRENT_TASK) as usize,
        true,
    );
    export_symbol_once(
        "__cpu_possible_mask",
        core::ptr::addr_of!(LINUX_CPU_POSSIBLE_MASK) as usize,
        false,
    );
}

pub fn set_linux_current_task(task: *mut crate::kernel::task::TaskStruct) {
    #[cfg(not(test))]
    if crate::kernel::sched::current_cpu() != 0 {
        return;
    }
    LINUX_CONST_CURRENT_TASK.store(task as usize, Ordering::Release);
}

pub fn linux_current_task() -> *mut crate::kernel::task::TaskStruct {
    LINUX_CONST_CURRENT_TASK.load(Ordering::Acquire) as *mut crate::kernel::task::TaskStruct
}

#[cfg(test)]
pub fn linux_current_task_for_tests() -> *mut crate::kernel::task::TaskStruct {
    linux_current_task()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn x86_cpu_common_exports_register_for_modules() {
        register_module_exports();
        assert!(crate::kernel::module::find_symbol("__preempt_count").is_some());
        assert!(crate::kernel::module::find_symbol("const_current_task").is_some());
        assert_eq!(
            crate::kernel::module::find_symbol("__cpu_possible_mask"),
            Some(core::ptr::addr_of!(LINUX_CPU_POSSIBLE_MASK) as usize)
        );
        assert_eq!(LINUX_CPU_POSSIBLE_MASK.bits[0], 1);
    }

    #[test]
    fn linux_current_task_export_tracks_pointer_value() {
        let task = 0x12345000usize as *mut crate::kernel::task::TaskStruct;

        set_linux_current_task(task);

        assert_eq!(linux_current_task_for_tests(), task);
        assert_eq!(
            LINUX_CONST_CURRENT_TASK.load(Ordering::Acquire),
            task as usize
        );
    }
}

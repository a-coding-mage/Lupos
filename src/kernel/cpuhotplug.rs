//! linux-parity: partial
//! linux-source: vendor/linux/kernel/cpu.c
//! test-origin: linux:vendor/linux/kernel/cpu.c
//! Minimal CPU hotplug state exports for Linux-built modules.

use core::ffi::{c_char, c_void};
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use crate::kernel::locking::percpu_rwsem::PerCpuRwSem;
use crate::kernel::module::{export_symbol, find_symbol};

pub type CpuHpCallback = Option<unsafe extern "C" fn(cpu: u32) -> i32>;

/// `struct cpumask __cpu_online_mask` for the vendor configuration's
/// `CONFIG_NR_CPUS=64`.
///
/// Linux stores one native word in this configuration. Keep the object itself
/// atomic because `set_cpu_online()` may be called while modules read it.
#[repr(transparent)]
struct LinuxCpuMask(AtomicU64);

static LINUX_CPU_ONLINE_MASK: LinuxCpuMask = LinuxCpuMask(AtomicU64::new(1));
static LINUX_CPU_ACTIVE_MASK: LinuxCpuMask = LinuxCpuMask(AtomicU64::new(1));
static LINUX_NUM_ONLINE_CPUS: AtomicU32 = AtomicU32::new(1);
static LINUX_NR_CPU_IDS: AtomicU32 = AtomicU32::new(1);
static CPU_HOTPLUG_LOCK: PerCpuRwSem = PerCpuRwSem::new();

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once(
        "__cpu_online_mask",
        core::ptr::addr_of!(LINUX_CPU_ONLINE_MASK) as usize,
        false,
    );
    export_symbol_once(
        "__cpu_active_mask",
        core::ptr::addr_of!(LINUX_CPU_ACTIVE_MASK) as usize,
        false,
    );
    export_symbol_once(
        "__num_online_cpus",
        core::ptr::addr_of!(LINUX_NUM_ONLINE_CPUS) as usize,
        false,
    );
    export_symbol_once(
        "nr_cpu_ids",
        core::ptr::addr_of!(LINUX_NR_CPU_IDS) as usize,
        false,
    );
    export_symbol_once("cpus_read_lock", cpus_read_lock as usize, true);
    export_symbol_once("cpus_read_unlock", cpus_read_unlock as usize, true);
    export_symbol_once("__cpuhp_setup_state", __cpuhp_setup_state as usize, false);
    export_symbol_once(
        "__cpuhp_setup_state_cpuslocked",
        __cpuhp_setup_state_cpuslocked as usize,
        false,
    );
    export_symbol_once(
        "__cpuhp_state_add_instance",
        __cpuhp_state_add_instance as usize,
        true,
    );
    export_symbol_once(
        "__cpuhp_state_add_instance_cpuslocked",
        __cpuhp_state_add_instance_cpuslocked as usize,
        true,
    );
    export_symbol_once(
        "__cpuhp_state_remove_instance",
        __cpuhp_state_remove_instance as usize,
        true,
    );
    export_symbol_once("__cpuhp_remove_state", __cpuhp_remove_state as usize, false);
    export_symbol_once(
        "__cpuhp_remove_state_cpuslocked",
        __cpuhp_remove_state_cpuslocked as usize,
        false,
    );
}

/// Reset the CPU maps to the boot CPU, matching the initial
/// `set_cpu_online(0, true)` sequence in `vendor/linux/kernel/cpu.c`.
pub fn reset_cpu_maps() {
    LINUX_CPU_ONLINE_MASK.0.store(1, Ordering::Release);
    LINUX_CPU_ACTIVE_MASK.0.store(1, Ordering::Release);
    LINUX_NUM_ONLINE_CPUS.store(1, Ordering::Release);
    LINUX_NR_CPU_IDS.store(1, Ordering::Release);
}

/// Publish an online CPU to the module-facing Linux cpumask objects.
pub fn set_cpu_online(cpu: u32, online: bool) {
    if cpu >= 64 {
        return;
    }
    let bit = 1u64 << cpu;
    if online {
        let old = LINUX_CPU_ONLINE_MASK.0.fetch_or(bit, Ordering::AcqRel);
        if old & bit == 0 {
            LINUX_NUM_ONLINE_CPUS.fetch_add(1, Ordering::AcqRel);
        }
        LINUX_NR_CPU_IDS.fetch_max(cpu + 1, Ordering::AcqRel);
    } else {
        // Linux requires cpu_active_mask to remain a subset of
        // cpu_online_mask. Stop normal placement before withdrawing the CPU
        // from the module-visible online mask.
        LINUX_CPU_ACTIVE_MASK.0.fetch_and(!bit, Ordering::AcqRel);
        let old = LINUX_CPU_ONLINE_MASK.0.fetch_and(!bit, Ordering::AcqRel);
        if old & bit != 0 {
            LINUX_NUM_ONLINE_CPUS.fetch_sub(1, Ordering::AcqRel);
        }
    }
}

pub fn cpu_online_mask() -> u64 {
    LINUX_CPU_ONLINE_MASK.0.load(Ordering::Acquire)
}

/// Publish whether normal scheduler placement may target `cpu`.
///
/// Linux brings a CPU online before activating it for the scheduler. Keeping
/// the masks distinct prevents a half-initialized AP from receiving ordinary
/// tasks while still allowing per-CPU bring-up work to observe it as online.
pub fn set_cpu_active(cpu: u32, active: bool) -> bool {
    if cpu >= 64 {
        return false;
    }
    let bit = 1u64 << cpu;
    if active {
        if LINUX_CPU_ONLINE_MASK.0.load(Ordering::Acquire) & bit == 0 {
            return false;
        }
        LINUX_CPU_ACTIVE_MASK.0.fetch_or(bit, Ordering::Release);
    } else {
        LINUX_CPU_ACTIVE_MASK.0.fetch_and(!bit, Ordering::Release);
    }
    true
}

pub fn cpu_active_mask() -> u64 {
    LINUX_CPU_ACTIVE_MASK.0.load(Ordering::Acquire)
}

pub fn nr_cpu_ids() -> u32 {
    LINUX_NR_CPU_IDS.load(Ordering::Acquire)
}

/// `cpus_read_lock()` — `vendor/linux/kernel/cpu.c:488`.
#[unsafe(no_mangle)]
pub extern "C" fn cpus_read_lock() {
    while !CPU_HOTPLUG_LOCK.down_read_trylock() {
        core::hint::spin_loop();
    }
}

/// `cpus_read_unlock()` — `vendor/linux/kernel/cpu.c:499`.
#[unsafe(no_mangle)]
pub extern "C" fn cpus_read_unlock() {
    CPU_HOTPLUG_LOCK.up_read();
}

/// `__cpuhp_setup_state` - `vendor/linux/kernel/cpu.c:2527`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __cpuhp_setup_state(
    _state: i32,
    name: *const c_char,
    _invoke: bool,
    _startup: CpuHpCallback,
    _teardown: CpuHpCallback,
    _multi_instance: bool,
) -> i32 {
    if name.is_null() {
        -crate::include::uapi::errno::EINVAL
    } else {
        0
    }
}

/// `__cpuhp_setup_state_cpuslocked` - `vendor/linux/kernel/cpu.c:2468`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __cpuhp_setup_state_cpuslocked(
    state: i32,
    name: *const c_char,
    invoke: bool,
    startup: CpuHpCallback,
    teardown: CpuHpCallback,
    multi_instance: bool,
) -> i32 {
    unsafe { __cpuhp_setup_state(state, name, invoke, startup, teardown, multi_instance) }
}

/// `__cpuhp_state_add_instance` - `vendor/linux/kernel/cpu.c:2438`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __cpuhp_state_add_instance(
    _state: i32,
    _node: *mut c_void,
    _invoke: bool,
) -> i32 {
    0
}

/// `__cpuhp_state_add_instance_cpuslocked` - `vendor/linux/kernel/cpu.c:2393`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __cpuhp_state_add_instance_cpuslocked(
    state: i32,
    node: *mut c_void,
    invoke: bool,
) -> i32 {
    unsafe { __cpuhp_state_add_instance(state, node, invoke) }
}

/// `__cpuhp_state_remove_instance` - `vendor/linux/kernel/cpu.c:2543`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __cpuhp_state_remove_instance(
    _state: i32,
    _node: *mut c_void,
    _invoke: bool,
) -> i32 {
    0
}

/// `__cpuhp_remove_state` - `vendor/linux/kernel/cpu.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __cpuhp_remove_state(_state: i32, _invoke: bool) {}

/// `__cpuhp_remove_state_cpuslocked` - `vendor/linux/kernel/cpu.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __cpuhp_remove_state_cpuslocked(_state: i32, _invoke: bool) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpuhotplug_symbols_are_exported() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("__cpuhp_state_remove_instance"),
            Some(__cpuhp_state_remove_instance as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("__cpuhp_setup_state"),
            Some(__cpuhp_setup_state as usize)
        );
    }
}

//! linux-parity: partial
//! linux-source: vendor/linux/kernel/cpu.c
//! test-origin: linux:vendor/linux/kernel/cpu.c
//! Minimal CPU hotplug state exports for Linux-built modules.

use core::ffi::{c_char, c_void};

use crate::kernel::module::{export_symbol, find_symbol};

pub type CpuHpCallback = Option<unsafe extern "C" fn(cpu: u32) -> i32>;

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
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

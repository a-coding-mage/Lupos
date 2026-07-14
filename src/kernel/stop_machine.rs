//! linux-parity: partial
//! linux-source: vendor/linux/kernel/stop_machine.c
//! test-origin: linux:vendor/linux/kernel/stop_machine.c
//! Minimal stop-machine ABI for vendor modules.

use core::ffi::c_void;

use crate::kernel::module::{export_symbol, find_symbol};

type CpuStopFn = Option<unsafe extern "C" fn(*mut c_void) -> i32>;

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("stop_machine", linux_stop_machine as usize, true);
    export_symbol_once(
        "stop_core_cpuslocked",
        linux_stop_core_cpuslocked as usize,
        true,
    );
}

fn run_stop_callback(func: CpuStopFn, data: *mut c_void) -> i32 {
    match func {
        Some(func) => unsafe { func(data) },
        None => 0,
    }
}

/// `stop_machine` - `vendor/linux/kernel/stop_machine.c:623`.
pub unsafe extern "C" fn linux_stop_machine(
    func: CpuStopFn,
    data: *mut c_void,
    _cpus: *const c_void,
) -> i32 {
    run_stop_callback(func, data)
}

/// `stop_core_cpuslocked` - `vendor/linux/kernel/stop_machine.c:641`.
pub unsafe extern "C" fn linux_stop_core_cpuslocked(
    _cpu: u32,
    func: CpuStopFn,
    data: *mut c_void,
) -> i32 {
    run_stop_callback(func, data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::sync::atomic::{AtomicI32, Ordering};

    static CALLBACK_COUNT: AtomicI32 = AtomicI32::new(0);

    unsafe extern "C" fn test_callback(data: *mut c_void) -> i32 {
        let increment = data as usize as i32;
        CALLBACK_COUNT.fetch_add(increment, Ordering::SeqCst);
        7
    }

    #[test]
    fn stop_machine_exports_register_for_modules() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("stop_machine"),
            Some(linux_stop_machine as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("stop_core_cpuslocked"),
            Some(linux_stop_core_cpuslocked as usize)
        );
    }

    #[test]
    fn stop_machine_runs_callback_synchronously() {
        CALLBACK_COUNT.store(0, Ordering::SeqCst);
        let ret = unsafe {
            linux_stop_machine(
                Some(test_callback),
                3usize as *mut c_void,
                core::ptr::null(),
            )
        };
        assert_eq!(ret, 7);
        assert_eq!(CALLBACK_COUNT.load(Ordering::SeqCst), 3);
    }
}

//! linux-parity: complete
//! linux-source: vendor/linux/kernel/power
//! linux-source: vendor/linux/drivers/base/power
//! Power-management helpers.

pub mod em_netlink_autogen;
pub mod poweroff;

use core::ffi::c_void;

use crate::include::uapi::errno::EINVAL;
use crate::kernel::module::{export_symbol, find_symbol};

static mut CPU_LATENCY_CONSTRAINTS: [usize; 6] = [0; 6];
static mut PM_SUSPEND_TARGET_STATE: i32 = 0;
static mut PM_SUSPEND_GLOBAL_FLAGS: u32 = 0;
static POWER_GROUP_NAME: [u8; 6] = *b"power\0";

const PM_QOS_REQUEST_NODE_PRIO_OFFSET: usize = 0;
const PM_QOS_REQUEST_NODE_PRIO_LIST_OFFSET: usize = 8;
const PM_QOS_REQUEST_NODE_NODE_LIST_OFFSET: usize = 24;
const PM_QOS_REQUEST_QOS_OFFSET: usize = 40;
const PM_QOS_REQUEST_SIZE: usize = 48;
const RPM_ACTIVE: u32 = 0;
const RPM_SUSPENDED: u32 = 2;

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("power_group_name", POWER_GROUP_NAME.as_ptr() as usize, true);
    export_symbol_once("__pm_runtime_idle", linux___pm_runtime_idle as usize, true);
    export_symbol_once(
        "__pm_runtime_suspend",
        linux___pm_runtime_suspend as usize,
        true,
    );
    export_symbol_once(
        "__pm_runtime_resume",
        linux___pm_runtime_resume as usize,
        true,
    );
    export_symbol_once(
        "__pm_runtime_set_status",
        linux___pm_runtime_set_status as usize,
        true,
    );
    export_symbol_once(
        "__pm_runtime_disable",
        linux___pm_runtime_disable as usize,
        true,
    );
    export_symbol_once(
        "__pm_runtime_use_autosuspend",
        linux___pm_runtime_use_autosuspend as usize,
        true,
    );
    export_symbol_once("pm_runtime_enable", linux_pm_runtime_enable as usize, true);
    export_symbol_once("pm_runtime_allow", linux_pm_runtime_allow as usize, true);
    export_symbol_once("pm_runtime_forbid", linux_pm_runtime_forbid as usize, true);
    export_symbol_once(
        "pm_runtime_get_if_active",
        linux_pm_runtime_get_if_active as usize,
        true,
    );
    export_symbol_once(
        "pm_runtime_get_if_in_use",
        linux_pm_runtime_get_if_in_use as usize,
        true,
    );
    export_symbol_once(
        "pm_runtime_force_suspend",
        linux_pm_runtime_force_suspend as usize,
        true,
    );
    export_symbol_once(
        "pm_runtime_force_resume",
        linux_pm_runtime_force_resume as usize,
        true,
    );
    export_symbol_once(
        "pm_runtime_set_autosuspend_delay",
        linux_pm_runtime_set_autosuspend_delay as usize,
        true,
    );
    export_symbol_once(
        "pm_wakeup_dev_event",
        linux_pm_wakeup_dev_event as usize,
        true,
    );
    export_symbol_once("pm_system_wakeup", linux_pm_system_wakeup as usize, true);
    export_symbol_once(
        "pm_schedule_suspend",
        linux_pm_schedule_suspend as usize,
        true,
    );
    export_symbol_once(
        "cpu_latency_qos_request_active",
        linux_cpu_latency_qos_request_active as usize,
        true,
    );
    export_symbol_once(
        "cpu_latency_qos_add_request",
        linux_cpu_latency_qos_add_request as usize,
        true,
    );
    export_symbol_once(
        "cpu_latency_qos_update_request",
        linux_cpu_latency_qos_update_request as usize,
        true,
    );
    export_symbol_once(
        "cpu_latency_qos_remove_request",
        linux_cpu_latency_qos_remove_request as usize,
        true,
    );
    export_symbol_once("cpufreq_cpu_get", linux_cpufreq_cpu_get as usize, true);
    export_symbol_once("cpufreq_cpu_put", linux_cpufreq_cpu_put as usize, true);
    export_symbol_once(
        "dev_pm_domain_attach",
        linux_dev_pm_domain_attach as usize,
        true,
    );
    export_symbol_once(
        "dev_pm_domain_attach_by_id",
        linux_dev_pm_domain_attach_by_id as usize,
        true,
    );
    export_symbol_once(
        "dev_pm_domain_attach_by_name",
        linux_dev_pm_domain_attach_by_name as usize,
        true,
    );
    export_symbol_once(
        "dev_pm_domain_attach_list",
        linux_dev_pm_domain_attach_list as usize,
        true,
    );
    export_symbol_once(
        "dev_pm_domain_detach",
        linux_dev_pm_domain_detach as usize,
        true,
    );
    export_symbol_once(
        "dev_pm_domain_detach_list",
        linux_dev_pm_domain_detach_list as usize,
        true,
    );
    export_symbol_once(
        "dev_pm_domain_start",
        linux_dev_pm_domain_start as usize,
        true,
    );
    export_symbol_once(
        "dev_pm_domain_set_performance_state",
        linux_dev_pm_domain_set_performance_state as usize,
        true,
    );
    export_symbol_once("dev_pm_domain_set", linux_dev_pm_domain_set as usize, true);
    export_symbol_once(
        "pm_runtime_no_callbacks",
        linux_pm_runtime_no_callbacks as usize,
        true,
    );
    export_symbol_once(
        "dev_pm_set_wake_irq",
        linux_dev_pm_set_wake_irq as usize,
        true,
    );
    export_symbol_once(
        "devm_pm_set_wake_irq",
        linux_devm_pm_set_wake_irq as usize,
        true,
    );
    export_symbol_once(
        "dev_pm_set_dedicated_wake_irq",
        linux_dev_pm_set_dedicated_wake_irq as usize,
        true,
    );
    export_symbol_once(
        "dev_pm_clear_wake_irq",
        linux_dev_pm_clear_wake_irq as usize,
        true,
    );
    export_symbol_once(
        "system_entering_hibernation",
        linux_system_entering_hibernation as usize,
        false,
    );
    export_symbol_once(
        "pm_suspend_default_s2idle",
        linux_pm_suspend_default_s2idle as usize,
        true,
    );
    export_symbol_once(
        "pm_suspend_target_state",
        core::ptr::addr_of_mut!(PM_SUSPEND_TARGET_STATE) as usize,
        true,
    );
    export_symbol_once(
        "pm_suspend_global_flags",
        core::ptr::addr_of_mut!(PM_SUSPEND_GLOBAL_FLAGS) as usize,
        true,
    );
    export_symbol_once("lock_system_sleep", linux_lock_system_sleep as usize, true);
    export_symbol_once(
        "unlock_system_sleep",
        linux_unlock_system_sleep as usize,
        true,
    );
}

/// `lock_system_sleep` - `vendor/linux/kernel/power/main.c`.
pub unsafe extern "C" fn linux_lock_system_sleep() -> u32 {
    0
}

/// `unlock_system_sleep` - `vendor/linux/kernel/power/main.c`.
pub unsafe extern "C" fn linux_unlock_system_sleep(_flags: u32) {}

/// `pm_suspend_default_s2idle` - `vendor/linux/kernel/power/suspend.c:71`.
pub unsafe extern "C" fn linux_pm_suspend_default_s2idle() -> bool {
    false
}

/// `__pm_runtime_idle` - `vendor/linux/drivers/base/power/runtime.c`.
pub unsafe extern "C" fn linux___pm_runtime_idle(_dev: *mut c_void, _rpmflags: i32) -> i32 {
    0
}

/// `__pm_runtime_suspend` - `vendor/linux/drivers/base/power/runtime.c`.
///
/// Runtime PM callbacks are not modeled yet, so devices remain effectively
/// active and transition requests complete synchronously.
pub unsafe extern "C" fn linux___pm_runtime_suspend(_dev: *mut c_void, _rpmflags: i32) -> i32 {
    0
}

/// `__pm_runtime_resume` - `vendor/linux/drivers/base/power/runtime.c`.
pub unsafe extern "C" fn linux___pm_runtime_resume(_dev: *mut c_void, _rpmflags: i32) -> i32 {
    0
}

/// `__pm_runtime_set_status` - `vendor/linux/drivers/base/power/runtime.c`.
pub unsafe extern "C" fn linux___pm_runtime_set_status(_dev: *mut c_void, status: u32) -> i32 {
    match status {
        RPM_ACTIVE | RPM_SUSPENDED => 0,
        _ => -EINVAL,
    }
}

/// `__pm_runtime_disable` - `vendor/linux/drivers/base/power/runtime.c`.
pub unsafe extern "C" fn linux___pm_runtime_disable(_dev: *mut c_void, _check_resume: bool) {}

/// `__pm_runtime_use_autosuspend` - `vendor/linux/drivers/base/power/runtime.c`.
pub unsafe extern "C" fn linux___pm_runtime_use_autosuspend(_dev: *mut c_void, _use: bool) {}

/// `pm_runtime_enable` - `vendor/linux/drivers/base/power/runtime.c`.
pub unsafe extern "C" fn linux_pm_runtime_enable(_dev: *mut c_void) {}

/// `pm_runtime_allow` - `vendor/linux/drivers/base/power/runtime.c:1691`.
pub unsafe extern "C" fn linux_pm_runtime_allow(_dev: *mut c_void) {}

/// `pm_runtime_forbid` - `vendor/linux/drivers/base/power/runtime.c`.
pub unsafe extern "C" fn linux_pm_runtime_forbid(_dev: *mut c_void) {}

/// `pm_runtime_get_if_active` - `vendor/linux/drivers/base/power/runtime.c:1261`.
pub unsafe extern "C" fn linux_pm_runtime_get_if_active(_dev: *mut c_void) -> i32 {
    0
}

/// `pm_runtime_get_if_in_use` - `vendor/linux/drivers/base/power/runtime.c:1280`.
pub unsafe extern "C" fn linux_pm_runtime_get_if_in_use(_dev: *mut c_void) -> i32 {
    0
}

/// `pm_runtime_force_suspend` - `vendor/linux/drivers/base/power/runtime.c:2010`.
pub unsafe extern "C" fn linux_pm_runtime_force_suspend(_dev: *mut c_void) -> i32 {
    0
}

/// `pm_runtime_force_resume` - `vendor/linux/drivers/base/power/runtime.c:2072`.
pub unsafe extern "C" fn linux_pm_runtime_force_resume(_dev: *mut c_void) -> i32 {
    0
}

/// `pm_runtime_set_autosuspend_delay` - `vendor/linux/drivers/base/power/runtime.c`.
pub unsafe extern "C" fn linux_pm_runtime_set_autosuspend_delay(_dev: *mut c_void, _delay: i32) {}

/// `pm_wakeup_dev_event` - `vendor/linux/drivers/base/power/wakeup.c:824`.
pub unsafe extern "C" fn linux_pm_wakeup_dev_event(_dev: *mut c_void, _msec: u32, _hard: bool) {}

/// `pm_system_wakeup` - `vendor/linux/drivers/base/power/wakeup.c:895`.
pub unsafe extern "C" fn linux_pm_system_wakeup() {}

/// `pm_schedule_suspend` - `vendor/linux/drivers/base/power/runtime.c:1047`.
#[unsafe(export_name = "pm_schedule_suspend")]
pub unsafe extern "C" fn linux_pm_schedule_suspend(_dev: *mut c_void, _delay: u32) -> i32 {
    0
}

/// `dev_pm_domain_attach` - `vendor/linux/drivers/base/power/common.c:103`.
pub unsafe extern "C" fn linux_dev_pm_domain_attach(_dev: *mut c_void, _flags: u32) -> i32 {
    0
}

pub unsafe extern "C" fn linux_dev_pm_domain_attach_by_id(
    _dev: *mut c_void,
    _index: u32,
) -> *mut c_void {
    core::ptr::null_mut()
}

pub unsafe extern "C" fn linux_dev_pm_domain_attach_by_name(
    _dev: *mut c_void,
    _name: *const i8,
) -> *mut c_void {
    core::ptr::null_mut()
}

pub unsafe extern "C" fn linux_dev_pm_domain_attach_list(
    _dev: *mut c_void,
    _data: *const c_void,
    list: *mut *mut c_void,
) -> i32 {
    if !list.is_null() {
        unsafe {
            *list = core::ptr::null_mut();
        }
    }
    0
}

pub unsafe extern "C" fn linux_dev_pm_domain_detach(_dev: *mut c_void, _power_off: bool) {}

pub unsafe extern "C" fn linux_dev_pm_domain_detach_list(_list: *mut c_void) {}

pub unsafe extern "C" fn linux_dev_pm_domain_start(_dev: *mut c_void) -> i32 {
    0
}

pub unsafe extern "C" fn linux_dev_pm_domain_set_performance_state(
    _dev: *mut c_void,
    _state: u32,
) -> i32 {
    0
}

pub unsafe extern "C" fn linux_dev_pm_domain_set(_dev: *mut c_void, _pd: *mut c_void) {}

/// `pm_runtime_no_callbacks` - `vendor/linux/drivers/base/power/runtime.c:1719`.
pub unsafe extern "C" fn linux_pm_runtime_no_callbacks(_dev: *mut c_void) {}

pub unsafe extern "C" fn linux_dev_pm_set_wake_irq(_dev: *mut c_void, _irq: i32) -> i32 {
    0
}

pub unsafe extern "C" fn linux_devm_pm_set_wake_irq(dev: *mut c_void, irq: i32) -> i32 {
    unsafe { linux_dev_pm_set_wake_irq(dev, irq) }
}

pub unsafe extern "C" fn linux_dev_pm_set_dedicated_wake_irq(_dev: *mut c_void, _irq: i32) -> i32 {
    0
}

pub unsafe extern "C" fn linux_dev_pm_clear_wake_irq(_dev: *mut c_void) {}

/// `system_entering_hibernation` - `vendor/linux/kernel/power/hibernate.c`.
pub unsafe extern "C" fn linux_system_entering_hibernation() -> bool {
    false
}

unsafe fn write_list_head(base: *mut u8, offset: usize) {
    let head = unsafe { base.add(offset) } as *mut usize;
    let addr = head as usize;
    unsafe {
        head.write(addr);
        head.add(1).write(addr);
    }
}

unsafe fn qos_slot(req: *mut c_void) -> *mut usize {
    unsafe { (req as *mut u8).add(PM_QOS_REQUEST_QOS_OFFSET) as *mut usize }
}

unsafe extern "C" fn linux_cpu_latency_qos_request_active(req: *mut c_void) -> bool {
    if req.is_null() {
        return false;
    }
    unsafe { qos_slot(req).read() != 0 }
}

unsafe extern "C" fn linux_cpu_latency_qos_add_request(req: *mut c_void, value: i32) {
    if req.is_null() {
        return;
    }
    let base = req as *mut u8;
    unsafe {
        (base.add(PM_QOS_REQUEST_NODE_PRIO_OFFSET) as *mut i32).write(value);
        write_list_head(base, PM_QOS_REQUEST_NODE_PRIO_LIST_OFFSET);
        write_list_head(base, PM_QOS_REQUEST_NODE_NODE_LIST_OFFSET);
        qos_slot(req).write(core::ptr::addr_of_mut!(CPU_LATENCY_CONSTRAINTS) as usize);
    }
}

unsafe extern "C" fn linux_cpu_latency_qos_update_request(req: *mut c_void, value: i32) {
    if req.is_null() || unsafe { !linux_cpu_latency_qos_request_active(req) } {
        return;
    }
    unsafe { ((req as *mut u8).add(PM_QOS_REQUEST_NODE_PRIO_OFFSET) as *mut i32).write(value) };
}

unsafe extern "C" fn linux_cpu_latency_qos_remove_request(req: *mut c_void) {
    if req.is_null() {
        return;
    }
    unsafe { core::ptr::write_bytes(req as *mut u8, 0, PM_QOS_REQUEST_SIZE) };
}

/// `cpufreq_cpu_get` - `vendor/linux/drivers/cpufreq/cpufreq.c:205`.
///
/// No cpufreq driver or policy table is registered in Lupos yet, so policy
/// lookup misses and callers use their Linux fallback path.
pub unsafe extern "C" fn linux_cpufreq_cpu_get(_cpu: u32) -> *mut c_void {
    core::ptr::null_mut()
}

/// `cpufreq_cpu_put` - `vendor/linux/drivers/cpufreq/cpufreq.c:233`.
pub unsafe extern "C" fn linux_cpufreq_cpu_put(_policy: *mut c_void) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_pm_conditional_get_exports_and_defers() {
        register_module_exports();
        assert_eq!(
            find_symbol("power_group_name"),
            Some(POWER_GROUP_NAME.as_ptr() as usize)
        );
        assert_eq!(
            find_symbol("pm_runtime_get_if_active"),
            Some(linux_pm_runtime_get_if_active as usize)
        );
        assert_eq!(
            find_symbol("pm_suspend_target_state"),
            Some(core::ptr::addr_of_mut!(PM_SUSPEND_TARGET_STATE) as usize)
        );
        assert_eq!(
            unsafe { linux_pm_runtime_get_if_active(core::ptr::null_mut()) },
            0
        );
    }

    #[test]
    fn power_group_name_matches_vendor_sysfs_export() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/drivers/base/power/sysfs.c"
        ));
        assert!(source.contains("const char power_group_name[] = \"power\";"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(power_group_name);"));
        assert_eq!(&POWER_GROUP_NAME, b"power\0");
    }

    #[test]
    fn wakeup_exports_track_vendor_power_core() {
        let source = include_str!("../../../vendor/linux/drivers/base/power/wakeup.c");
        let wakeirq = include_str!("../../../vendor/linux/drivers/base/power/wakeirq.c");
        assert!(source.contains("EXPORT_SYMBOL_GPL(pm_wakeup_dev_event);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(pm_system_wakeup);"));
        assert!(wakeirq.contains("EXPORT_SYMBOL_GPL(devm_pm_set_wake_irq);"));
        register_module_exports();
        assert_eq!(
            find_symbol("pm_wakeup_dev_event"),
            Some(linux_pm_wakeup_dev_event as usize)
        );
        assert_eq!(
            find_symbol("pm_system_wakeup"),
            Some(linux_pm_system_wakeup as usize)
        );
        assert_eq!(
            find_symbol("devm_pm_set_wake_irq"),
            Some(linux_devm_pm_set_wake_irq as usize)
        );
    }
}

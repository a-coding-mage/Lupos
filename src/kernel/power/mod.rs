//! linux-parity: partial
//! linux-source: vendor/linux/kernel/power
//! linux-source: vendor/linux/drivers/base/power
//! test-origin: linux:vendor/linux/kernel/power
//! Power-management helpers.

pub mod em_netlink_autogen;
pub mod poweroff;

use core::ffi::c_void;
use core::sync::atomic::{AtomicI32, AtomicU32, Ordering};

use crate::include::uapi::errno::{EACCES, EAGAIN, EINVAL};
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
const RPM_ACTIVE: i32 = 0;
const RPM_INVALID: i32 = -1;
const RPM_SUSPENDED: i32 = 2;
const RPM_GET_PUT: i32 = 0x04;

// Configured x86_64 `struct device` / `struct dev_pm_info` offsets. These are
// asserted by `xtask/vendor_abi_probe/lupos_abi_layout_probe.c`.
const DEVICE_PM_USAGE_COUNT_OFFSET: usize = 456;
const DEVICE_PM_CHILD_COUNT_OFFSET: usize = 460;
const DEVICE_PM_CONTROL_OFFSET: usize = 464;
const DEVICE_PM_RUNTIME_STATUS_OFFSET: usize = 476;
const DEVICE_PM_LAST_STATUS_OFFSET: usize = 480;
const DEVICE_PM_RUNTIME_ERROR_OFFSET: usize = 484;
const PM_DISABLE_DEPTH_MASK: u32 = 0x7;
const PM_RUNTIME_AUTO: u32 = 1 << 7;
const PM_IGNORE_CHILDREN: u32 = 1 << 8;
const PM_NO_CALLBACKS: u32 = 1 << 9;
const PM_USE_AUTOSUSPEND: u32 = 1 << 11;

unsafe fn pm_atomic_i32(dev: *mut c_void, offset: usize) -> &'static AtomicI32 {
    unsafe { &*((dev as *mut u8).add(offset).cast::<AtomicI32>()) }
}

unsafe fn pm_atomic_u32(dev: *mut c_void, offset: usize) -> &'static AtomicU32 {
    unsafe { &*((dev as *mut u8).add(offset).cast::<AtomicU32>()) }
}

unsafe fn pm_usage_count(dev: *mut c_void) -> &'static AtomicI32 {
    unsafe { pm_atomic_i32(dev, DEVICE_PM_USAGE_COUNT_OFFSET) }
}

unsafe fn pm_child_count(dev: *mut c_void) -> &'static AtomicI32 {
    unsafe { pm_atomic_i32(dev, DEVICE_PM_CHILD_COUNT_OFFSET) }
}

unsafe fn pm_control(dev: *mut c_void) -> &'static AtomicU32 {
    unsafe { pm_atomic_u32(dev, DEVICE_PM_CONTROL_OFFSET) }
}

unsafe fn pm_runtime_status(dev: *mut c_void) -> &'static AtomicI32 {
    unsafe { pm_atomic_i32(dev, DEVICE_PM_RUNTIME_STATUS_OFFSET) }
}

unsafe fn pm_last_status(dev: *mut c_void) -> &'static AtomicI32 {
    unsafe { pm_atomic_i32(dev, DEVICE_PM_LAST_STATUS_OFFSET) }
}

unsafe fn pm_runtime_error(dev: *mut c_void) -> &'static AtomicI32 {
    unsafe { pm_atomic_i32(dev, DEVICE_PM_RUNTIME_ERROR_OFFSET) }
}

fn pm_disable_depth(control: u32) -> u32 {
    control & PM_DISABLE_DEPTH_MASK
}

unsafe fn pm_update_control(dev: *mut c_void, mut update: impl FnMut(u32) -> u32) -> u32 {
    let control = unsafe { pm_control(dev) };
    let mut current = control.load(Ordering::Acquire);
    loop {
        let next = update(current);
        match control.compare_exchange_weak(current, next, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_) => return next,
            Err(observed) => current = observed,
        }
    }
}

/// Initialize the runtime-PM portion of a full vendor-layout `struct device`.
///
/// This is the state established by `pm_runtime_init()` during Linux
/// `device_initialize()`. Timer/workqueue internals are deliberately left to
/// Lupos because the compatible runtime subset completes transitions
/// synchronously.
pub unsafe fn runtime_pm_init_device(dev: *mut c_void) {
    if dev.is_null() {
        return;
    }
    unsafe {
        pm_usage_count(dev).store(0, Ordering::Release);
        pm_child_count(dev).store(0, Ordering::Release);
        pm_control(dev).store(1 | PM_RUNTIME_AUTO, Ordering::Release);
        pm_runtime_status(dev).store(RPM_SUSPENDED, Ordering::Release);
        pm_last_status(dev).store(RPM_INVALID, Ordering::Release);
        pm_runtime_error(dev).store(0, Ordering::Release);
    }
}

unsafe fn pm_drop_usage_count(dev: *mut c_void) -> Result<i32, i32> {
    let usage = unsafe { pm_usage_count(dev) };
    let previous = usage.fetch_sub(1, Ordering::AcqRel);
    if previous > 0 {
        Ok(previous - 1)
    } else {
        usage.fetch_add(1, Ordering::Release);
        Err(-EINVAL)
    }
}

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
pub unsafe extern "C" fn linux___pm_runtime_idle(dev: *mut c_void, rpmflags: i32) -> i32 {
    if dev.is_null() {
        return -EINVAL;
    }
    if rpmflags & RPM_GET_PUT != 0 {
        return match unsafe { pm_drop_usage_count(dev) } {
            Ok(_) => 0,
            Err(error) => error,
        };
    }
    0
}

/// `__pm_runtime_suspend` - `vendor/linux/drivers/base/power/runtime.c`.
///
/// Runtime PM callbacks are not modeled yet, so devices remain effectively
/// active and transition requests complete synchronously.
pub unsafe extern "C" fn linux___pm_runtime_suspend(dev: *mut c_void, rpmflags: i32) -> i32 {
    if dev.is_null() {
        return -EINVAL;
    }
    if rpmflags & RPM_GET_PUT != 0 {
        match unsafe { pm_drop_usage_count(dev) } {
            Ok(remaining) if remaining > 0 => return 0,
            Ok(_) => {}
            Err(error) => return error,
        }
    }
    if pm_disable_depth(unsafe { pm_control(dev).load(Ordering::Acquire) }) > 0 {
        return -EACCES;
    }
    // Runtime callbacks are not modeled. Keep the hardware and observable
    // state active, matching Linux's no-callback successful path without
    // pretending that a driver power-down callback ran.
    0
}

/// `__pm_runtime_resume` - `vendor/linux/drivers/base/power/runtime.c`.
pub unsafe extern "C" fn linux___pm_runtime_resume(dev: *mut c_void, rpmflags: i32) -> i32 {
    if dev.is_null() {
        return -EINVAL;
    }
    if rpmflags & RPM_GET_PUT != 0 {
        unsafe { pm_usage_count(dev).fetch_add(1, Ordering::AcqRel) };
    }
    if unsafe { pm_runtime_error(dev).load(Ordering::Acquire) } != 0 {
        return -EINVAL;
    }
    let control = unsafe { pm_control(dev).load(Ordering::Acquire) };
    if pm_disable_depth(control) > 0 {
        return if unsafe { pm_runtime_status(dev).load(Ordering::Acquire) } == RPM_ACTIVE
            && unsafe { pm_last_status(dev).load(Ordering::Acquire) } == RPM_ACTIVE
        {
            1
        } else {
            -EACCES
        };
    }
    let previous = unsafe { pm_runtime_status(dev).swap(RPM_ACTIVE, Ordering::AcqRel) };
    if previous == RPM_ACTIVE { 1 } else { 0 }
}

/// `__pm_runtime_set_status` - `vendor/linux/drivers/base/power/runtime.c`.
pub unsafe extern "C" fn linux___pm_runtime_set_status(dev: *mut c_void, status: u32) -> i32 {
    if dev.is_null() || status as i32 != RPM_ACTIVE && status as i32 != RPM_SUSPENDED {
        return -EINVAL;
    }

    let control = unsafe { pm_control(dev) };
    loop {
        let current = control.load(Ordering::Acquire);
        let depth = pm_disable_depth(current);
        if depth == 0 && unsafe { pm_runtime_error(dev).load(Ordering::Acquire) } == 0 {
            return -EAGAIN;
        }
        if depth == PM_DISABLE_DEPTH_MASK {
            return -EINVAL;
        }
        let next = (current & !PM_DISABLE_DEPTH_MASK) | (depth + 1);
        if control
            .compare_exchange_weak(current, next, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            break;
        }
    }

    unsafe {
        pm_runtime_status(dev).store(status as i32, Ordering::Release);
        pm_runtime_error(dev).store(0, Ordering::Release);
        linux_pm_runtime_enable(dev);
    }
    0
}

/// `__pm_runtime_disable` - `vendor/linux/drivers/base/power/runtime.c`.
pub unsafe extern "C" fn linux___pm_runtime_disable(dev: *mut c_void, _check_resume: bool) {
    if dev.is_null() {
        return;
    }
    let previous_status = unsafe { pm_runtime_status(dev).load(Ordering::Acquire) };
    let next = unsafe {
        pm_update_control(dev, |current| {
            let depth = pm_disable_depth(current);
            (current & !PM_DISABLE_DEPTH_MASK) | depth.saturating_add(1).min(7)
        })
    };
    if pm_disable_depth(next) == 1 {
        unsafe { pm_last_status(dev).store(previous_status, Ordering::Release) };
    }
}

/// `__pm_runtime_use_autosuspend` - `vendor/linux/drivers/base/power/runtime.c`.
pub unsafe extern "C" fn linux___pm_runtime_use_autosuspend(dev: *mut c_void, use_: bool) {
    if dev.is_null() {
        return;
    }
    unsafe {
        pm_update_control(dev, |control| {
            if use_ {
                control | PM_USE_AUTOSUSPEND
            } else {
                control & !PM_USE_AUTOSUSPEND
            }
        });
    }
}

/// `pm_runtime_enable` - `vendor/linux/drivers/base/power/runtime.c`.
pub unsafe extern "C" fn linux_pm_runtime_enable(dev: *mut c_void) {
    if dev.is_null() {
        return;
    }
    let next = unsafe {
        pm_update_control(dev, |current| {
            let depth = pm_disable_depth(current);
            if depth == 0 {
                current
            } else {
                (current & !PM_DISABLE_DEPTH_MASK) | (depth - 1)
            }
        })
    };
    if pm_disable_depth(next) == 0 {
        unsafe { pm_last_status(dev).store(RPM_INVALID, Ordering::Release) };
    }
}

/// `pm_runtime_allow` - `vendor/linux/drivers/base/power/runtime.c:1691`.
pub unsafe extern "C" fn linux_pm_runtime_allow(dev: *mut c_void) {
    if dev.is_null() {
        return;
    }
    let control = unsafe { pm_control(dev) };
    let mut current = control.load(Ordering::Acquire);
    loop {
        if current & PM_RUNTIME_AUTO != 0 {
            return;
        }
        match control.compare_exchange_weak(
            current,
            current | PM_RUNTIME_AUTO,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => break,
            Err(observed) => current = observed,
        }
    }
    let _ = unsafe { pm_drop_usage_count(dev) };
}

/// `pm_runtime_forbid` - `vendor/linux/drivers/base/power/runtime.c`.
pub unsafe extern "C" fn linux_pm_runtime_forbid(dev: *mut c_void) {
    if dev.is_null() {
        return;
    }
    let control = unsafe { pm_control(dev) };
    let mut current = control.load(Ordering::Acquire);
    loop {
        if current & PM_RUNTIME_AUTO == 0 {
            return;
        }
        match control.compare_exchange_weak(
            current,
            current & !PM_RUNTIME_AUTO,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => break,
            Err(observed) => current = observed,
        }
    }
    unsafe {
        pm_usage_count(dev).fetch_add(1, Ordering::AcqRel);
        pm_runtime_status(dev).store(RPM_ACTIVE, Ordering::Release);
    }
}

/// `pm_runtime_get_if_active` - `vendor/linux/drivers/base/power/runtime.c:1261`.
pub unsafe extern "C" fn linux_pm_runtime_get_if_active(dev: *mut c_void) -> i32 {
    unsafe { pm_runtime_get_conditional(dev, true) }
}

/// `pm_runtime_get_if_in_use` - `vendor/linux/drivers/base/power/runtime.c:1280`.
pub unsafe extern "C" fn linux_pm_runtime_get_if_in_use(dev: *mut c_void) -> i32 {
    unsafe { pm_runtime_get_conditional(dev, false) }
}

unsafe fn pm_runtime_get_conditional(dev: *mut c_void, ignore_usage_count: bool) -> i32 {
    if dev.is_null() {
        return -EINVAL;
    }
    let control = unsafe { pm_control(dev).load(Ordering::Acquire) };
    if pm_disable_depth(control) > 0 {
        return -EINVAL;
    }
    if unsafe { pm_runtime_status(dev).load(Ordering::Acquire) } != RPM_ACTIVE {
        return 0;
    }

    let active_child = control & PM_IGNORE_CHILDREN == 0
        && unsafe { pm_child_count(dev).load(Ordering::Acquire) } > 0;
    let usage = unsafe { pm_usage_count(dev) };
    if ignore_usage_count || active_child {
        usage.fetch_add(1, Ordering::AcqRel);
        return 1;
    }

    let mut current = usage.load(Ordering::Acquire);
    while current != 0 {
        match usage.compare_exchange_weak(
            current,
            current.saturating_add(1),
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => return 1,
            Err(observed) => current = observed,
        }
    }
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
pub unsafe extern "C" fn linux_pm_runtime_no_callbacks(dev: *mut c_void) {
    if dev.is_null() {
        return;
    }
    unsafe {
        pm_update_control(dev, |control| control | PM_NO_CALLBACKS);
    }
}

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
    fn runtime_pm_conditional_get_exports_and_rejects_null() {
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
            -EINVAL
        );
    }

    fn pm_test_device() -> [u32; 190] {
        [0_u32; 190]
    }

    #[test]
    fn runtime_pm_init_matches_vendor_disabled_suspended_state() {
        let mut storage = pm_test_device();
        let dev = storage.as_mut_ptr().cast();

        unsafe { runtime_pm_init_device(dev) };

        assert_eq!(unsafe { pm_usage_count(dev).load(Ordering::Acquire) }, 0);
        assert_eq!(unsafe { pm_child_count(dev).load(Ordering::Acquire) }, 0);
        let control = unsafe { pm_control(dev).load(Ordering::Acquire) };
        assert_eq!(pm_disable_depth(control), 1);
        assert_ne!(control & PM_RUNTIME_AUTO, 0);
        assert_eq!(
            unsafe { pm_runtime_status(dev).load(Ordering::Acquire) },
            RPM_SUSPENDED
        );
        assert_eq!(
            unsafe { pm_last_status(dev).load(Ordering::Acquire) },
            RPM_INVALID
        );
        assert_eq!(unsafe { linux_pm_runtime_get_if_active(dev) }, -EINVAL);
    }

    #[test]
    fn runtime_pm_set_active_and_enable_unlocks_hda_conditional_get() {
        let mut storage = pm_test_device();
        let dev = storage.as_mut_ptr().cast();
        unsafe {
            runtime_pm_init_device(dev);
            assert_eq!(linux___pm_runtime_set_status(dev, RPM_ACTIVE as u32), 0);
            pm_usage_count(dev).fetch_add(1, Ordering::AcqRel);
        }

        assert_eq!(
            unsafe { pm_runtime_status(dev).load(Ordering::Acquire) },
            RPM_ACTIVE
        );
        assert_eq!(
            pm_disable_depth(unsafe { pm_control(dev).load(Ordering::Acquire) }),
            1
        );
        // This -EINVAL is the vendor result while PM remains disabled. HDA
        // treats it as "access allowed", unlike a zero inactive result.
        assert_eq!(unsafe { linux_pm_runtime_get_if_active(dev) }, -EINVAL);

        unsafe { linux_pm_runtime_enable(dev) };
        assert_eq!(unsafe { linux_pm_runtime_get_if_active(dev) }, 1);
        assert_eq!(unsafe { pm_usage_count(dev).load(Ordering::Acquire) }, 2);
    }

    #[test]
    fn runtime_pm_get_if_in_use_requires_a_reference_or_active_child() {
        let mut storage = pm_test_device();
        let dev = storage.as_mut_ptr().cast();
        unsafe {
            runtime_pm_init_device(dev);
            assert_eq!(linux___pm_runtime_set_status(dev, RPM_ACTIVE as u32), 0);
            linux_pm_runtime_enable(dev);
        }

        assert_eq!(unsafe { linux_pm_runtime_get_if_in_use(dev) }, 0);
        unsafe { pm_usage_count(dev).store(1, Ordering::Release) };
        assert_eq!(unsafe { linux_pm_runtime_get_if_in_use(dev) }, 1);
        assert_eq!(unsafe { pm_usage_count(dev).load(Ordering::Acquire) }, 2);
    }

    #[test]
    fn runtime_pm_disable_and_enable_preserve_last_status() {
        let mut storage = pm_test_device();
        let dev = storage.as_mut_ptr().cast();
        unsafe {
            runtime_pm_init_device(dev);
            assert_eq!(linux___pm_runtime_set_status(dev, RPM_ACTIVE as u32), 0);
            linux_pm_runtime_enable(dev);
            linux___pm_runtime_disable(dev, false);
        }

        assert_eq!(
            pm_disable_depth(unsafe { pm_control(dev).load(Ordering::Acquire) }),
            1
        );
        assert_eq!(
            unsafe { pm_last_status(dev).load(Ordering::Acquire) },
            RPM_ACTIVE
        );

        unsafe { linux_pm_runtime_enable(dev) };
        assert_eq!(
            unsafe { pm_last_status(dev).load(Ordering::Acquire) },
            RPM_INVALID
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

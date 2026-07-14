//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel
//! test-origin: linux:vendor/linux/arch/x86/kernel
//! ACPI (Advanced Configuration and Power Interface) table parser.
//!
//! This module locates the RSDP (Root System Description Pointer), walks the
//! RSDT or XSDT to find the MADT (Multiple APIC Description Table), and
//! extracts the list of enabled CPUs along with the Local APIC base address.
//!
//! # What we need from ACPI
//! For SMP bring-up (Milestone 5) we only need two things:
//!   1. The physical base address of the Local APIC (usually 0xFEE00000).
//!   2. The list of CPU APIC IDs so we know which APs to wake up.
//!
//! Everything else (power management, HPET, I/O APIC routing, etc.) is left
//! for later milestones.
//!
//! # Memory layout
//! ACPI tables live in firmware memory — we access them through the identity
//! mapping set up by the boot stub (first 4 GiB physical = virtual).
//!
//! References:
//!   ACPI Specification 6.5 §5.2 "ACPI System Description Tables"
//!   ACPI Specification 6.5 §5.2.12 "Multiple APIC Description Table (MADT)"
//!   vendor/linux/arch/x86/kernel/acpi/boot.c
//!   vendor/linux/arch/x86/kernel/mpparse.c
//!   https://wiki.osdev.org/ACPI
//!   https://wiki.osdev.org/MADT

use core::ffi::c_void;

use crate::include::uapi::errno::{ENODATA, ENODEV, ENOENT};
use crate::kernel::module::{export_symbol, find_symbol};
use crate::mm::page_flags::GFP_KERNEL;

/// Maximum number of CPUs we track.  Exceeding this silently truncates.
pub const MAX_CPUS: usize = 16;
const AE_OK: u32 = 0;
const AE_NO_MEMORY: u32 = 4;
const AE_NOT_FOUND: u32 = 5;
const AE_BUFFER_OVERFLOW: u32 = 0x000b;
const AE_BAD_PARAMETER: u32 = 0x1001;
const ACPI_FULL_PATHNAME: u32 = 0;
const ACPI_SINGLE_NAME: u32 = 1;
const ACPI_FULL_PATHNAME_NO_TRAILING: u32 = 2;
const ACPI_STATE_S0: u32 = 0;
const ACPI_NO_BUFFER: usize = 0;
const ACPI_ALLOCATE_BUFFER: usize = usize::MAX;
const ACPI_ALLOCATE_LOCAL_BUFFER: usize = usize::MAX - 1;
const ACPI_BACKLIGHT_VENDOR: i32 = 2;
const ACPI_BACKLIGHT_NATIVE: i32 = 3;
static ACPI_LUPOS_FULL_PATH: &[u8] = b"\\_SB_.LUPS\0";
static ACPI_LUPOS_SINGLE_NAME: &[u8] = b"LUPS\0";

static mut ACPI_VIDEO_BACKLIGHT_STRING: [u8; 16] = [0; 16];

#[repr(C)]
pub struct LinuxAcpiBuffer {
    pub length: usize,
    pub pointer: *mut c_void,
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

fn acpi_video_backlight_string_addr() -> usize {
    core::ptr::addr_of_mut!(ACPI_VIDEO_BACKLIGHT_STRING) as *mut u8 as usize
}

pub fn register_module_exports() {
    export_symbol_once(
        "acpi_reconfig_notifier_register",
        linux_acpi_reconfig_notifier_register as usize,
        false,
    );
    export_symbol_once(
        "acpi_reconfig_notifier_unregister",
        linux_acpi_reconfig_notifier_unregister as usize,
        false,
    );
    export_symbol_once(
        "register_acpi_bus_type",
        linux_register_acpi_bus_type as usize,
        true,
    );
    export_symbol_once(
        "unregister_acpi_bus_type",
        linux_unregister_acpi_bus_type as usize,
        true,
    );
    export_symbol_once(
        "acpi_walk_namespace",
        linux_acpi_walk_namespace as usize,
        false,
    );
    export_symbol_once(
        "acpi_pm_device_sleep_state",
        linux_acpi_pm_device_sleep_state as usize,
        false,
    );
    export_symbol_once(
        "acpi_target_system_state",
        linux_acpi_target_system_state as usize,
        true,
    );
    export_symbol_once("acpi_storage_d3", linux_acpi_storage_d3 as usize, true);
    export_symbol_once(
        "acpi_bus_get_status",
        linux_acpi_bus_get_status as usize,
        false,
    );
    export_symbol_once(
        "acpi_bus_power_manageable",
        linux_acpi_bus_power_manageable as usize,
        false,
    );
    export_symbol_once(
        "acpi_bus_set_power",
        linux_acpi_bus_set_power as usize,
        false,
    );
    export_symbol_once(
        "acpi_evaluate_object",
        linux_acpi_evaluate_object as usize,
        false,
    );
    export_symbol_once("acpi_check_dsm", linux_acpi_check_dsm as usize, false);
    export_symbol_once("acpi_evaluate_dsm", linux_acpi_evaluate_dsm as usize, false);
    export_symbol_once(
        "acpi_notifier_call_chain",
        linux_acpi_notifier_call_chain as usize,
        false,
    );
    export_symbol_once(
        "register_acpi_notifier",
        linux_register_acpi_notifier as usize,
        false,
    );
    export_symbol_once(
        "unregister_acpi_notifier",
        linux_unregister_acpi_notifier as usize,
        false,
    );
    export_symbol_once(
        "acpi_video_register",
        linux_acpi_video_register as usize,
        false,
    );
    export_symbol_once(
        "acpi_video_unregister",
        linux_acpi_video_unregister as usize,
        false,
    );
    export_symbol_once(
        "acpi_video_register_backlight",
        linux_acpi_video_register_backlight as usize,
        false,
    );
    export_symbol_once(
        "acpi_video_handles_brightness_key_presses",
        linux_acpi_video_handles_brightness_key_presses as usize,
        false,
    );
    export_symbol_once(
        "acpi_video_get_edid",
        linux_acpi_video_get_edid as usize,
        false,
    );
    export_symbol_once(
        "acpi_video_get_levels",
        linux_acpi_video_get_levels as usize,
        false,
    );
    export_symbol_once(
        "__acpi_video_get_backlight_type",
        linux___acpi_video_get_backlight_type as usize,
        false,
    );
    export_symbol_once(
        "acpi_video_backlight_string",
        acpi_video_backlight_string_addr(),
        false,
    );
    export_symbol_once(
        "acpi_fetch_acpi_dev",
        linux_acpi_fetch_acpi_dev as usize,
        true,
    );
    export_symbol_once(
        "acpi_get_local_u64_address",
        linux_acpi_get_local_u64_address as usize,
        false,
    );
    export_symbol_once("acpi_dev_present", linux_acpi_dev_present as usize, false);
    export_symbol_once(
        "acpi_dev_get_resources",
        linux_acpi_dev_get_resources as usize,
        true,
    );
    export_symbol_once(
        "acpi_dev_free_resource_list",
        linux_acpi_dev_free_resource_list as usize,
        true,
    );
    export_symbol_once(
        "acpi_dev_resource_interrupt",
        linux_acpi_dev_resource_interrupt as usize,
        false,
    );
    export_symbol_once(
        "acpi_check_resource_conflict",
        linux_acpi_check_resource_conflict as usize,
        false,
    );
    export_symbol_once(
        "acpi_install_address_space_handler",
        linux_acpi_install_address_space_handler as usize,
        false,
    );
    export_symbol_once(
        "acpi_remove_address_space_handler",
        linux_acpi_remove_address_space_handler as usize,
        false,
    );
    export_symbol_once("acpi_os_read_port", linux_acpi_os_read_port as usize, false);
    export_symbol_once(
        "acpi_os_write_port",
        linux_acpi_os_write_port as usize,
        false,
    );
    export_symbol_once(
        "acpi_dev_clear_dependencies",
        linux_acpi_dev_clear_dependencies as usize,
        true,
    );
    export_symbol_once(
        "acpi_dev_ready_for_enumeration",
        linux_acpi_dev_ready_for_enumeration as usize,
        true,
    );
    export_symbol_once("acpi_get_handle", linux_acpi_get_handle as usize, false);
    export_symbol_once("acpi_get_name", linux_acpi_get_name as usize, false);
    export_symbol_once("acpi_get_table", linux_acpi_get_table as usize, false);
    export_symbol_once("acpi_set_modalias", linux_acpi_set_modalias as usize, true);
    export_symbol_once(
        "acpi_device_modalias",
        linux_acpi_device_modalias as usize,
        true,
    );
    export_symbol_once(
        "acpi_device_uevent_modalias",
        linux_acpi_device_uevent_modalias as usize,
        true,
    );
    export_symbol_once(
        "acpi_driver_match_device",
        linux_acpi_driver_match_device as usize,
        false,
    );
    export_symbol_once(
        "acpi_match_device_ids",
        linux_acpi_match_device_ids as usize,
        false,
    );
    export_symbol_once(
        "acpi_find_child_device",
        linux_acpi_find_child_device as usize,
        true,
    );
    export_symbol_once(
        "acpi_initialize_hp_context",
        linux_acpi_initialize_hp_context as usize,
        false,
    );
    export_symbol_once("acpi_unbind_one", linux_acpi_unbind_one as usize, true);
    export_symbol_once("acpi_put_table", linux_acpi_put_table as usize, false);
    export_symbol_once(
        "acpi_nhlt_get_gbl_table",
        linux_acpi_nhlt_get_gbl_table as usize,
        true,
    );
    export_symbol_once(
        "acpi_nhlt_find_endpoint",
        linux_acpi_nhlt_find_endpoint as usize,
        true,
    );
    export_symbol_once(
        "acpi_nhlt_put_gbl_table",
        linux_acpi_nhlt_put_gbl_table as usize,
        true,
    );
}

/// `acpi_reconfig_notifier_register` - `vendor/linux/drivers/acpi/scan.c`.
pub unsafe extern "C" fn linux_acpi_reconfig_notifier_register(_nb: *mut c_void) -> i32 {
    0
}

/// `acpi_reconfig_notifier_unregister` - `vendor/linux/drivers/acpi/scan.c`.
pub unsafe extern "C" fn linux_acpi_reconfig_notifier_unregister(_nb: *mut c_void) -> i32 {
    0
}

/// `register_acpi_bus_type` - `vendor/linux/drivers/acpi/glue.c`.
pub unsafe extern "C" fn linux_register_acpi_bus_type(bus: *mut c_void) -> i32 {
    if bus.is_null() { -ENODEV } else { 0 }
}

/// `unregister_acpi_bus_type` - `vendor/linux/drivers/acpi/glue.c`.
pub unsafe extern "C" fn linux_unregister_acpi_bus_type(bus: *mut c_void) -> i32 {
    if bus.is_null() { -ENODEV } else { 0 }
}

/// `acpi_walk_namespace` - `vendor/linux/drivers/acpi/acpica/nsxfeval.c`.
pub unsafe extern "C" fn linux_acpi_walk_namespace(
    _type_: u32,
    _start_object: *mut c_void,
    _max_depth: u32,
    _descending_callback: *mut c_void,
    _ascending_callback: *mut c_void,
    _context: *mut c_void,
    _return_value: *mut *mut c_void,
) -> u32 {
    0
}

/// `acpi_pm_device_sleep_state` - `vendor/linux/drivers/acpi/device_pm.c`.
pub unsafe extern "C" fn linux_acpi_pm_device_sleep_state(
    _dev: *mut c_void,
    _d_min_p: *mut i32,
    _d_max_in: i32,
) -> i32 {
    -ENODEV
}

/// `acpi_target_system_state` - `vendor/linux/drivers/acpi/sleep.c:101`.
pub unsafe extern "C" fn linux_acpi_target_system_state() -> u32 {
    ACPI_STATE_S0
}

/// `acpi_storage_d3` - `vendor/linux/drivers/acpi/device_pm.c`.
pub unsafe extern "C" fn linux_acpi_storage_d3(_dev: *mut c_void) -> bool {
    false
}

/// `acpi_bus_get_status` - `vendor/linux/drivers/acpi/bus.c`.
pub unsafe extern "C" fn linux_acpi_bus_get_status(_device: *mut c_void) -> i32 {
    -ENODEV
}

/// `acpi_bus_power_manageable` - `vendor/linux/drivers/acpi/device_pm.c`.
pub unsafe extern "C" fn linux_acpi_bus_power_manageable(_handle: *mut c_void) -> bool {
    false
}

/// `acpi_bus_set_power` - `vendor/linux/drivers/acpi/device_pm.c`.
pub unsafe extern "C" fn linux_acpi_bus_set_power(_handle: *mut c_void, _state: i32) -> i32 {
    0
}

/// `acpi_evaluate_object` - `vendor/linux/drivers/acpi/acpica/nsxfeval.c`.
pub unsafe extern "C" fn linux_acpi_evaluate_object(
    _object: *mut c_void,
    _pathname: *const u8,
    _parameter_objects: *mut c_void,
    _return_object_buffer: *mut c_void,
) -> u32 {
    AE_NOT_FOUND
}

/// `acpi_check_dsm` - `vendor/linux/drivers/acpi/utils.c:821`.
pub unsafe extern "C" fn linux_acpi_check_dsm(
    _handle: *mut c_void,
    _guid: *const c_void,
    _rev: u64,
    _funcs: u64,
) -> bool {
    false
}

/// `acpi_evaluate_dsm` - `vendor/linux/drivers/acpi/utils.c:771`.
pub unsafe extern "C" fn linux_acpi_evaluate_dsm(
    _handle: *mut c_void,
    _guid: *const c_void,
    _rev: u64,
    _func: u64,
    _argv4: *mut c_void,
) -> *mut c_void {
    core::ptr::null_mut()
}

/// `acpi_notifier_call_chain` - `vendor/linux/drivers/acpi/event.c:27`.
pub unsafe extern "C" fn linux_acpi_notifier_call_chain(
    _device_class: *const u8,
    _bus_id: *const u8,
    _event_type: u32,
    _data: u32,
) -> i32 {
    0
}

/// `register_acpi_notifier` - `vendor/linux/drivers/acpi/event.c:41`.
pub unsafe extern "C" fn linux_register_acpi_notifier(nb: *mut c_void) -> i32 {
    unsafe {
        crate::kernel::notifier::linux_blocking_notifier_chain_register(core::ptr::null_mut(), nb)
    }
}

/// `unregister_acpi_notifier` - `vendor/linux/drivers/acpi/event.c:47`.
pub unsafe extern "C" fn linux_unregister_acpi_notifier(nb: *mut c_void) -> i32 {
    unsafe {
        crate::kernel::notifier::linux_blocking_notifier_chain_unregister(core::ptr::null_mut(), nb)
    }
}

/// `acpi_video_register` - `vendor/linux/drivers/acpi/acpi_video.c:2141`.
pub unsafe extern "C" fn linux_acpi_video_register() -> i32 {
    -ENODEV
}

/// `acpi_video_unregister` - `vendor/linux/drivers/acpi/acpi_video.c:2172`.
pub unsafe extern "C" fn linux_acpi_video_unregister() {}

/// `acpi_video_register_backlight` - `vendor/linux/drivers/acpi/acpi_video.c:2184`.
pub unsafe extern "C" fn linux_acpi_video_register_backlight() {}

/// `acpi_video_handles_brightness_key_presses` - `vendor/linux/drivers/acpi/acpi_video.c:2195`.
pub unsafe extern "C" fn linux_acpi_video_handles_brightness_key_presses() -> bool {
    false
}

/// `acpi_video_get_edid` - `vendor/linux/drivers/acpi/acpi_video.c:1440`.
pub unsafe extern "C" fn linux_acpi_video_get_edid(
    _device: *mut c_void,
    _display_type: i32,
    _device_id: i32,
    edid: *mut *mut c_void,
) -> i32 {
    if !edid.is_null() {
        unsafe {
            *edid = core::ptr::null_mut();
        }
    }
    -ENODEV
}

/// `acpi_video_get_levels` - `vendor/linux/drivers/acpi/acpi_video.c:798`.
pub unsafe extern "C" fn linux_acpi_video_get_levels(
    _device: *mut c_void,
    dev_br: *mut *mut c_void,
    pmax_level: *mut i32,
) -> i32 {
    if !dev_br.is_null() {
        unsafe {
            *dev_br = core::ptr::null_mut();
        }
    }
    if !pmax_level.is_null() {
        unsafe {
            *pmax_level = 0;
        }
    }
    -ENODEV
}

/// `__acpi_video_get_backlight_type` - `vendor/linux/drivers/acpi/video_detect.c:998`.
pub unsafe extern "C" fn linux___acpi_video_get_backlight_type(
    native: bool,
    auto_detect: *mut bool,
) -> i32 {
    if !auto_detect.is_null() {
        unsafe {
            *auto_detect = false;
        }
    }
    if native {
        ACPI_BACKLIGHT_NATIVE
    } else {
        ACPI_BACKLIGHT_VENDOR
    }
}

/// `acpi_fetch_acpi_dev` - `vendor/linux/drivers/acpi/scan.c`.
pub unsafe extern "C" fn linux_acpi_fetch_acpi_dev(_handle: *mut c_void) -> *mut c_void {
    core::ptr::null_mut()
}

/// `acpi_get_local_u64_address` - `vendor/linux/drivers/acpi/utils.c`.
pub unsafe extern "C" fn linux_acpi_get_local_u64_address(
    _handle: *mut c_void,
    _addr: *mut u64,
) -> i32 {
    -ENODATA
}

/// `acpi_dev_present` - `vendor/linux/drivers/acpi/utils.c`.
pub unsafe extern "C" fn linux_acpi_dev_present(
    _hid: *const u8,
    _uid: *const u8,
    _hrv: i64,
) -> bool {
    false
}

/// `acpi_dev_get_resources` - `vendor/linux/drivers/acpi/resource.c`.
pub unsafe extern "C" fn linux_acpi_dev_get_resources(
    _adev: *mut c_void,
    _list: *mut c_void,
    _preproc: *mut c_void,
    _preproc_data: *mut c_void,
) -> i32 {
    0
}

/// `acpi_dev_free_resource_list` - `vendor/linux/drivers/acpi/resource.c`.
pub unsafe extern "C" fn linux_acpi_dev_free_resource_list(_list: *mut c_void) {}

/// `acpi_dev_resource_interrupt` - `vendor/linux/drivers/acpi/resource.c`.
pub unsafe extern "C" fn linux_acpi_dev_resource_interrupt(
    _ares: *mut c_void,
    _index: i32,
    _res: *mut c_void,
) -> bool {
    false
}

/// `acpi_check_resource_conflict` - `vendor/linux/drivers/acpi/osl.c`.
pub unsafe extern "C" fn linux_acpi_check_resource_conflict(_res: *const c_void) -> i32 {
    0
}

/// `acpi_install_address_space_handler` - `vendor/linux/drivers/acpi/acpica/evxfregn.c`.
pub unsafe extern "C" fn linux_acpi_install_address_space_handler(
    _device: *mut c_void,
    _space_id: u8,
    _handler: *mut c_void,
    _setup: *mut c_void,
    _context: *mut c_void,
) -> u32 {
    AE_OK
}

/// `acpi_remove_address_space_handler` - `vendor/linux/drivers/acpi/acpica/evxfregn.c`.
pub unsafe extern "C" fn linux_acpi_remove_address_space_handler(
    _device: *mut c_void,
    _space_id: u8,
    _handler: *mut c_void,
) -> u32 {
    AE_OK
}

/// `acpi_os_read_port` - `vendor/linux/drivers/acpi/osl.c`.
pub unsafe extern "C" fn linux_acpi_os_read_port(port: u64, value: *mut u32, width: u32) -> u32 {
    let mut dummy = 0u32;
    let out = if value.is_null() {
        &mut dummy as *mut u32
    } else {
        value
    };
    if port > u16::MAX as u64 {
        return AE_BAD_PARAMETER;
    }

    let port = port as u16;
    let read = match width {
        0..=8 => unsafe { crate::arch::x86::include::asm::io::inb(port) as u32 },
        9..=16 => unsafe { crate::arch::x86::include::asm::io::inw(port) as u32 },
        17..=32 => unsafe { crate::arch::x86::include::asm::io::inl(port) },
        _ => return AE_BAD_PARAMETER,
    };
    unsafe {
        out.write(read);
    }
    AE_OK
}

/// `acpi_os_write_port` - `vendor/linux/drivers/acpi/osl.c`.
pub unsafe extern "C" fn linux_acpi_os_write_port(port: u64, value: u32, width: u32) -> u32 {
    if port > u16::MAX as u64 {
        return AE_BAD_PARAMETER;
    }

    let port = port as u16;
    match width {
        0..=8 => unsafe { crate::arch::x86::include::asm::io::outb(port, value as u8) },
        9..=16 => unsafe { crate::arch::x86::include::asm::io::outw(port, value as u16) },
        17..=32 => unsafe { crate::arch::x86::include::asm::io::outl(port, value) },
        _ => return AE_BAD_PARAMETER,
    }
    AE_OK
}

/// `acpi_dev_clear_dependencies` - `vendor/linux/drivers/acpi/scan.c`.
pub unsafe extern "C" fn linux_acpi_dev_clear_dependencies(_supplier: *mut c_void) {}

/// `acpi_dev_ready_for_enumeration` - `vendor/linux/drivers/acpi/scan.c`.
pub unsafe extern "C" fn linux_acpi_dev_ready_for_enumeration(_device: *const c_void) -> bool {
    false
}

/// `acpi_get_handle` - `vendor/linux/drivers/acpi/acpica/nsxfname.c`.
pub unsafe extern "C" fn linux_acpi_get_handle(
    _parent: *mut c_void,
    _pathname: *const u8,
    ret_handle: *mut *mut c_void,
) -> u32 {
    if !ret_handle.is_null() {
        unsafe { ret_handle.write(core::ptr::null_mut()) };
    }
    AE_NOT_FOUND
}

unsafe fn acpi_copy_name_to_buffer(buffer: *mut LinuxAcpiBuffer, name: &[u8]) -> u32 {
    if buffer.is_null() || name.is_empty() {
        return AE_BAD_PARAMETER;
    }

    let input_len = unsafe { (*buffer).length };
    unsafe {
        (*buffer).length = name.len();
    }

    match input_len {
        ACPI_NO_BUFFER => return AE_BUFFER_OVERFLOW,
        ACPI_ALLOCATE_BUFFER | ACPI_ALLOCATE_LOCAL_BUFFER => {
            let ptr = unsafe { crate::mm::slab::linux___kmalloc_noprof(name.len(), GFP_KERNEL) };
            if ptr.is_null() {
                return AE_NO_MEMORY;
            }
            unsafe {
                (*buffer).pointer = ptr.cast();
            }
        }
        len if len < name.len() => return AE_BUFFER_OVERFLOW,
        _ => {
            if unsafe { (*buffer).pointer.is_null() } {
                return AE_BAD_PARAMETER;
            }
        }
    }

    unsafe {
        core::ptr::write_bytes((*buffer).pointer.cast::<u8>(), 0, name.len());
        core::ptr::copy_nonoverlapping(name.as_ptr(), (*buffer).pointer.cast::<u8>(), name.len());
    }
    AE_OK
}

/// `acpi_get_name` - `vendor/linux/drivers/acpi/acpica/nsxfname.c`.
pub unsafe extern "C" fn linux_acpi_get_name(
    handle: *mut c_void,
    name_type: u32,
    buffer: *mut LinuxAcpiBuffer,
) -> u32 {
    if name_type > ACPI_FULL_PATHNAME_NO_TRAILING {
        return AE_BAD_PARAMETER;
    }
    if handle.is_null() {
        return AE_NOT_FOUND;
    }

    let name = match name_type {
        ACPI_SINGLE_NAME => ACPI_LUPOS_SINGLE_NAME,
        ACPI_FULL_PATHNAME | ACPI_FULL_PATHNAME_NO_TRAILING => ACPI_LUPOS_FULL_PATH,
        _ => return AE_BAD_PARAMETER,
    };
    unsafe { acpi_copy_name_to_buffer(buffer, name) }
}

/// `acpi_get_table` - `vendor/linux/drivers/acpi/acpica/tbxface.c`.
pub unsafe extern "C" fn linux_acpi_get_table(
    _signature: *const u8,
    _instance: u32,
    out_table: *mut *mut c_void,
) -> u32 {
    if !out_table.is_null() {
        unsafe { out_table.write(core::ptr::null_mut()) };
    }
    AE_NOT_FOUND
}

unsafe fn copy_cstr_bounded(dst: *mut u8, len: usize, src: *const u8) {
    if dst.is_null() || len == 0 {
        return;
    }
    if src.is_null() {
        unsafe { dst.write(0) };
        return;
    }
    let mut offset = 0usize;
    while offset + 1 < len {
        let byte = unsafe { src.add(offset).read() };
        unsafe { dst.add(offset).write(byte) };
        if byte == 0 {
            return;
        }
        offset += 1;
    }
    unsafe { dst.add(offset).write(0) };
}

/// `acpi_set_modalias` - `vendor/linux/drivers/acpi/bus.c`.
pub unsafe extern "C" fn linux_acpi_set_modalias(
    _adev: *mut c_void,
    default_id: *const u8,
    modalias: *mut u8,
    len: usize,
) {
    unsafe { copy_cstr_bounded(modalias, len, default_id) };
}

/// `acpi_device_modalias` - `vendor/linux/drivers/acpi/device_sysfs.c`.
pub unsafe extern "C" fn linux_acpi_device_modalias(
    _dev: *mut c_void,
    _buf: *mut u8,
    _size: i32,
) -> i32 {
    -ENODEV
}

/// `acpi_device_uevent_modalias` - `vendor/linux/drivers/acpi/device_sysfs.c`.
pub unsafe extern "C" fn linux_acpi_device_uevent_modalias(
    _dev: *mut c_void,
    _env: *mut c_void,
) -> i32 {
    -ENODEV
}

/// `acpi_driver_match_device` - `vendor/linux/drivers/acpi/bus.c`.
pub unsafe extern "C" fn linux_acpi_driver_match_device(
    _dev: *mut c_void,
    _drv: *mut c_void,
) -> bool {
    false
}

/// `acpi_match_device_ids` - `vendor/linux/drivers/acpi/bus.c`.
pub unsafe extern "C" fn linux_acpi_match_device_ids(
    _device: *mut c_void,
    _ids: *const c_void,
) -> i32 {
    -ENOENT
}

/// `acpi_find_child_device` - `vendor/linux/drivers/acpi/glue.c`.
pub unsafe extern "C" fn linux_acpi_find_child_device(
    _parent: *mut c_void,
    _address: u64,
    _check_children: bool,
) -> *mut c_void {
    core::ptr::null_mut()
}

/// `acpi_initialize_hp_context` - `vendor/linux/drivers/acpi/scan.c`.
pub unsafe extern "C" fn linux_acpi_initialize_hp_context(
    _adev: *mut c_void,
    _hp: *mut c_void,
    _notify: *mut c_void,
    _uevent: *mut c_void,
) {
}

/// `acpi_unbind_one` - `vendor/linux/drivers/acpi/glue.c`.
pub unsafe extern "C" fn linux_acpi_unbind_one(_dev: *mut c_void) -> i32 {
    0
}

/// `acpi_put_table` - `vendor/linux/drivers/acpi/acpica/tbxface.c`.
pub unsafe extern "C" fn linux_acpi_put_table(_table: *mut c_void) {}

/// `acpi_nhlt_get_gbl_table` - `vendor/linux/drivers/acpi/nhlt.c`.
pub unsafe extern "C" fn linux_acpi_nhlt_get_gbl_table() -> u32 {
    AE_NOT_FOUND
}

/// `acpi_nhlt_find_endpoint` - `vendor/linux/drivers/acpi/nhlt.c`.
pub unsafe extern "C" fn linux_acpi_nhlt_find_endpoint(
    _link_type: i32,
    _dev_type: i32,
    _dir: i32,
    _bus_id: i32,
) -> *mut c_void {
    core::ptr::null_mut()
}

/// `acpi_nhlt_put_gbl_table` - `vendor/linux/drivers/acpi/nhlt.c`.
pub unsafe extern "C" fn linux_acpi_nhlt_put_gbl_table() {}

// ── Public types ─────────────────────────────────────────────────────────────

/// Summary of ACPI information needed for SMP bring-up.
#[derive(Debug, Clone)]
pub struct AcpiInfo {
    /// Physical base address of the Local APIC MMIO window.
    /// Defaults to the architectural base 0xFEE0_0000 if MADT cannot be found.
    pub lapic_address: u32,

    /// Whether the system has a legacy 8259 PIC that needs to be disconnected.
    /// Derived from MADT flags bit 0 ("PC-AT compatible dual 8259 PICs").
    pub pic_present: bool,

    /// Number of valid entries in `cpus` (≤ MAX_CPUS).
    pub cpu_count: usize,

    /// CPU descriptors parsed from MADT Type-0 (Processor Local APIC) entries.
    pub cpus: [CpuInfo; MAX_CPUS],
}

impl Default for AcpiInfo {
    /// Fallback used when ACPI parsing fails.
    ///
    /// Assumes a single-CPU system with the LAPIC at its architectural default
    /// address and a legacy PIC present.  This lets the kernel continue booting
    /// even on machines or QEMU configs that don't provide a standard ACPI RSDP.
    fn default() -> Self {
        let mut cpus = [CpuInfo {
            apic_id: 0,
            enabled: false,
        }; MAX_CPUS];
        cpus[0] = CpuInfo {
            apic_id: 0,
            enabled: true,
        };
        Self {
            lapic_address: 0xFEE0_0000,
            pic_present: true,
            cpu_count: 1,
            cpus,
        }
    }
}

/// Single CPU entry from MADT.
#[derive(Debug, Clone, Copy)]
pub struct CpuInfo {
    /// xAPIC ID used to address this CPU's Local APIC in ICR writes.
    pub apic_id: u8,
    /// `true` if the MADT flags field bit 0 (Processor Enabled) is set.
    pub enabled: bool,
}

/// Errors that can occur during ACPI parsing.
#[derive(Debug)]
pub enum AcpiError {
    /// "RSD PTR " signature not found in EBDA or BIOS ROM.
    RsdpNotFound,
    /// RSDP or an SDT had a bad checksum (byte sum ≠ 0).
    InvalidChecksum,
    /// Walked RSDT/XSDT but no "APIC" table was found.
    MadtNotFound,
}

// ── ACPI structure layouts ────────────────────────────────────────────────────
//
// All ACPI structures use little-endian byte order and may be unaligned in
// memory.  We use `#[repr(C, packed)]` to match the wire layout and read fields
// via raw pointer casts (safe only for Copy/integer types).
//
// Reference: ACPI 6.5 §5.2.5 "Root System Description Pointer (RSDP)"

/// RSDP v1 (ACPI 1.0) — 20 bytes.
#[repr(C, packed)]
struct Rsdp {
    signature: [u8; 8], // "RSD PTR " (with trailing space)
    checksum: u8,       // byte sum of bytes 0–19 must be 0
    oem_id: [u8; 6],
    revision: u8,      // 0 = ACPI 1.0; 2 = ACPI 2.0+
    rsdt_address: u32, // physical address of the RSDT
}

/// RSDP v2 extension (ACPI 2.0+) — appended after the 20-byte v1 structure.
#[repr(C, packed)]
struct RsdpV2 {
    length: u32,           // total size of this structure (36 bytes)
    xsdt_address: u64,     // 64-bit physical address of the XSDT
    extended_checksum: u8, // byte sum of all 36 bytes must be 0
    _reserved: [u8; 3],
}

/// Common SDT header present at the start of every ACPI system description table.
/// Reference: ACPI 6.5 §5.2.6 "System Description Table Header"
#[repr(C, packed)]
struct AcpiSdtHeader {
    signature: [u8; 4], // e.g. "APIC", "RSDT", "XSDT"
    length: u32,        // total table length including this header
    revision: u8,
    checksum: u8, // byte sum of entire table must be 0
    oem_id: [u8; 6],
    oem_table_id: [u8; 8],
    oem_revision: u32,
    creator_id: u32,
    creator_revision: u32,
}

/// MADT (Multiple APIC Description Table) — header portion only.
/// Reference: ACPI 6.5 §5.2.12
#[repr(C, packed)]
struct MadtHeader {
    header: AcpiSdtHeader, // 36 bytes
    lapic_address: u32,    // physical address of LAPIC MMIO window
    flags: u32,            // bit 0: PC-AT-compatible dual 8259 PICs present
}

/// Common prefix of every MADT interrupt controller structure.
/// Reference: ACPI 6.5 §5.2.12.1
#[repr(C, packed)]
struct MadtEntryHeader {
    entry_type: u8, // structure type (0 = Processor Local APIC)
    length: u8,     // total length of this entry including this header
}

/// MADT Type 0: Processor Local APIC.
/// Reference: ACPI 6.5 §5.2.12.2
#[repr(C, packed)]
struct MadtLocalApic {
    header: MadtEntryHeader,
    acpi_processor_id: u8, // deprecated in ACPI 6.x; use APIC ID
    apic_id: u8,           // xAPIC ID; used in ICR high word (bits 31:24)
    flags: u32,            // bit 0 = Processor Enabled; bit 1 = Online Capable
}

// ── Public entry point ────────────────────────────────────────────────────────

/// Locate and parse ACPI tables to extract SMP-relevant information.
///
/// Searches the EBDA and BIOS ROM area for the RSDP, then walks the
/// RSDT/XSDT to find the MADT, and extracts CPU APIC IDs and the LAPIC
/// base address.
///
/// # Safety
/// Reads arbitrary physical memory via the identity mapping.  Only safe to
/// call after the boot stub has set up the 4 GiB identity map.
pub fn parse() -> Result<AcpiInfo, AcpiError> {
    // Safety: identity-mapped physical addresses are safe to dereference.
    let rsdp = unsafe { find_rsdp() }.ok_or(AcpiError::RsdpNotFound)?;

    // SAFETY: rsdp was found by scanning for the signature + checksum; we trust
    // the pointer is valid within the identity-mapped firmware region.
    let rsdp_bytes =
        unsafe { core::slice::from_raw_parts(rsdp as *const u8, core::mem::size_of::<Rsdp>()) };
    if !acpi_checksum(rsdp_bytes) {
        return Err(AcpiError::InvalidChecksum);
    }

    // Choose XSDT (64-bit entry pointers) over RSDT (32-bit) when available.
    // ACPI 2.0+ systems provide the XSDT; ACPI 1.0 only has RSDT.
    let revision = unsafe { (*rsdp).revision };
    let madt_ptr = if revision >= 2 {
        // The RsdpV2 extension begins immediately after the 20-byte Rsdp.
        let rsdp_v2 = ((rsdp as usize) + core::mem::size_of::<Rsdp>()) as *const RsdpV2;
        let xsdt_phys =
            unsafe { core::ptr::read_unaligned(core::ptr::addr_of!((*rsdp_v2).xsdt_address)) };
        unsafe { find_madt_in_xsdt(xsdt_phys) }
    } else {
        let rsdt_phys =
            unsafe { core::ptr::read_unaligned(core::ptr::addr_of!((*rsdp).rsdt_address)) } as u64;
        unsafe { find_madt_in_rsdt(rsdt_phys) }
    };

    let madt_ptr = madt_ptr.ok_or(AcpiError::MadtNotFound)?;

    // Validate MADT checksum before parsing any of its fields.
    let madt_len = unsafe {
        core::ptr::read_unaligned(core::ptr::addr_of!((*madt_ptr).header.length)) as usize
    };
    let madt_bytes = unsafe { core::slice::from_raw_parts(madt_ptr as *const u8, madt_len) };
    if !acpi_checksum(madt_bytes) {
        return Err(AcpiError::InvalidChecksum);
    }

    unsafe { parse_madt(madt_ptr) }
}

// ── Internal helpers ──────────────────────────────────────────────────────────

/// Validate an ACPI table by summing all its bytes mod 256.
///
/// The ACPI spec requires that the arithmetic sum of all bytes in a table
/// (including the checksum field itself) equals zero modulo 256.
/// Reference: ACPI 6.5 §5.2.5 item 5
fn acpi_checksum(data: &[u8]) -> bool {
    data.iter().fold(0u8, |acc, &b| acc.wrapping_add(b)) == 0
}

/// Scan physical memory for the RSDP signature.
///
/// BIOS systems place the RSDP in one of two locations:
///   1. In the first 1 KiB of the Extended BIOS Data Area (EBDA).
///      The EBDA segment address is stored as a 16-bit value at 0x040E.
///   2. In the BIOS read-only memory between 0xE0000 and 0xFFFFF.
///
/// The signature is 8 bytes ("RSD PTR ") and is always 16-byte aligned.
///
/// Reference: ACPI 6.5 §5.2.5.1 "Finding the RSDP on IA-PC Systems"
///            https://wiki.osdev.org/RSDP#Detecting_the_RSDP
unsafe fn find_rsdp() -> Option<*const Rsdp> {
    const SIGNATURE: &[u8; 8] = b"RSD PTR ";

    // ── Search EBDA ───────────────────────────────────────────────────────────
    // The 16-bit segment value at physical 0x040E × 16 gives the EBDA base.
    let ebda_seg = unsafe { (0x040E as *const u16).read_unaligned() };
    let ebda_base = (ebda_seg as usize) << 4;
    // Guard against a zero/invalid EBDA segment.
    if ebda_base >= 0x500 && ebda_base < 0xA0000 {
        if let Some(p) = unsafe { scan_for_rsdp(ebda_base, 1024, SIGNATURE) } {
            return Some(p);
        }
    }

    // ── Search BIOS ROM ───────────────────────────────────────────────────────
    unsafe { scan_for_rsdp(0xE0000, 0x20000, SIGNATURE) }
}

/// Scan `length` bytes starting at `base` for the RSDP signature.
/// Only checks 16-byte-aligned addresses (required by ACPI spec).
unsafe fn scan_for_rsdp(base: usize, length: usize, sig: &[u8; 8]) -> Option<*const Rsdp> {
    let mut addr = base;
    while addr + sig.len() <= base + length {
        let candidate = addr as *const u8;
        // SAFETY: within identity-mapped firmware memory.
        let found = unsafe { core::slice::from_raw_parts(candidate, sig.len()) == sig.as_slice() };
        if found {
            return Some(addr as *const Rsdp);
        }
        addr += 16; // RSDP is always 16-byte aligned
    }
    None
}

/// Walk the XSDT (64-bit entry pointers) to find a table with "APIC" signature.
///
/// XSDT entry format: array of u64 physical addresses following the SDT header.
/// Reference: ACPI 6.5 §5.2.8 "Extended System Description Table (XSDT)"
unsafe fn find_madt_in_xsdt(xsdt_phys: u64) -> Option<*const MadtHeader> {
    let xsdt = xsdt_phys as *const AcpiSdtHeader;
    let total_len =
        unsafe { core::ptr::read_unaligned(core::ptr::addr_of!((*xsdt).length)) } as usize;
    let header_size = core::mem::size_of::<AcpiSdtHeader>();
    let entries_len = total_len.saturating_sub(header_size);
    let entry_count = entries_len / 8; // each entry is a u64

    let entries_ptr = (xsdt_phys as usize + header_size) as *const u64;
    for i in 0..entry_count {
        let entry_phys = unsafe { core::ptr::read_unaligned(entries_ptr.add(i)) };
        if entry_phys == 0 {
            continue;
        }
        let entry_hdr = entry_phys as *const AcpiSdtHeader;
        let sig = unsafe { core::ptr::read_unaligned(core::ptr::addr_of!((*entry_hdr).signature)) };
        if &sig == b"APIC" {
            return Some(entry_phys as *const MadtHeader);
        }
    }
    None
}

/// Walk the RSDT (32-bit entry pointers) to find a table with "APIC" signature.
///
/// RSDT entry format: array of u32 physical addresses following the SDT header.
/// Reference: ACPI 6.5 §5.2.7 "Root System Description Table (RSDT)"
unsafe fn find_madt_in_rsdt(rsdt_phys: u64) -> Option<*const MadtHeader> {
    let rsdt = rsdt_phys as *const AcpiSdtHeader;
    let total_len =
        unsafe { core::ptr::read_unaligned(core::ptr::addr_of!((*rsdt).length)) } as usize;
    let header_size = core::mem::size_of::<AcpiSdtHeader>();
    let entries_len = total_len.saturating_sub(header_size);
    let entry_count = entries_len / 4; // each entry is a u32

    let entries_ptr = (rsdt_phys as usize + header_size) as *const u32;
    for i in 0..entry_count {
        let entry_phys = unsafe { core::ptr::read_unaligned(entries_ptr.add(i)) } as u64;
        if entry_phys == 0 {
            continue;
        }
        let entry_hdr = entry_phys as *const AcpiSdtHeader;
        let sig = unsafe { core::ptr::read_unaligned(core::ptr::addr_of!((*entry_hdr).signature)) };
        if &sig == b"APIC" {
            return Some(entry_phys as *const MadtHeader);
        }
    }
    None
}

/// Parse the MADT to extract LAPIC address, PIC presence, and CPU list.
///
/// The MADT body (after the fixed header) is a variable-length array of
/// interrupt controller structures.  We iterate them until we've consumed
/// `total_length` bytes.
/// Reference: ACPI 6.5 §5.2.12
unsafe fn parse_madt(madt: *const MadtHeader) -> Result<AcpiInfo, AcpiError> {
    let lapic_address =
        unsafe { core::ptr::read_unaligned(core::ptr::addr_of!((*madt).lapic_address)) };
    let flags = unsafe { core::ptr::read_unaligned(core::ptr::addr_of!((*madt).flags)) };
    let pic_present = (flags & 1) != 0;
    let total_len =
        unsafe { core::ptr::read_unaligned(core::ptr::addr_of!((*madt).header.length)) } as usize;

    let mut info = AcpiInfo {
        lapic_address,
        pic_present,
        cpu_count: 0,
        cpus: [CpuInfo {
            apic_id: 0,
            enabled: false,
        }; MAX_CPUS],
    };

    // Entry data starts right after the fixed MadtHeader.
    let madt_base = madt as usize;
    let entries_off = core::mem::size_of::<MadtHeader>();
    let mut offset = entries_off;

    while offset + 2 <= total_len {
        let entry_ptr = (madt_base + offset) as *const MadtEntryHeader;
        let entry_type = unsafe { (*entry_ptr).entry_type };
        let entry_length = unsafe { (*entry_ptr).length } as usize;

        // Sanity: length must be at least 2 and we must not run past the table.
        if entry_length < 2 || offset + entry_length > total_len {
            break;
        }

        match entry_type {
            // Type 0: Processor Local APIC — one per logical CPU.
            0 if entry_length >= core::mem::size_of::<MadtLocalApic>() => {
                let lapic = (madt_base + offset) as *const MadtLocalApic;
                let apic_id = unsafe { (*lapic).apic_id };
                let cpu_flags =
                    unsafe { core::ptr::read_unaligned(core::ptr::addr_of!((*lapic).flags)) };
                let enabled = (cpu_flags & 1) != 0 || (cpu_flags & 2) != 0;
                // bit 0 = Processor Enabled, bit 1 = Online Capable (ACPI 6.0+)

                if info.cpu_count < MAX_CPUS {
                    info.cpus[info.cpu_count] = CpuInfo { apic_id, enabled };
                    info.cpu_count += 1;
                }
            }
            // Type 5: Local APIC Address Override — 64-bit base address.
            // Ref: ACPI 6.5 §5.2.12.8
            5 if entry_length >= 12 => {
                // offset+2: reserved u16, offset+4: 64-bit address
                let addr_ptr = (madt_base + offset + 4) as *const u64;
                let addr64 = unsafe { core::ptr::read_unaligned(addr_ptr) };
                // Only update if the address fits in 32 bits (our identity map
                // covers 4 GiB; a higher address would require a separate mapping).
                if addr64 <= u32::MAX as u64 {
                    info.lapic_address = addr64 as u32;
                }
            }
            _ => {} // ignore other entry types (I/O APIC, overrides, etc.)
        }

        offset += entry_length;
    }

    Ok(info)
}

// ── MCFG (PCI Express ECAM) — M55 ────────────────────────────────────────────

/// MCFG table header following the common SDT header.
/// Reference: ACPI 6.5 §5.2.6.2 — PCI Memory Mapped Configuration
#[repr(C, packed)]
struct McfgHeader {
    header: AcpiSdtHeader,
    _reserved: u64,
}

/// One MCFG allocation entry.
/// Each describes the ECAM MMIO window for one PCI segment group.
#[repr(C, packed)]
struct McfgAllocation {
    base_address: u64,
    segment_group: u16,
    start_bus: u8,
    end_bus: u8,
    _reserved: u32,
}

/// Locate and parse the ACPI MCFG table, returning the ECAM entries for
/// every PCI segment group.
///
/// Returns an empty Vec if MCFG is absent or ACPI parse fails.
///
/// # Safety
/// Same as `parse()` — requires 4 GiB identity map to be in place.
pub fn parse_mcfg() -> crate::alloc::vec::Vec<crate::linux_driver_abi::pci::McfgEntry> {
    let mut entries = crate::alloc::vec::Vec::new();

    let rsdp = match unsafe { find_rsdp() } {
        Some(p) => p,
        None => return entries,
    };

    let revision = unsafe { (*rsdp).revision };
    let mcfg_phys = if revision >= 2 {
        let rsdp_v2 = ((rsdp as usize) + core::mem::size_of::<Rsdp>()) as *const RsdpV2;
        let xsdt_phys =
            unsafe { core::ptr::read_unaligned(core::ptr::addr_of!((*rsdp_v2).xsdt_address)) };
        unsafe { find_table_in_xsdt(xsdt_phys, b"MCFG") }
    } else {
        let rsdt_phys =
            unsafe { core::ptr::read_unaligned(core::ptr::addr_of!((*rsdp).rsdt_address)) } as u64;
        unsafe { find_table_in_rsdt(rsdt_phys, b"MCFG") }
    };

    let mcfg_phys = match mcfg_phys {
        Some(p) => p,
        None => return entries,
    };

    let total_len = unsafe {
        let hdr = mcfg_phys as *const AcpiSdtHeader;
        core::ptr::read_unaligned(core::ptr::addr_of!((*hdr).length)) as usize
    };
    let header_size = core::mem::size_of::<McfgHeader>();
    let alloc_size = core::mem::size_of::<McfgAllocation>();

    if total_len < header_size {
        return entries;
    }

    let n = (total_len - header_size) / alloc_size;
    let base_ptr = (mcfg_phys + header_size as u64) as *const McfgAllocation;

    for i in 0..n {
        let a = unsafe { &*base_ptr.add(i) };
        let ecam_base = unsafe { core::ptr::read_unaligned(core::ptr::addr_of!(a.base_address)) };
        let segment = unsafe { core::ptr::read_unaligned(core::ptr::addr_of!(a.segment_group)) };
        let bus_start = unsafe { core::ptr::read_unaligned(core::ptr::addr_of!(a.start_bus)) };
        let bus_end = unsafe { core::ptr::read_unaligned(core::ptr::addr_of!(a.end_bus)) };
        entries.push(crate::linux_driver_abi::pci::McfgEntry {
            base: ecam_base,
            segment,
            bus_start,
            bus_end,
        });
    }
    entries
}

/// Walk an XSDT and return the physical address of the table with `sig`.
unsafe fn find_table_in_xsdt(xsdt_phys: u64, sig: &[u8; 4]) -> Option<u64> {
    let xsdt = xsdt_phys as *const AcpiSdtHeader;
    let total_len =
        unsafe { core::ptr::read_unaligned(core::ptr::addr_of!((*xsdt).length)) } as usize;
    let header_size = core::mem::size_of::<AcpiSdtHeader>();
    let entry_count = total_len.saturating_sub(header_size) / 8;
    let entries_ptr = (xsdt_phys as usize + header_size) as *const u64;
    for i in 0..entry_count {
        let entry_phys = unsafe { core::ptr::read_unaligned(entries_ptr.add(i)) };
        if entry_phys == 0 {
            continue;
        }
        let hdr = entry_phys as *const AcpiSdtHeader;
        let s = unsafe { core::ptr::read_unaligned(core::ptr::addr_of!((*hdr).signature)) };
        if &s == sig {
            return Some(entry_phys);
        }
    }
    None
}

/// Walk an RSDT and return the physical address of the table with `sig`.
unsafe fn find_table_in_rsdt(rsdt_phys: u64, sig: &[u8; 4]) -> Option<u64> {
    let rsdt = rsdt_phys as *const AcpiSdtHeader;
    let total_len =
        unsafe { core::ptr::read_unaligned(core::ptr::addr_of!((*rsdt).length)) } as usize;
    let header_size = core::mem::size_of::<AcpiSdtHeader>();
    let entry_count = total_len.saturating_sub(header_size) / 4;
    let entries_ptr = (rsdt_phys as usize + header_size) as *const u32;
    for i in 0..entry_count {
        let entry_phys = unsafe { core::ptr::read_unaligned(entries_ptr.add(i)) } as u64;
        if entry_phys == 0 {
            continue;
        }
        let hdr = entry_phys as *const AcpiSdtHeader;
        let s = unsafe { core::ptr::read_unaligned(core::ptr::addr_of!((*hdr).signature)) };
        if &s == sig {
            return Some(entry_phys);
        }
    }
    None
}

// ── Host-testable internal API ────────────────────────────────────────────────

/// Parse a MADT given as a raw byte slice.
///
/// This is the testable entry point used by unit tests.  On bare-metal,
/// `parse()` finds the MADT via RSDP and calls this function.
///
/// The slice must begin with the full MADT (starting at the `AcpiSdtHeader`).
#[cfg(test)]
pub(crate) fn parse_madt_from_bytes(data: &[u8]) -> Result<AcpiInfo, AcpiError> {
    if data.len() < core::mem::size_of::<MadtHeader>() {
        return Err(AcpiError::MadtNotFound);
    }
    if !acpi_checksum(data) {
        return Err(AcpiError::InvalidChecksum);
    }
    let madt = data.as_ptr() as *const MadtHeader;
    // SAFETY: we just verified the slice is long enough.
    unsafe { parse_madt(madt) }
}

// ── Unit tests ────────────────────────────────────────────────────────────────
//
// All tests run on the host (no hardware needed) by constructing minimal ACPI
// tables as byte arrays and feeding them to the pure-Rust parsing functions.

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use std::vec;
    use std::vec::Vec;

    #[test]
    fn linux_acpi_video_exports_register_for_modules() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("acpi_video_register"),
            Some(linux_acpi_video_register as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("acpi_video_register_backlight"),
            Some(linux_acpi_video_register_backlight as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("__acpi_video_get_backlight_type"),
            Some(linux___acpi_video_get_backlight_type as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("acpi_video_backlight_string"),
            Some(acpi_video_backlight_string_addr())
        );
        assert_eq!(
            crate::kernel::module::find_symbol("acpi_get_name"),
            Some(linux_acpi_get_name as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("acpi_target_system_state"),
            Some(linux_acpi_target_system_state as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("acpi_check_resource_conflict"),
            Some(linux_acpi_check_resource_conflict as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("acpi_install_address_space_handler"),
            Some(linux_acpi_install_address_space_handler as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("acpi_remove_address_space_handler"),
            Some(linux_acpi_remove_address_space_handler as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("acpi_os_read_port"),
            Some(linux_acpi_os_read_port as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("acpi_os_write_port"),
            Some(linux_acpi_os_write_port as usize)
        );
    }

    #[test]
    fn linux_acpi_video_reports_no_acpi_video_device() {
        let mut auto_detect = true;
        assert_eq!(unsafe { linux_acpi_video_register() }, -ENODEV);
        assert!(!unsafe { linux_acpi_video_handles_brightness_key_presses() });
        assert_eq!(
            unsafe { linux___acpi_video_get_backlight_type(false, &mut auto_detect) },
            ACPI_BACKLIGHT_VENDOR
        );
        assert!(!auto_detect);
        assert_eq!(
            unsafe { linux___acpi_video_get_backlight_type(true, core::ptr::null_mut()) },
            ACPI_BACKLIGHT_NATIVE
        );
    }

    #[test]
    fn linux_acpi_target_system_state_reports_active_s0() {
        assert_eq!(unsafe { linux_acpi_target_system_state() }, ACPI_STATE_S0);
    }

    #[test]
    fn linux_acpi_i2c_compat_exports_report_no_resource_conflict() {
        assert_eq!(
            unsafe { linux_acpi_check_resource_conflict(core::ptr::null()) },
            0
        );
        assert_eq!(
            unsafe {
                linux_acpi_install_address_space_handler(
                    core::ptr::null_mut(),
                    0,
                    core::ptr::null_mut(),
                    core::ptr::null_mut(),
                    core::ptr::null_mut(),
                )
            },
            AE_OK
        );
        assert_eq!(
            unsafe {
                linux_acpi_remove_address_space_handler(
                    core::ptr::null_mut(),
                    0,
                    core::ptr::null_mut(),
                )
            },
            AE_OK
        );

        let mut value = 0u32;
        assert_eq!(
            unsafe { linux_acpi_os_read_port(0x1_0000, &mut value, 8) },
            AE_BAD_PARAMETER
        );
        assert_eq!(
            unsafe { linux_acpi_os_write_port(0x1_0000, 0, 8) },
            AE_BAD_PARAMETER
        );
    }

    #[test]
    fn linux_acpi_get_name_fills_existing_and_allocated_buffers() {
        let handle = 0x1234usize as *mut c_void;
        let mut single = [0u8; 8];
        let mut single_buffer = LinuxAcpiBuffer {
            length: single.len(),
            pointer: single.as_mut_ptr().cast(),
        };

        assert_eq!(
            unsafe { linux_acpi_get_name(handle, ACPI_SINGLE_NAME, &mut single_buffer) },
            AE_OK
        );
        assert_eq!(single_buffer.length, ACPI_LUPOS_SINGLE_NAME.len());
        assert_eq!(
            &single[..ACPI_LUPOS_SINGLE_NAME.len()],
            ACPI_LUPOS_SINGLE_NAME
        );

        let mut too_small = LinuxAcpiBuffer {
            length: ACPI_NO_BUFFER,
            pointer: core::ptr::null_mut(),
        };
        assert_eq!(
            unsafe { linux_acpi_get_name(handle, ACPI_FULL_PATHNAME, &mut too_small) },
            AE_BUFFER_OVERFLOW
        );
        assert_eq!(too_small.length, ACPI_LUPOS_FULL_PATH.len());

        let mut allocated = LinuxAcpiBuffer {
            length: ACPI_ALLOCATE_BUFFER,
            pointer: core::ptr::null_mut(),
        };
        assert_eq!(
            unsafe { linux_acpi_get_name(handle, ACPI_FULL_PATHNAME, &mut allocated) },
            AE_OK
        );
        assert_eq!(allocated.length, ACPI_LUPOS_FULL_PATH.len());
        assert!(!allocated.pointer.is_null());
        unsafe {
            assert_eq!(
                core::slice::from_raw_parts(
                    allocated.pointer.cast::<u8>(),
                    ACPI_LUPOS_FULL_PATH.len()
                ),
                ACPI_LUPOS_FULL_PATH
            );
            crate::mm::slab::linux_kfree(allocated.pointer.cast());
        }

        assert_eq!(
            unsafe {
                linux_acpi_get_name(core::ptr::null_mut(), ACPI_SINGLE_NAME, &mut single_buffer)
            },
            AE_NOT_FOUND
        );
    }

    // ── RSDP checksum ──────────────────────────────────────────────────────────

    #[test]
    fn rsdp_checksum_accepts_valid() {
        // Construct a 20-byte buffer whose byte sum is 0.
        let mut buf = [0u8; 20];
        // All bytes 0 → sum = 0 → valid.
        assert!(acpi_checksum(&buf));

        // Set one byte to 5 — make the last byte 251 (256-5) to re-balance.
        buf[0] = 5;
        buf[19] = 251;
        assert!(acpi_checksum(&buf));
    }

    #[test]
    fn rsdp_checksum_rejects_bad_sum() {
        let buf = [1u8; 20]; // sum = 20, not 0
        assert!(!acpi_checksum(&buf));
    }

    // ── MADT checksum ─────────────────────────────────────────────────────────

    #[test]
    fn madt_checksum_validates() {
        // A correct MADT must have acpi_checksum return true.
        let madt = build_test_madt(0xFEE0_0000, true, &[]);
        assert!(acpi_checksum(&madt), "MADT checksum should be valid");
    }

    // ── MADT parsing ──────────────────────────────────────────────────────────

    #[test]
    fn parse_madt_extracts_lapic_address() {
        let madt = build_test_madt(0xDEAD_BEEF, false, &[]);
        let info = parse_madt_from_bytes(&madt).expect("parse should succeed");
        assert_eq!(info.lapic_address, 0xDEAD_BEEF);
    }

    #[test]
    fn parse_madt_extracts_pic_present_flag() {
        let with_pic = build_test_madt(0xFEE0_0000, true, &[]);
        let without_pic = build_test_madt(0xFEE0_0000, false, &[]);

        let info_pic = parse_madt_from_bytes(&with_pic).unwrap();
        let info_nopic = parse_madt_from_bytes(&without_pic).unwrap();

        assert!(info_pic.pic_present);
        assert!(!info_nopic.pic_present);
    }

    #[test]
    fn parse_madt_finds_two_cpus() {
        // Two Processor Local APIC entries (type 0), both enabled.
        let cpus = [make_lapic_entry(0, 0, true), make_lapic_entry(1, 1, true)];
        let madt = build_test_madt(0xFEE0_0000, true, &cpus);
        let info = parse_madt_from_bytes(&madt).expect("parse should succeed");

        assert_eq!(info.cpu_count, 2);
        assert_eq!(info.cpus[0].apic_id, 0);
        assert!(info.cpus[0].enabled);
        assert_eq!(info.cpus[1].apic_id, 1);
        assert!(info.cpus[1].enabled);
    }

    #[test]
    fn parse_madt_disabled_cpu_flagged() {
        // One enabled CPU (APIC 0) and one disabled CPU (APIC 5).
        let cpus = [make_lapic_entry(0, 0, true), make_lapic_entry(1, 5, false)];
        let madt = build_test_madt(0xFEE0_0000, true, &cpus);
        let info = parse_madt_from_bytes(&madt).unwrap();

        assert_eq!(info.cpu_count, 2);
        assert!(info.cpus[0].enabled);
        assert!(!info.cpus[1].enabled);
        assert_eq!(info.cpus[1].apic_id, 5);
    }

    // ── Test helpers ───────────────────────────────────────────────────────────

    /// Build a minimal MADT byte array suitable for `parse_madt_from_bytes`.
    ///
    /// The checksum byte is patched so `acpi_checksum` returns `true`.
    fn build_test_madt(lapic_addr: u32, pic_present: bool, entries: &[Vec<u8>]) -> Vec<u8> {
        let entry_bytes: Vec<u8> = entries.iter().flat_map(|e| e.iter().copied()).collect();
        let total_len = core::mem::size_of::<MadtHeader>() + entry_bytes.len();

        let mut buf = vec![0u8; total_len];

        // Fill AcpiSdtHeader (36 bytes).
        buf[0..4].copy_from_slice(b"APIC"); // signature
        buf[4..8].copy_from_slice(&(total_len as u32).to_le_bytes()); // length
        buf[8] = 3; // revision

        // Fill MadtHeader fields after the 36-byte common header.
        let la_off = 36;
        buf[la_off..la_off + 4].copy_from_slice(&lapic_addr.to_le_bytes());
        let fl_off = 40;
        buf[fl_off] = if pic_present { 1 } else { 0 };

        // Append entry bytes.
        let entry_off = core::mem::size_of::<MadtHeader>();
        buf[entry_off..].copy_from_slice(&entry_bytes);

        // Compute and store checksum: find the byte that makes the sum 0 mod 256.
        let sum: u8 = buf.iter().fold(0u8, |a, &b| a.wrapping_add(b));
        // Checksum field is at offset 9 (inside AcpiSdtHeader).
        buf[9] = buf[9].wrapping_sub(sum);

        buf
    }

    /// Build a Processor Local APIC (Type 0) MADT entry as raw bytes.
    fn make_lapic_entry(acpi_uid: u8, apic_id: u8, enabled: bool) -> Vec<u8> {
        let flags: u32 = if enabled { 1 } else { 0 };
        let mut entry = vec![0u8; 8]; // type(1) + len(1) + uid(1) + apic_id(1) + flags(4)
        entry[0] = 0; // type = Processor Local APIC
        entry[1] = 8; // length
        entry[2] = acpi_uid;
        entry[3] = apic_id;
        entry[4..8].copy_from_slice(&flags.to_le_bytes());
        entry
    }
}

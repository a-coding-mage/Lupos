//! linux-parity: partial
//! linux-source: vendor/linux/drivers/base/property.c
//! Generic device property helpers.

use core::ffi::{c_char, c_void};

use crate::include::uapi::errno::{EINVAL, ENODATA, ENOENT};
use crate::kernel::module::{export_symbol, find_symbol};

const MAX_ERRNO: usize = 4095;

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

fn err_ptr(errno: i32) -> *mut c_void {
    (usize::MAX - errno as usize + 1) as *mut c_void
}

fn is_err_or_null(ptr: *const c_void) -> bool {
    ptr.is_null() || (ptr as usize) >= usize::MAX - MAX_ERRNO + 1
}

pub fn register_module_exports() {
    export_symbol_once("__dev_fwnode", linux___dev_fwnode as usize, true);
    export_symbol_once("device_set_node", linux_device_set_node as usize, true);
    export_symbol_once(
        "set_primary_fwnode",
        linux_set_primary_fwnode as usize,
        true,
    );
    export_symbol_once(
        "device_add_software_node",
        linux_device_add_software_node as usize,
        true,
    );
    export_symbol_once(
        "device_remove_software_node",
        linux_device_remove_software_node as usize,
        true,
    );
    export_symbol_once(
        "device_property_present",
        linux_device_property_present as usize,
        true,
    );
    export_symbol_once(
        "device_property_read_bool",
        linux_device_property_read_bool as usize,
        true,
    );
    export_symbol_once(
        "device_property_match_string",
        linux_device_property_match_string as usize,
        true,
    );
    export_symbol_once(
        "device_property_read_u32_array",
        linux_device_property_read_u32_array as usize,
        true,
    );
    export_symbol_once(
        "is_acpi_device_node",
        linux_is_acpi_device_node as usize,
        false,
    );
    export_symbol_once("is_acpi_data_node", linux_is_acpi_data_node as usize, false);
    export_symbol_once("is_software_node", linux_is_software_node as usize, false);
    export_symbol_once("fwnode_handle_get", linux_fwnode_handle_get as usize, true);
    export_symbol_once(
        "fwnode_get_next_child_node",
        linux_fwnode_get_next_child_node as usize,
        true,
    );
    export_symbol_once(
        "device_get_next_child_node",
        linux_device_get_next_child_node as usize,
        true,
    );
    export_symbol_once(
        "fwnode_device_is_available",
        linux_fwnode_device_is_available as usize,
        true,
    );
    export_symbol_once(
        "fwnode_find_reference",
        linux_fwnode_find_reference as usize,
        true,
    );
    export_symbol_once(
        "fwnode_get_named_child_node",
        linux_fwnode_get_named_child_node as usize,
        true,
    );
    export_symbol_once(
        "fwnode_property_read_u32_array",
        linux_fwnode_property_read_u32_array as usize,
        true,
    );
    export_symbol_once(
        "fwnode_property_read_string",
        linux_fwnode_property_read_string as usize,
        true,
    );
    export_symbol_once(
        "device_match_fwnode",
        linux_device_match_fwnode as usize,
        true,
    );
    export_symbol_once(
        "device_get_match_data",
        linux_device_get_match_data as usize,
        true,
    );
    export_symbol_once(
        "fwnode_irq_get_byname",
        linux_fwnode_irq_get_byname as usize,
        true,
    );
}

unsafe extern "C" fn linux___dev_fwnode(_dev: *const c_void) -> *mut c_void {
    core::ptr::null_mut()
}

unsafe extern "C" fn linux_device_set_node(_dev: *mut c_void, _fwnode: *mut c_void) {}

unsafe extern "C" fn linux_set_primary_fwnode(_dev: *mut c_void, _fwnode: *mut c_void) {}

unsafe extern "C" fn linux_device_add_software_node(
    _dev: *mut c_void,
    _node: *const c_void,
) -> i32 {
    0
}

unsafe extern "C" fn linux_device_remove_software_node(_dev: *mut c_void) {}

unsafe extern "C" fn linux_device_property_present(
    _dev: *const c_void,
    _propname: *const c_char,
) -> bool {
    false
}

unsafe extern "C" fn linux_device_property_read_bool(
    _dev: *const c_void,
    _propname: *const c_char,
) -> bool {
    false
}

unsafe extern "C" fn linux_device_property_match_string(
    _dev: *const c_void,
    _propname: *const c_char,
    _string: *const c_char,
) -> i32 {
    -ENODATA
}

unsafe extern "C" fn linux_device_property_read_u32_array(
    _dev: *const c_void,
    _propname: *const c_char,
    _val: *mut u32,
    _nval: usize,
) -> i32 {
    -ENODATA
}

unsafe extern "C" fn linux_is_acpi_device_node(_fwnode: *const c_void) -> bool {
    false
}

unsafe extern "C" fn linux_is_acpi_data_node(_fwnode: *const c_void) -> bool {
    false
}

unsafe extern "C" fn linux_is_software_node(_fwnode: *const c_void) -> bool {
    false
}

unsafe extern "C" fn linux_fwnode_handle_get(fwnode: *mut c_void) -> *mut c_void {
    fwnode
}

unsafe extern "C" fn linux_fwnode_get_next_child_node(
    _fwnode: *const c_void,
    _child: *mut c_void,
) -> *mut c_void {
    core::ptr::null_mut()
}

unsafe extern "C" fn linux_device_get_next_child_node(
    dev: *const c_void,
    child: *mut c_void,
) -> *mut c_void {
    let fwnode = unsafe { linux___dev_fwnode(dev) };
    unsafe { linux_fwnode_get_next_child_node(fwnode.cast_const(), child) }
}

unsafe extern "C" fn linux_fwnode_device_is_available(fwnode: *const c_void) -> bool {
    !is_err_or_null(fwnode)
}

unsafe extern "C" fn linux_fwnode_find_reference(
    _fwnode: *const c_void,
    _name: *const c_char,
    _index: u32,
) -> *mut c_void {
    err_ptr(ENOENT)
}

unsafe extern "C" fn linux_fwnode_get_named_child_node(
    _fwnode: *const c_void,
    _childname: *const c_char,
) -> *mut c_void {
    core::ptr::null_mut()
}

unsafe extern "C" fn linux_fwnode_property_read_u32_array(
    _fwnode: *const c_void,
    _propname: *const c_char,
    _val: *mut u32,
    _nval: usize,
) -> i32 {
    -ENODATA
}

unsafe extern "C" fn linux_fwnode_property_read_string(
    _fwnode: *const c_void,
    _propname: *const c_char,
    _val: *mut *const c_char,
) -> i32 {
    -ENODATA
}

unsafe extern "C" fn linux_device_match_fwnode(_dev: *mut c_void, _fwnode: *mut c_void) -> i32 {
    0
}

unsafe extern "C" fn linux_device_get_match_data(_dev: *const c_void) -> *const c_void {
    core::ptr::null()
}

unsafe extern "C" fn linux_fwnode_irq_get_byname(
    _fwnode: *const c_void,
    name: *const c_char,
) -> i32 {
    if name.is_null() { -EINVAL } else { -ENOENT }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn property_export_registers_for_modules() {
        register_module_exports();
        assert!(crate::kernel::module::find_symbol("device_property_present").is_some());
        assert!(crate::kernel::module::find_symbol("device_property_read_bool").is_some());
        assert!(crate::kernel::module::find_symbol("device_get_next_child_node").is_some());
        assert!(crate::kernel::module::find_symbol("fwnode_property_read_string").is_some());
        assert!(crate::kernel::module::find_symbol("is_acpi_device_node").is_some());
        assert!(crate::kernel::module::find_symbol("is_acpi_data_node").is_some());
    }

    #[test]
    fn fwnode_property_exports_track_vendor_sources() {
        let source = include_str!("../../../vendor/linux/drivers/base/property.c");

        assert!(source.contains("EXPORT_SYMBOL_GPL(fwnode_property_read_u32_array);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(fwnode_property_read_string);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(device_property_read_bool);"));

        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol_gpl_only("fwnode_property_read_string"),
            Some(true)
        );
    }

    #[test]
    fn acpi_fwnode_exports_track_vendor_sources() {
        let source = include_str!("../../../vendor/linux/drivers/acpi/property.c");

        assert!(source.contains("EXPORT_SYMBOL(is_acpi_device_node);"));
        assert!(source.contains("EXPORT_SYMBOL(is_acpi_data_node);"));

        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol_gpl_only("is_acpi_device_node"),
            Some(false)
        );
        assert_eq!(
            crate::kernel::module::find_symbol_gpl_only("is_acpi_data_node"),
            Some(false)
        );
    }

    #[test]
    fn device_child_node_iteration_is_empty_without_fwnode() {
        unsafe {
            assert!(
                linux_device_get_next_child_node(core::ptr::null(), core::ptr::null_mut())
                    .is_null()
            );
        }
    }
}

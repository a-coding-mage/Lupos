//! linux-parity: partial
//! linux-source: vendor/linux/drivers/firmware/dmi_scan.c
//! test-origin: linux:vendor/linux/drivers/firmware/dmi_scan.c
//! x86 DMI firmware table exports.

use core::ffi::{c_char, c_int, c_void};

use crate::kernel::module::{export_symbol, find_symbol};

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once(
        "dmi_get_system_info",
        linux_dmi_get_system_info as usize,
        true,
    );
    export_symbol_once("dmi_first_match", linux_dmi_first_match as usize, true);
    export_symbol_once("dmi_check_system", linux_dmi_check_system as usize, false);
    export_symbol_once("dmi_match", linux_dmi_match as usize, true);
    export_symbol_once("dmi_get_date", linux_dmi_get_date as usize, false);
    export_symbol_once("dmi_get_bios_year", linux_dmi_get_bios_year as usize, false);
    export_symbol_once(
        "dmi_name_in_vendors",
        linux_dmi_name_in_vendors as usize,
        false,
    );
    export_symbol_once("dmi_walk", linux_dmi_walk as usize, true);
    export_symbol_once("dmi_memdev_size", linux_dmi_memdev_size as usize, true);
    export_symbol_once("dmi_memdev_type", linux_dmi_memdev_type as usize, true);
    export_symbol_once("dmi_memdev_handle", linux_dmi_memdev_handle as usize, true);
}

unsafe extern "C" fn linux_dmi_get_system_info(_field: c_int) -> *const c_char {
    core::ptr::null()
}

unsafe extern "C" fn linux_dmi_first_match(_list: *const c_void) -> *const c_void {
    core::ptr::null()
}

unsafe extern "C" fn linux_dmi_check_system(_list: *const c_void) -> c_int {
    0
}

unsafe extern "C" fn linux_dmi_match(_field: c_int, _string: *const c_char) -> bool {
    false
}

unsafe extern "C" fn linux_dmi_get_date(
    _field: c_int,
    year: *mut c_int,
    month: *mut c_int,
    day: *mut c_int,
) -> bool {
    unsafe {
        if !year.is_null() {
            *year = 0;
        }
        if !month.is_null() {
            *month = 0;
        }
        if !day.is_null() {
            *day = 0;
        }
    }
    false
}

unsafe extern "C" fn linux_dmi_get_bios_year() -> c_int {
    -crate::include::uapi::errno::ENXIO
}

unsafe extern "C" fn linux_dmi_name_in_vendors(_str: *const c_char) -> c_int {
    0
}

unsafe extern "C" fn linux_dmi_walk(
    _decode: Option<unsafe extern "C" fn(*const c_void, *mut c_void)>,
    _private_data: *mut c_void,
) -> c_int {
    -crate::include::uapi::errno::ENXIO
}

unsafe extern "C" fn linux_dmi_memdev_size(_handle: u16) -> u64 {
    !0
}

unsafe extern "C" fn linux_dmi_memdev_type(_handle: u16) -> u8 {
    0
}

unsafe extern "C" fn linux_dmi_memdev_handle(_slot: c_int) -> u16 {
    0xffff
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dmi_get_system_info_export_registers_for_modules() {
        register_module_exports();
        assert!(crate::kernel::module::find_symbol("dmi_get_system_info").is_some());
        assert!(crate::kernel::module::find_symbol("dmi_first_match").is_some());
        assert!(crate::kernel::module::find_symbol("dmi_match").is_some());
        assert!(crate::kernel::module::find_symbol("dmi_name_in_vendors").is_some());
        assert!(crate::kernel::module::find_symbol("dmi_walk").is_some());
        assert!(crate::kernel::module::find_symbol("dmi_memdev_size").is_some());
        assert!(crate::kernel::module::find_symbol("dmi_memdev_type").is_some());
        assert!(crate::kernel::module::find_symbol("dmi_memdev_handle").is_some());
    }

    #[test]
    fn dmi_disabled_fallbacks_match_linux_header_defaults() {
        assert_eq!(unsafe { linux_dmi_name_in_vendors(core::ptr::null()) }, 0);
        assert_eq!(
            unsafe { linux_dmi_walk(None, core::ptr::null_mut()) },
            -crate::include::uapi::errno::ENXIO
        );
        assert_eq!(unsafe { linux_dmi_memdev_size(0) }, !0);
        assert_eq!(unsafe { linux_dmi_memdev_type(0) }, 0);
        assert_eq!(unsafe { linux_dmi_memdev_handle(0) }, 0xffff);
    }
}

//! linux-parity: partial
//! linux-source: vendor/linux/drivers/i2c/i2c-boardinfo.c
//! I2C core globals exported by built-in Linux board-info support.

use core::ffi::c_void;

use crate::kernel::module::{export_symbol, find_symbol};

#[repr(C)]
struct LinuxListHead {
    next: *mut c_void,
    prev: *mut c_void,
}

#[repr(C, align(8))]
struct LinuxRwSemaphoreStorage {
    bytes: [u8; 64],
}

static mut I2C_BOARD_LOCK: LinuxRwSemaphoreStorage = LinuxRwSemaphoreStorage { bytes: [0; 64] };
static mut I2C_BOARD_LIST: LinuxListHead = LinuxListHead {
    next: core::ptr::null_mut(),
    prev: core::ptr::null_mut(),
};
static mut I2C_FIRST_DYNAMIC_BUS_NUM: i32 = 0;

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

fn init_i2c_board_list() {
    unsafe {
        let head = core::ptr::addr_of_mut!(I2C_BOARD_LIST);
        if (*head).next.is_null() {
            let self_ptr = head.cast::<c_void>();
            (*head).next = self_ptr;
            (*head).prev = self_ptr;
        }
    }
}

pub fn register_module_exports() {
    init_i2c_board_list();
    export_symbol_once(
        "__i2c_board_lock",
        core::ptr::addr_of_mut!(I2C_BOARD_LOCK) as usize,
        true,
    );
    export_symbol_once(
        "__i2c_board_list",
        core::ptr::addr_of_mut!(I2C_BOARD_LIST) as usize,
        true,
    );
    export_symbol_once(
        "__i2c_first_dynamic_bus_num",
        core::ptr::addr_of_mut!(I2C_FIRST_DYNAMIC_BUS_NUM) as usize,
        true,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn i2c_board_globals_export_empty_initialized_list() {
        register_module_exports();

        unsafe {
            let head = core::ptr::addr_of_mut!(I2C_BOARD_LIST).cast::<c_void>();
            let list = core::ptr::addr_of!(I2C_BOARD_LIST);
            assert_eq!(core::ptr::addr_of!((*list).next).read(), head);
            assert_eq!(core::ptr::addr_of!((*list).prev).read(), head);
        }
        assert!(crate::kernel::module::find_symbol("__i2c_board_lock").is_some());
        assert!(crate::kernel::module::find_symbol("__i2c_board_list").is_some());
        assert!(crate::kernel::module::find_symbol("__i2c_first_dynamic_bus_num").is_some());
    }
}

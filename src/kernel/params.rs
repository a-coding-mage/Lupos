//! linux-parity: partial
//! linux-source: vendor/linux/kernel/params.c
//! test-origin: linux:vendor/linux/kernel/params.c
//! Module parameter ABI exports.

use crate::kernel::module::{export_symbol, find_symbol};

static LINUX_PARAM_OPS_UINT: usize = 0;

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once(
        "param_ops_uint",
        core::ptr::addr_of!(LINUX_PARAM_OPS_UINT) as usize,
        true,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn param_ops_uint_export_registers_for_modules() {
        register_module_exports();
        assert!(crate::kernel::module::find_symbol("param_ops_uint").is_some());
    }
}

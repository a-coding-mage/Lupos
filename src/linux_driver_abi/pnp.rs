//! linux-parity: partial
//! linux-source: vendor/linux/drivers/pnp/resource.c
//! test-origin: linux:vendor/linux/drivers/pnp/resource.c
//! PNP core helper exports used by Linux-built modules.

use crate::kernel::module::{export_symbol, find_symbol};

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once(
        "pnp_range_reserved",
        linux_pnp_range_reserved as usize,
        false,
    );
}

/// `pnp_range_reserved` - `vendor/linux/drivers/pnp/resource.c:687`.
#[unsafe(export_name = "pnp_range_reserved")]
pub extern "C" fn linux_pnp_range_reserved(_start: u64, _end: u64) -> i32 {
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pnp_range_reserved_exports_empty_registry_result() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/drivers/pnp/resource.c"
        ));
        assert!(
            source.contains("int pnp_range_reserved(resource_size_t start, resource_size_t end)")
        );
        assert!(source.contains("EXPORT_SYMBOL(pnp_range_reserved);"));

        register_module_exports();
        assert_eq!(
            find_symbol("pnp_range_reserved"),
            Some(linux_pnp_range_reserved as usize)
        );
        assert_eq!(linux_pnp_range_reserved(0xa0000, 0xbffff), 0);
    }
}

//! linux-parity: complete
//! linux-source: vendor/linux/kernel/module/debug_kmemleak.c
//! test-origin: linux:vendor/linux/kernel/module/debug_kmemleak.c
//! Module kmemleak scan policy.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModuleMemoryType {
    Text,
    Data,
    RoData,
    InitText,
    InitData,
    Other,
}

pub const fn kmemleak_load_module_calls_no_scan(mem_type: ModuleMemoryType, is_rox: bool) -> bool {
    !matches!(
        mem_type,
        ModuleMemoryType::Data | ModuleMemoryType::InitData
    ) && !is_rox
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kmemleak_load_module_source_matches_linux() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/module/debug_kmemleak.c"
        ));
        assert!(source.contains("for_each_mod_mem_type(type)"));
        assert!(source.contains("type != MOD_DATA && type != MOD_INIT_DATA"));
        assert!(source.contains("!mod->mem[type].is_rox"));
        assert!(source.contains("kmemleak_no_scan(mod->mem[type].base);"));
        assert!(!kmemleak_load_module_calls_no_scan(
            ModuleMemoryType::Data,
            false
        ));
        assert!(!kmemleak_load_module_calls_no_scan(
            ModuleMemoryType::InitData,
            false
        ));
        assert!(!kmemleak_load_module_calls_no_scan(
            ModuleMemoryType::Text,
            true
        ));
        assert!(kmemleak_load_module_calls_no_scan(
            ModuleMemoryType::Other,
            false
        ));
    }
}

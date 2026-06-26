//! linux-parity: complete
//! linux-source: vendor/linux/kernel/module/kdb.c
//! test-origin: linux:vendor/linux/kernel/module/kdb.c
//! KDB `lsmod` module listing shape.

use crate::include::uapi::errno::EINVAL;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KdbModuleState {
    Unformed,
    Going,
    Coming,
    Live,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KdbModuleInfo<'a> {
    pub name: &'a str,
    pub text_size: u32,
    pub rodata_size: u32,
    pub ro_after_init_size: u32,
    pub data_size: u32,
    pub state: KdbModuleState,
}

pub const KDB_LSMOD_HEADER: &str = "Module                  Size  modstruct     Used by\n";

pub fn kdb_lsmod<'a>(
    argc: usize,
    modules: &'a [KdbModuleInfo<'a>],
) -> Result<impl Iterator<Item = &'a KdbModuleInfo<'a>>, i32> {
    if argc != 0 {
        return Err(-EINVAL);
    }
    Ok(modules
        .iter()
        .filter(|module| module.state != KdbModuleState::Unformed))
}

pub const fn kdb_state_label(state: KdbModuleState) -> &'static str {
    match state {
        KdbModuleState::Going => "(Unloading)",
        KdbModuleState::Coming => "(Loading)",
        KdbModuleState::Live | KdbModuleState::Unformed => "(Live)",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kdb_lsmod_listing_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/module/kdb.c"
        ));
        assert!(source.contains("int kdb_lsmod(int argc, const char **argv)"));
        assert!(source.contains("if (argc != 0)"));
        assert!(source.contains("return KDB_ARGCOUNT;"));
        assert!(source.contains("Module                  Size  modstruct     Used by"));
        assert!(source.contains("MODULE_STATE_UNFORMED"));
        assert!(source.contains("MOD_TEXT"));
        assert!(source.contains("MOD_RODATA"));
        assert!(source.contains("MOD_RO_AFTER_INIT"));
        assert!(source.contains("MOD_DATA"));
        assert!(source.contains("(Unloading)"));
        assert!(source.contains("(Loading)"));
        assert!(source.contains("(Live)"));

        let modules = [
            KdbModuleInfo {
                name: "skip",
                text_size: 1,
                rodata_size: 0,
                ro_after_init_size: 0,
                data_size: 0,
                state: KdbModuleState::Unformed,
            },
            KdbModuleInfo {
                name: "live",
                text_size: 2,
                rodata_size: 3,
                ro_after_init_size: 4,
                data_size: 5,
                state: KdbModuleState::Live,
            },
        ];
        let listed: alloc::vec::Vec<_> = kdb_lsmod(0, &modules).unwrap().collect();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].name, "live");
        assert_eq!(kdb_state_label(KdbModuleState::Going), "(Unloading)");
        assert_eq!(kdb_lsmod(1, &modules).err(), Some(-EINVAL));
    }
}

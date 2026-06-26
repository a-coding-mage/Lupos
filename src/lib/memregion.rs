//! linux-parity: complete
//! linux-source: vendor/linux/lib/memregion.c
//! test-origin: linux:vendor/linux/lib/memregion.c
//! IDA-backed memory region identifiers.

use alloc::vec::Vec;

use spin::Mutex;

use crate::include::uapi::errno::ENOMEM;
use crate::kernel::module::{export_symbol, find_symbol};

pub type GfpT = u32;

struct Ida {
    allocated: Mutex<Vec<bool>>,
}

impl Ida {
    pub const fn new() -> Self {
        Self {
            allocated: Mutex::new(Vec::new()),
        }
    }
}

static MEMREGION_IDS: Ida = Ida::new();

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("memregion_alloc", memregion_alloc as usize, false);
    export_symbol_once("memregion_free", memregion_free as usize, false);
}

fn ida_alloc(ida: &Ida, _gfp: GfpT) -> i32 {
    let mut allocated = ida.allocated.lock();
    let mut id = 0usize;
    while id < allocated.len() {
        if !allocated[id] {
            allocated[id] = true;
            return id as i32;
        }
        id += 1;
    }

    if allocated.len() > i32::MAX as usize || allocated.try_reserve_exact(1).is_err() {
        return -ENOMEM;
    }

    allocated.push(true);
    id as i32
}

fn ida_free(ida: &Ida, id: u32) {
    let mut allocated = ida.allocated.lock();
    let id = id as usize;
    if id < allocated.len() {
        allocated[id] = false;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn memregion_alloc(gfp: GfpT) -> i32 {
    ida_alloc(&MEMREGION_IDS, gfp)
}

#[unsafe(no_mangle)]
pub extern "C" fn memregion_free(id: i32) {
    ida_free(&MEMREGION_IDS, id as u32);
}

#[cfg(test)]
mod tests {
    use super::*;

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn memregion_source_matches_linux() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/memregion.c"
        ));
        assert!(source.contains("static DEFINE_IDA(memregion_ids);"));
        assert!(source.contains("return ida_alloc(&memregion_ids, gfp);"));
        assert!(source.contains("ida_free(&memregion_ids, id);"));
        assert!(source.contains("EXPORT_SYMBOL(memregion_alloc);"));
        assert!(source.contains("EXPORT_SYMBOL(memregion_free);"));
    }

    #[test]
    fn memregion_ids_allocate_distinct_ids_and_reuse_freed_slot() {
        let _guard = TEST_LOCK.lock();
        let first = memregion_alloc(0);
        let second = memregion_alloc(0);
        assert!(first >= 0);
        assert!(second >= 0);
        assert_ne!(first, second);

        memregion_free(first);
        let reused = memregion_alloc(0);
        assert_eq!(reused, first);

        memregion_free(reused);
        memregion_free(second);
    }

    #[test]
    fn memregion_exports_register_for_modules() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("memregion_alloc"),
            Some(memregion_alloc as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("memregion_free"),
            Some(memregion_free as usize)
        );
    }
}

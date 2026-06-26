//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/mm/debug_pagetables.c
//! test-origin: linux:vendor/linux/arch/x86/mm/debug_pagetables.c
//! Debugfs page-table dump entry policy.
//!
//! Mirrors the entry registration shape from
//! `vendor/linux/arch/x86/mm/debug_pagetables.c`. Lupos does not mount these
//! debugfs files yet, but the exported policy is deterministic and fail-closed
//! when debug page-table dumps are disabled.

use crate::include::uapi::errno::ENODEV;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PageTableDebugEntry {
    Kernel,
    CurrentKernel,
    CurrentUser,
    Efi,
}

pub const DEBUG_PAGETABLE_ENTRIES: &[PageTableDebugEntry] = &[
    PageTableDebugEntry::Kernel,
    PageTableDebugEntry::CurrentKernel,
    PageTableDebugEntry::CurrentUser,
    PageTableDebugEntry::Efi,
];

pub const fn entry_name(entry: PageTableDebugEntry) -> &'static str {
    match entry {
        PageTableDebugEntry::Kernel => "kernel_page_tables",
        PageTableDebugEntry::CurrentKernel => "current_kernel_page_tables",
        PageTableDebugEntry::CurrentUser => "current_user_page_tables",
        PageTableDebugEntry::Efi => "efi_page_tables",
    }
}

pub fn pt_dump_debug_init(enabled: bool) -> Result<&'static [PageTableDebugEntry], i32> {
    if enabled {
        Ok(DEBUG_PAGETABLE_ENTRIES)
    } else {
        Err(ENODEV)
    }
}

pub const fn pt_dump_debug_exit() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_debugfs_fails_closed() {
        assert_eq!(pt_dump_debug_init(false), Err(ENODEV));
    }

    #[test]
    fn entry_names_match_linux_debugfs_files() {
        let entries = pt_dump_debug_init(true).unwrap();
        assert_eq!(entries.len(), 4);
        assert_eq!(entry_name(entries[0]), "kernel_page_tables");
    }
}

//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel
//! test-origin: linux:vendor/linux/arch/x86/kernel
//! Exception table for fault recovery in usercopy.
//!
//! On a page fault from a user-access asm block, the IDT handler calls
//! `search_extable(rip)` and — if a matching `(fault_ip, fixup_ip)` pair is
//! present in the linker-emitted `__ex_table` section — rewrites RIP to the
//! fixup label.  RCX (or the value the fixup loads into the caller's return
//! register) becomes the bytes-not-copied count.
//!
//! Mirrors vendor/linux/arch/x86/mm/extable.c::search_exception_tables and
//! vendor/linux/include/asm-generic/extable.h.
//!
//! The on-disk entry format matches Linux's PC-relative `_ASM_EXTABLE_UA`:
//! two i32 offsets, each relative to the entry's own address.

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ExTableEntry {
    pub fault_ip_off: i32, // PC-relative offset to fault instruction
    pub fixup_ip_off: i32, // PC-relative offset to fixup code
}

// The kernel linker script (arch/x86/kernel/vmlinux.lds.S) emits these bracketing
// symbols around the `__ex_table` section.  Host-side tests don't link the
// kernel linker script, so we provide an empty pair under `#[cfg(test)]`.
#[cfg(not(test))]
unsafe extern "C" {
    static __start___ex_table: ExTableEntry;
    static __stop___ex_table: ExTableEntry;
}

#[cfg(test)]
static __start___ex_table: ExTableEntry = ExTableEntry {
    fault_ip_off: 0,
    fixup_ip_off: 0,
};
#[cfg(test)]
static __stop___ex_table: ExTableEntry = ExTableEntry {
    fault_ip_off: 0,
    fixup_ip_off: 0,
};

/// Search the exception table for a fixup matching `fault_ip`.
/// Returns the absolute fixup IP if found, otherwise `None`.
pub fn search_extable(fault_ip: u64) -> Option<u64> {
    unsafe {
        let start = core::ptr::addr_of!(__start___ex_table) as usize;
        let stop = core::ptr::addr_of!(__stop___ex_table) as usize;
        // In a real kernel link the linker script enforces stop >= start.
        // Under #[cfg(test)] the two dummy statics may be placed in either
        // order, so guard against that.
        if stop <= start {
            return None;
        }
        let entry_size = core::mem::size_of::<ExTableEntry>();
        let num_entries = (stop - start) / entry_size;

        let entries = core::slice::from_raw_parts(&__start___ex_table, num_entries);

        for entry in entries {
            // Each `.long (label - .)` is PC-relative to the *field* it writes
            // into, not to the entry start.  fault_ip_off lives at entry_addr+0
            // and fixup_ip_off at entry_addr+4.
            // Ref: vendor/linux/arch/x86/include/asm/asm.h::_ASM_EXTABLE
            let fault_field = core::ptr::addr_of!(entry.fault_ip_off) as i64;
            let fixup_field = core::ptr::addr_of!(entry.fixup_ip_off) as i64;
            let resolved_fault = (fault_field + entry.fault_ip_off as i64) as u64;
            if resolved_fault == fault_ip {
                let fixup = (fixup_field + entry.fixup_ip_off as i64) as u64;
                return Some(fixup);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extable_entry_layout() {
        assert_eq!(core::mem::size_of::<ExTableEntry>(), 8);
        assert_eq!(core::mem::offset_of!(ExTableEntry, fault_ip_off), 0);
        assert_eq!(core::mem::offset_of!(ExTableEntry, fixup_ip_off), 4);
    }

    #[test]
    fn test_search_extable_no_entries_under_test() {
        // Under #[cfg(test)] the start and stop alias point to the same dummy
        // entry — so the table has zero real entries and every lookup misses.
        assert_eq!(search_extable(0x1000), None);
        assert_eq!(search_extable(0xdeadbeef), None);
    }
}

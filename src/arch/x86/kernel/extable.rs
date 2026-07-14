//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/mm/extable.c
//! test-origin: linux:vendor/linux/arch/x86/mm/extable.c
//! x86 exception-table lookup for built-in and module text.
//!
//! Vendor x86 uses three 32-bit fields per entry: faulting instruction,
//! fixup instruction, and handler data.  The first two are PC-relative to
//! their own fields; `data` selects the fixup handler.  Lupos currently uses
//! the default redirect behavior for the module/usercopy cases it supports,
//! while preserving the full 12-byte record so sorting and future handler
//! dispatch match Linux.

extern crate alloc;

use alloc::vec::Vec;

pub const EXTABLE_ENTRY_SIZE: usize = core::mem::size_of::<ExTableEntry>();

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ExTableEntry {
    pub insn: i32,
    pub fixup: i32,
    pub data: i32,
}

// The kernel linker script emits these bracketing symbols around `__ex_table`.
#[cfg(not(test))]
unsafe extern "C" {
    static __start___ex_table: ExTableEntry;
    static __stop___ex_table: ExTableEntry;
}

#[cfg(test)]
static __start___ex_table: ExTableEntry = ExTableEntry {
    insn: 0,
    fixup: 0,
    data: 0,
};
#[cfg(test)]
static __stop___ex_table: ExTableEntry = ExTableEntry {
    insn: 0,
    fixup: 0,
    data: 0,
};

fn read_i32(data: &[u8], offset: usize) -> Option<i32> {
    Some(i32::from_le_bytes(
        data.get(offset..offset + 4)?.try_into().ok()?,
    ))
}

fn write_i32(data: &mut [u8], offset: usize, value: i32) -> Option<()> {
    data.get_mut(offset..offset + 4)?
        .copy_from_slice(&value.to_le_bytes());
    Some(())
}

fn relative_addr(field_addr: usize, displacement: i32) -> usize {
    (field_addr as isize).wrapping_add(displacement as isize) as usize
}

fn entry_from_bytes(data: &[u8], offset: usize) -> Option<ExTableEntry> {
    Some(ExTableEntry {
        insn: read_i32(data, offset)?,
        fixup: read_i32(data, offset + 4)?,
        data: read_i32(data, offset + 8)?,
    })
}

fn write_entry(data: &mut [u8], offset: usize, entry: ExTableEntry) -> Option<()> {
    write_i32(data, offset, entry.insn)?;
    write_i32(data, offset + 4, entry.fixup)?;
    write_i32(data, offset + 8, entry.data)
}

fn entry_abs_from_bytes(base: usize, index: usize, data: &[u8]) -> Option<(usize, usize, i32)> {
    let offset = index.checked_mul(EXTABLE_ENTRY_SIZE)?;
    let entry = entry_from_bytes(data, offset)?;
    let entry_addr = base.checked_add(offset)?;
    Some((
        relative_addr(entry_addr, entry.insn),
        relative_addr(entry_addr + 4, entry.fixup),
        entry.data,
    ))
}

pub fn sort_extable_bytes(data: &mut [u8]) -> Result<(), i32> {
    if data.len() % EXTABLE_ENTRY_SIZE != 0 {
        return Err(crate::include::uapi::errno::EINVAL);
    }
    let base = data.as_ptr() as usize;
    let entries = data.len() / EXTABLE_ENTRY_SIZE;
    let mut absolute = Vec::with_capacity(entries);
    for index in 0..entries {
        absolute.push(
            entry_abs_from_bytes(base, index, data).ok_or(crate::include::uapi::errno::EINVAL)?,
        );
    }
    absolute.sort_by(|left, right| left.0.cmp(&right.0));
    for (index, (insn, fixup, entry_data)) in absolute.into_iter().enumerate() {
        let offset = index * EXTABLE_ENTRY_SIZE;
        let entry_addr = base + offset;
        let insn_delta = (insn as isize).wrapping_sub(entry_addr as isize);
        let fixup_delta = (fixup as isize).wrapping_sub((entry_addr + 4) as isize);
        let entry = ExTableEntry {
            insn: insn_delta as i32,
            fixup: fixup_delta as i32,
            data: entry_data,
        };
        write_entry(data, offset, entry).ok_or(crate::include::uapi::errno::EINVAL)?;
    }
    Ok(())
}

pub fn search_extable_slice(data: &[u8], fault_ip: u64) -> Option<u64> {
    if data.len() % EXTABLE_ENTRY_SIZE != 0 {
        return None;
    }
    let base = data.as_ptr() as usize;
    let mut left = 0usize;
    let mut right = data.len() / EXTABLE_ENTRY_SIZE;
    let target = usize::try_from(fault_ip).ok()?;
    while left < right {
        let mid = left + (right - left) / 2;
        let (insn, fixup, _entry_data) = entry_abs_from_bytes(base, mid, data)?;
        match insn.cmp(&target) {
            core::cmp::Ordering::Less => left = mid + 1,
            core::cmp::Ordering::Greater => right = mid,
            core::cmp::Ordering::Equal => return Some(fixup as u64),
        }
    }
    None
}

/// Search the built-in and loaded-module exception tables.
pub fn search_extable(fault_ip: u64) -> Option<u64> {
    unsafe {
        let start = core::ptr::addr_of!(__start___ex_table) as usize;
        let stop = core::ptr::addr_of!(__stop___ex_table) as usize;
        if stop > start {
            let len = stop - start;
            let bytes = core::slice::from_raw_parts(start as *const u8, len);
            if let Some(fixup) = search_extable_slice(bytes, fault_ip) {
                return Some(fixup);
            }
        }
    }
    crate::kernel::module::loader::search_module_extable_fixup(fault_ip)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_entry_abs(data: &mut [u8], index: usize, insn: usize, fixup: usize, entry_data: i32) {
        let offset = index * EXTABLE_ENTRY_SIZE;
        let base = data.as_ptr() as usize + offset;
        let insn_delta = (insn as isize).wrapping_sub(base as isize) as i32;
        let fixup_delta = (fixup as isize).wrapping_sub((base + 4) as isize) as i32;
        write_entry(
            data,
            offset,
            ExTableEntry {
                insn: insn_delta,
                fixup: fixup_delta,
                data: entry_data,
            },
        )
        .unwrap();
    }

    #[test]
    fn test_extable_entry_layout() {
        assert_eq!(EXTABLE_ENTRY_SIZE, 12);
        assert_eq!(core::mem::offset_of!(ExTableEntry, insn), 0);
        assert_eq!(core::mem::offset_of!(ExTableEntry, fixup), 4);
        assert_eq!(core::mem::offset_of!(ExTableEntry, data), 8);
    }

    #[test]
    fn module_extable_sort_and_search_preserves_data() {
        let mut table = [0u8; EXTABLE_ENTRY_SIZE * 2];
        let base = table.as_ptr() as usize;
        write_entry_abs(&mut table, 0, base + 0x80, base + 0x100, 3);
        write_entry_abs(&mut table, 1, base + 0x40, base + 0x120, 5);

        sort_extable_bytes(&mut table).unwrap();

        assert_eq!(
            search_extable_slice(&table, (base + 0x40) as u64),
            Some((base + 0x120) as u64)
        );
        assert_eq!(
            search_extable_slice(&table, (base + 0x80) as u64),
            Some((base + 0x100) as u64)
        );
        assert_eq!(entry_from_bytes(&table, 0).unwrap().data, 5);
        assert_eq!(
            entry_from_bytes(&table, EXTABLE_ENTRY_SIZE).unwrap().data,
            3
        );
    }

    #[test]
    fn test_search_extable_no_entries_under_test() {
        assert_eq!(search_extable(0x1000), None);
    }
}

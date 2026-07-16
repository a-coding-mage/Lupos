//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/kernel/unwind_orc.c
//! linux-source: vendor/linux/arch/x86/include/asm/orc_types.h
//! x86 ORC module-table registration and lookup.
//!
//! `unwind_module_init()` below follows the module-specific path in
//! `vendor/linux/arch/x86/kernel/unwind_orc.c`: the PREL32 IP table is sorted
//! together with its packed ORC table, then retained for module-address
//! lookup until `OrcModuleRegistration` is dropped.

#![allow(dead_code)]

extern crate alloc;

use alloc::vec::Vec;
use core::sync::atomic::{AtomicUsize, Ordering};

use spin::Mutex;

use crate::include::uapi::errno::EINVAL;

pub const ORC_ENTRY_SIZE: usize = 6;
pub const ORC_IP_ENTRY_SIZE: usize = core::mem::size_of::<i32>();
/// `ORC_HASH` generated from the vendored x86 `struct orc_entry` definition.
/// A module carrying a different hash was built for a different packed
/// unwinder ABI and must not be interpreted with this decoder.
pub const VENDOR_ORC_HASH: [u8; 20] = [
    0x13, 0x7c, 0x36, 0xad, 0x76, 0x27, 0xc4, 0xdd, 0x55, 0x93, 0x21, 0x76, 0xbb, 0x0e, 0xcd,
    0xbd, 0x50, 0x41, 0xfc, 0x82,
];

// vendor/linux/arch/x86/include/asm/orc_types.h
pub const ORC_REG_UNDEFINED: u8 = 0;
pub const ORC_REG_AX: u8 = 1;
pub const ORC_REG_DX: u8 = 2;
pub const ORC_REG_SP: u8 = 3;
pub const ORC_REG_BP: u8 = 4;
pub const ORC_REG_DI: u8 = 5;
pub const ORC_REG_R10: u8 = 6;
pub const ORC_REG_R13: u8 = 7;
pub const ORC_REG_PREV_SP: u8 = 8;
pub const ORC_REG_SP_INDIRECT: u8 = 9;
pub const ORC_REG_BP_INDIRECT: u8 = 10;
pub const ORC_REG_MAX: u8 = 15;

pub const ORC_TYPE_UNDEFINED: u8 = 0;
pub const ORC_TYPE_END_OF_STACK: u8 = 1;
pub const ORC_TYPE_CALL: u8 = 2;
pub const ORC_TYPE_REGS: u8 = 3;
pub const ORC_TYPE_REGS_PARTIAL: u8 = 4;

/// Byte-exact decoded form of Linux `struct orc_entry` (six packed bytes).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RawOrcEntry {
    pub sp_offset: i16,
    pub bp_offset: i16,
    pub sp_reg: u8,
    pub bp_reg: u8,
    pub type_: u8,
    pub signal: bool,
}

impl RawOrcEntry {
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        let bytes = bytes.get(..ORC_ENTRY_SIZE)?;
        Some(Self {
            sp_offset: i16::from_le_bytes([bytes[0], bytes[1]]),
            bp_offset: i16::from_le_bytes([bytes[2], bytes[3]]),
            sp_reg: bytes[4] & 0x0f,
            bp_reg: bytes[4] >> 4,
            type_: bytes[5] & 0x07,
            signal: bytes[5] & 0x08 != 0,
        })
    }

    pub fn encode(self) -> [u8; ORC_ENTRY_SIZE] {
        let mut out = [0u8; ORC_ENTRY_SIZE];
        out[0..2].copy_from_slice(&self.sp_offset.to_le_bytes());
        out[2..4].copy_from_slice(&self.bp_offset.to_le_bytes());
        out[4] = (self.sp_reg & 0x0f) | ((self.bp_reg & 0x0f) << 4);
        out[5] = (self.type_ & 0x07) | if self.signal { 0x08 } else { 0 };
        out
    }
}

struct RegisteredOrcTable {
    id: usize,
    ip_table: usize,
    unwind_table: usize,
    num_entries: usize,
    text_ranges: Vec<(usize, usize)>,
}

static NEXT_ORC_TABLE_ID: AtomicUsize = AtomicUsize::new(1);
static MODULE_ORC_TABLES: Mutex<Vec<RegisteredOrcTable>> = Mutex::new(Vec::new());

/// Owns Linux's `mod->arch.orc_unwind{,_ip}` registration lifetime.
///
/// The x86 module owner stores this object before its section allocations, so
/// Drop mirrors `module_arch_cleanup()` ordering and removes lookup visibility
/// before the table memory is released.
#[derive(Debug)]
pub struct OrcModuleRegistration {
    id: usize,
    pub orc_unwind_ip: usize,
    pub orc_unwind: usize,
    pub num_orcs: usize,
}

impl Drop for OrcModuleRegistration {
    fn drop(&mut self) {
        self.unregister();
    }
}

impl OrcModuleRegistration {
    /// Remove this table from global ORC lookup visibility without requiring
    /// the owning module descriptor to be dropped.  Linux performs this from
    /// `module_arch_cleanup()` during `delete_module()`; callers may still
    /// retain a reference to the dead module descriptor at that point.
    pub fn unregister(&self) {
        let mut tables = MODULE_ORC_TABLES.lock();
        if let Some(index) = tables.iter().position(|table| table.id == self.id) {
            tables.remove(index);
        }
    }
}

fn read_ip_entry(bytes: &[u8], index: usize) -> Option<i32> {
    let offset = index.checked_mul(ORC_IP_ENTRY_SIZE)?;
    Some(i32::from_le_bytes(
        bytes
            .get(offset..offset + ORC_IP_ENTRY_SIZE)?
            .try_into()
            .ok()?,
    ))
}

fn absolute_ip(table_base: usize, index: usize, displacement: i32) -> usize {
    table_base
        .wrapping_add(index * ORC_IP_ENTRY_SIZE)
        .wrapping_add_signed(displacement as isize)
}

/// Sort a module's paired ORC tables exactly like Linux
/// `orc_sort_cmp()`/`orc_sort_swap()`.
///
/// For duplicate IPs, `ORC_TYPE_UNDEFINED` (weak section terminators) sorts
/// before real entries, allowing the rightmost-match lookup to select the
/// real entry.
pub fn sort_module_orc_tables(
    ip_table_base: usize,
    ip_table: &mut [u8],
    unwind_table: &mut [u8],
) -> Result<usize, i32> {
    if ip_table.len() % ORC_IP_ENTRY_SIZE != 0
        || unwind_table.len() % ORC_ENTRY_SIZE != 0
        || ip_table.len() / ORC_IP_ENTRY_SIZE != unwind_table.len() / ORC_ENTRY_SIZE
    {
        return Err(EINVAL);
    }

    let num_entries = ip_table.len() / ORC_IP_ENTRY_SIZE;
    let mut entries = Vec::with_capacity(num_entries);
    for index in 0..num_entries {
        let displacement = read_ip_entry(ip_table, index).ok_or(EINVAL)?;
        let ip = absolute_ip(ip_table_base, index, displacement);
        let offset = index * ORC_ENTRY_SIZE;
        let raw = RawOrcEntry::decode(&unwind_table[offset..]).ok_or(EINVAL)?;
        entries.push((ip, raw));
    }

    entries.sort_by(|(ip_a, orc_a), (ip_b, orc_b)| {
        ip_a.cmp(ip_b).then_with(|| {
            let a_weak = orc_a.type_ == ORC_TYPE_UNDEFINED;
            let b_weak = orc_b.type_ == ORC_TYPE_UNDEFINED;
            b_weak.cmp(&a_weak)
        })
    });

    for (index, (ip, raw)) in entries.into_iter().enumerate() {
        let entry_addr = ip_table_base.wrapping_add(index * ORC_IP_ENTRY_SIZE);
        let relative = ip as i128 - entry_addr as i128;
        if !(i32::MIN as i128..=i32::MAX as i128).contains(&relative) {
            return Err(EINVAL);
        }
        let displacement = relative as i32;
        let ip_offset = index * ORC_IP_ENTRY_SIZE;
        ip_table[ip_offset..ip_offset + ORC_IP_ENTRY_SIZE]
            .copy_from_slice(&displacement.to_le_bytes());
        let orc_offset = index * ORC_ENTRY_SIZE;
        unwind_table[orc_offset..orc_offset + ORC_ENTRY_SIZE].copy_from_slice(&raw.encode());
    }

    Ok(num_entries)
}

/// Linux `unwind_module_init()` for already-relocated module sections.
pub fn unwind_module_init(
    ip_table_base: usize,
    ip_table: &mut [u8],
    unwind_table_base: usize,
    unwind_table: &mut [u8],
) -> Result<Option<OrcModuleRegistration>, i32> {
    let num_entries = sort_module_orc_tables(ip_table_base, ip_table, unwind_table)?;
    register_sorted_module_orc_tables(
        ip_table_base,
        ip_table,
        unwind_table_base,
        unwind_table,
        num_entries,
    )
}

/// Register tables already sorted by `sort_module_orc_tables()`. This split
/// lets the module finalizer update two independently owned ELF sections
/// without manufacturing overlapping Rust borrows.
pub fn register_sorted_module_orc_tables(
    ip_table_base: usize,
    ip_table: &[u8],
    unwind_table_base: usize,
    unwind_table: &[u8],
    num_entries: usize,
) -> Result<Option<OrcModuleRegistration>, i32> {
    let first_ip = if num_entries == 0 {
        0
    } else {
        absolute_ip(ip_table_base, 0, read_ip_entry(ip_table, 0).ok_or(EINVAL)?)
    };
    let last_ip = if num_entries == 0 {
        0
    } else {
        absolute_ip(
            ip_table_base,
            num_entries - 1,
            read_ip_entry(ip_table, num_entries - 1).ok_or(EINVAL)?,
        )
    };
    register_sorted_module_orc_tables_for_range(
        ip_table_base,
        ip_table,
        unwind_table_base,
        unwind_table,
        num_entries,
        first_ip,
        last_ip.saturating_add(1),
    )
}

/// Register sorted ORC tables with the owning module's executable-text range,
/// matching the `__module_address(ip)` guard in Linux `orc_module_find()`.
pub fn register_sorted_module_orc_tables_for_range(
    ip_table_base: usize,
    ip_table: &[u8],
    unwind_table_base: usize,
    unwind_table: &[u8],
    num_entries: usize,
    text_start: usize,
    text_end: usize,
) -> Result<Option<OrcModuleRegistration>, i32> {
    register_sorted_module_orc_tables_for_ranges(
        ip_table_base,
        ip_table,
        unwind_table_base,
        unwind_table,
        num_entries,
        &[(text_start, text_end)],
    )
}

pub fn register_sorted_module_orc_tables_for_ranges(
    ip_table_base: usize,
    ip_table: &[u8],
    unwind_table_base: usize,
    unwind_table: &[u8],
    num_entries: usize,
    text_ranges: &[(usize, usize)],
) -> Result<Option<OrcModuleRegistration>, i32> {
    if ip_table.len() != num_entries * ORC_IP_ENTRY_SIZE
        || unwind_table.len() != num_entries * ORC_ENTRY_SIZE
    {
        return Err(EINVAL);
    }
    if num_entries == 0 {
        return Ok(None);
    }

    if text_ranges.is_empty() || text_ranges.iter().any(|(start, end)| start >= end) {
        return Err(EINVAL);
    }
    let id = NEXT_ORC_TABLE_ID.fetch_add(1, Ordering::Relaxed);
    MODULE_ORC_TABLES.lock().push(RegisteredOrcTable {
        id,
        ip_table: ip_table_base,
        unwind_table: unwind_table_base,
        num_entries,
        text_ranges: text_ranges.to_vec(),
    });
    Ok(Some(OrcModuleRegistration {
        id,
        orc_unwind_ip: ip_table_base,
        orc_unwind: unwind_table_base,
        num_orcs: num_entries,
    }))
}

fn find_index(table: &RegisteredOrcTable, ip: usize) -> Option<usize> {
    if table.num_entries == 0
        || !table
            .text_ranges
            .iter()
            .any(|(start, end)| (*start..*end).contains(&ip))
    {
        return None;
    }
    let bytes = unsafe {
        core::slice::from_raw_parts(
            table.ip_table as *const u8,
            table.num_entries * ORC_IP_ENTRY_SIZE,
        )
    };
    let mut first = 0usize;
    let mut last = table.num_entries;
    while first < last {
        let mid = first + (last - first) / 2;
        let mid_ip = absolute_ip(table.ip_table, mid, read_ip_entry(bytes, mid)?);
        if mid_ip <= ip {
            first = mid + 1;
        } else {
            last = mid;
        }
    }
    first.checked_sub(1)
}

/// Linux `orc_module_find()`: rightmost ORC entry whose generated IP does not
/// exceed `ip`.
pub fn orc_module_find(ip: usize) -> Option<RawOrcEntry> {
    let tables = MODULE_ORC_TABLES.lock();
    for table in tables.iter() {
        let Some(index) = find_index(table, ip) else {
            continue;
        };
        let bytes = unsafe {
            core::slice::from_raw_parts(
                table.unwind_table as *const u8,
                table.num_entries * ORC_ENTRY_SIZE,
            )
        };
        return RawOrcEntry::decode(&bytes[index * ORC_ENTRY_SIZE..]);
    }
    None
}

/// Live register state for walking packed module ORC data.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ModuleOrcUnwindState {
    pub ip: usize,
    pub sp: usize,
    pub bp: usize,
    /// Address of Linux's current `struct pt_regs`, when an ORC register frame
    /// has been crossed.  Partial frames point 128 bytes before their IRET
    /// frame, exactly like `state->regs` in the vendor unwinder.
    pub regs: usize,
    pub prev_regs: usize,
    pub full_regs: bool,
    pub signal: bool,
    pub done: bool,
    pub error: bool,
}

fn dereference_word(address: usize) -> Result<usize, i32> {
    if address == 0 || address & (core::mem::align_of::<usize>() - 1) != 0 {
        return Err(EINVAL);
    }
    let mut bytes = [0u8; core::mem::size_of::<usize>()];
    unsafe {
        crate::arch::x86::mm::maccess::copy_from_kernel_nofault(
            bytes.as_mut_ptr(),
            address as *const u8,
            bytes.len(),
        )
    }
    .map_err(|_| EINVAL)?;
    Ok(usize::from_le_bytes(bytes))
}

// x86-64 `struct pt_regs` offsets from vendor/linux/arch/x86/include/asm/ptrace.h.
const PT_REGS_R13: usize = 16;
const PT_REGS_BP: usize = 32;
const PT_REGS_R10: usize = 56;
const PT_REGS_AX: usize = 80;
const PT_REGS_DX: usize = 96;
const PT_REGS_DI: usize = 112;
const PT_REGS_IP: usize = 128;
const PT_REGS_CS: usize = 136;
const PT_REGS_SP: usize = 152;
const PT_REGS_SIZE: usize = 168;
const IRET_FRAME_OFFSET: usize = PT_REGS_IP;
const IRET_FRAME_SIZE: usize = PT_REGS_SIZE - IRET_FRAME_OFFSET;

fn get_saved_reg(state: &ModuleOrcUnwindState, offset: usize) -> Result<usize, i32> {
    let base = if state.regs != 0 && state.full_regs {
        state.regs
    } else if state.prev_regs != 0 {
        state.prev_regs
    } else {
        return Err(EINVAL);
    };
    dereference_word(base.checked_add(offset).ok_or(EINVAL)?)
}

fn saved_regs_are_user(state: &ModuleOrcUnwindState) -> Result<bool, i32> {
    if state.regs == 0 {
        return Ok(false);
    }
    Ok(dereference_word(state.regs.checked_add(PT_REGS_CS).ok_or(EINVAL)?)? & 3 != 0)
}

fn dereference_full_regs(address: usize) -> Result<(usize, usize), i32> {
    let end = address.checked_add(PT_REGS_SIZE).ok_or(EINVAL)?;
    if end <= address {
        return Err(EINVAL);
    }
    Ok((
        dereference_word(address + PT_REGS_IP)?,
        dereference_word(address + PT_REGS_SP)?,
    ))
}

fn dereference_iret_regs(address: usize) -> Result<(usize, usize), i32> {
    let end = address.checked_add(IRET_FRAME_SIZE).ok_or(EINVAL)?;
    if end <= address {
        return Err(EINVAL);
    }
    Ok((
        dereference_word(address)?,
        dereference_word(address + (PT_REGS_SP - IRET_FRAME_OFFSET))?,
    ))
}

/// Walk one frame using the registered packed Linux ORC table.
///
/// This is the module half of Linux `unwind_next_frame()`: lookup uses `ip-1`
/// for call frames, evaluates SP/BP base encodings, dereferences the saved
/// return address, and rejects non-progressing stacks. Register-frame entries
/// remain fail-closed because Lupos does not yet expose Linux's byte-exact
/// `pt_regs` layout to this Rust-side state object.
pub fn orc_module_unwind_next(state: &mut ModuleOrcUnwindState) -> Result<bool, i32> {
    if state.done || state.ip == 0 {
        return Ok(false);
    }
    if saved_regs_are_user(state)? {
        state.done = true;
        return Ok(false);
    }
    let lookup_ip = if state.signal {
        state.ip
    } else {
        state.ip.saturating_sub(1)
    };
    let entry = orc_module_find(lookup_ip).ok_or(EINVAL)?;
    if entry.type_ == ORC_TYPE_UNDEFINED {
        state.error = true;
        return Err(EINVAL);
    }
    if entry.type_ == ORC_TYPE_END_OF_STACK {
        state.done = true;
        return Ok(false);
    }

    let previous_sp = state.sp;
    let mut sp = match entry.sp_reg {
        ORC_REG_SP => state.sp.wrapping_add_signed(entry.sp_offset as isize),
        ORC_REG_BP => state.bp.wrapping_add_signed(entry.sp_offset as isize),
        ORC_REG_SP_INDIRECT => {
            let base = dereference_word(state.sp)?;
            base.wrapping_add_signed(entry.sp_offset as isize)
        }
        ORC_REG_BP_INDIRECT => {
            dereference_word(state.bp.wrapping_add_signed(entry.sp_offset as isize))?
        }
        ORC_REG_AX => get_saved_reg(state, PT_REGS_AX)?,
        ORC_REG_DX => get_saved_reg(state, PT_REGS_DX)?,
        ORC_REG_DI => get_saved_reg(state, PT_REGS_DI)?,
        ORC_REG_R10 => get_saved_reg(state, PT_REGS_R10)?,
        ORC_REG_R13 => get_saved_reg(state, PT_REGS_R13)?,
        _ => {
            state.error = true;
            return Err(EINVAL);
        }
    };

    match entry.type_ {
        ORC_TYPE_CALL => {
            state.ip = dereference_word(sp.wrapping_sub(core::mem::size_of::<usize>()))?;
            state.sp = sp;
            state.regs = 0;
            state.prev_regs = 0;
            state.full_regs = false;
        }
        ORC_TYPE_REGS => {
            let (ip, saved_sp) = dereference_full_regs(sp)?;
            state.ip = ip;
            state.sp = saved_sp;
            state.regs = sp;
            state.prev_regs = 0;
            state.full_regs = true;
        }
        ORC_TYPE_REGS_PARTIAL => {
            let (ip, saved_sp) = dereference_iret_regs(sp)?;
            state.ip = ip;
            state.sp = saved_sp;
            if state.full_regs {
                state.prev_regs = state.regs;
            }
            state.regs = sp.checked_sub(IRET_FRAME_OFFSET).ok_or(EINVAL)?;
            state.full_regs = false;
        }
        _ => {
            state.error = true;
            return Err(EINVAL);
        }
    }

    match entry.bp_reg {
        ORC_REG_UNDEFINED => {
            if let Ok(bp) = get_saved_reg(state, PT_REGS_BP) {
                state.bp = bp;
            }
        }
        ORC_REG_PREV_SP => {
            state.bp = dereference_word(sp.wrapping_add_signed(entry.bp_offset as isize))?;
        }
        ORC_REG_BP => {
            state.bp = dereference_word(state.bp.wrapping_add_signed(entry.bp_offset as isize))?;
        }
        _ => {
            state.error = true;
            return Err(EINVAL);
        }
    }
    state.signal = entry.signal;
    if state.ip == 0 {
        state.done = true;
        return Ok(false);
    }
    if state.sp <= previous_sp && entry.type_ == ORC_TYPE_CALL {
        state.error = true;
        return Err(EINVAL);
    }
    Ok(true)
}

pub fn orc_module_walk(mut state: ModuleOrcUnwindState, limit: usize) -> Result<Vec<usize>, i32> {
    let mut frames = Vec::new();
    while frames.len() < limit && !state.done && state.ip != 0 {
        frames.push(state.ip);
        if !orc_module_unwind_next(&mut state)? {
            break;
        }
    }
    Ok(frames)
}

// Compatibility model used by the existing stack-walk callers. Module-table
// registration above is the authoritative packed Linux ABI path.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct OrcEntry {
    pub ip: u64,
    pub sp_offset: i32,
    pub bp_offset: i32,
    pub type_flags: u8,
    pub signal: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct OrcUnwindState {
    pub ip: u64,
    pub sp: u64,
    pub bp: u64,
    pub done: bool,
    pub error: bool,
}

pub fn orc_sort(table: &mut [OrcEntry]) {
    table.sort_by_key(|entry| entry.ip);
}

pub fn orc_find(table: &[OrcEntry], ip: u64) -> Option<OrcEntry> {
    let index = table
        .partition_point(|entry| entry.ip <= ip)
        .checked_sub(1)?;
    table.get(index).copied()
}

pub fn unwind_init(table: &mut [OrcEntry]) {
    orc_sort(table);
}

pub const fn unwind_get_return_address(state: &OrcUnwindState) -> u64 {
    state.ip
}

pub fn unwind_next_frame(
    state: &mut OrcUnwindState,
    table: &[OrcEntry],
    stack: &[u64],
) -> Result<bool, i32> {
    if state.done {
        return Ok(false);
    }
    let entry = orc_find(table, state.ip).ok_or(EINVAL)?;
    let next_sp = state.sp.wrapping_add(entry.sp_offset as i64 as u64);
    let stack_index = (next_sp / 8) as usize;
    let Some(next_ip) = stack.get(stack_index).copied() else {
        state.error = true;
        return Err(EINVAL);
    };
    if next_ip == 0 {
        state.done = true;
        state.ip = 0;
        return Ok(false);
    }
    state.sp = next_sp + 8;
    state.bp = state.bp.wrapping_add(entry.bp_offset as i64 as u64);
    state.ip = next_ip;
    Ok(true)
}

pub fn unwind_all(
    mut state: OrcUnwindState,
    table: &[OrcEntry],
    stack: &[u64],
    limit: usize,
) -> Result<Vec<u64>, i32> {
    let mut out = Vec::new();
    while !state.done && out.len() < limit {
        let ip = unwind_get_return_address(&state);
        if ip == 0 {
            break;
        }
        out.push(ip);
        unwind_next_frame(&mut state, table, stack)?;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fill_registered_raw_entry(
        ip: &mut [u8; ORC_IP_ENTRY_SIZE],
        text_ip: usize,
        entry: RawOrcEntry,
    ) -> [u8; ORC_ENTRY_SIZE] {
        let slot = ip.as_ptr() as usize;
        let displacement = text_ip as i128 - slot as i128;
        assert!((i32::MIN as i128..=i32::MAX as i128).contains(&displacement));
        ip.copy_from_slice(&(displacement as i32).to_le_bytes());
        entry.encode()
    }

    #[test]
    fn orc_lookup_uses_nearest_lower_ip() {
        let mut table = [
            OrcEntry {
                ip: 0x2000,
                sp_offset: 0,
                ..OrcEntry::default()
            },
            OrcEntry {
                ip: 0x1000,
                sp_offset: 8,
                ..OrcEntry::default()
            },
        ];
        unwind_init(&mut table);
        assert_eq!(orc_find(&table, 0x1800).unwrap().ip, 0x1000);
    }

    #[test]
    fn orc_unwind_reads_next_ip_from_stack() {
        let table = [OrcEntry {
            ip: 0x1000,
            sp_offset: 0,
            ..OrcEntry::default()
        }];
        let stack = [0x2000, 0];
        let out = unwind_all(
            OrcUnwindState {
                ip: 0x1000,
                sp: 0,
                ..OrcUnwindState::default()
            },
            &table,
            &stack,
            2,
        )
        .unwrap();
        assert_eq!(out, alloc::vec![0x1000, 0x2000]);
    }

    #[test]
    fn packed_module_orc_call_frame_reads_saved_return_address() {
        let text = [0x90u8; 2];
        let text_ip = text.as_ptr() as usize;
        let mut ip = [0u8; ORC_IP_ENTRY_SIZE];
        let orc = fill_registered_raw_entry(
            &mut ip,
            text_ip,
            RawOrcEntry {
                sp_offset: 8,
                sp_reg: ORC_REG_SP,
                type_: ORC_TYPE_CALL,
                ..RawOrcEntry::default()
            },
        );
        let registration = register_sorted_module_orc_tables_for_range(
            ip.as_ptr() as usize,
            &ip,
            orc.as_ptr() as usize,
            &orc,
            1,
            text_ip,
            text_ip + text.len(),
        )
        .unwrap()
        .unwrap();
        let stack = [0x1234_5678usize];
        let mut state = ModuleOrcUnwindState {
            ip: text_ip + 1,
            sp: stack.as_ptr() as usize,
            ..Default::default()
        };
        assert_eq!(orc_module_unwind_next(&mut state), Ok(true));
        assert_eq!(state.ip, stack[0]);
        assert_eq!(state.sp, stack.as_ptr() as usize + 8);
        drop(registration);
    }

    #[test]
    fn packed_module_orc_full_regs_recovers_ip_sp_bp_and_register_bases() {
        let text = [0x90u8; 2];
        let text_ip = text.as_ptr() as usize;
        let mut ip = [0u8; ORC_IP_ENTRY_SIZE];
        let orc = fill_registered_raw_entry(
            &mut ip,
            text_ip,
            RawOrcEntry {
                sp_reg: ORC_REG_SP,
                bp_reg: ORC_REG_UNDEFINED,
                type_: ORC_TYPE_REGS,
                signal: true,
                ..RawOrcEntry::default()
            },
        );
        let registration = register_sorted_module_orc_tables_for_range(
            ip.as_ptr() as usize,
            &ip,
            orc.as_ptr() as usize,
            &orc,
            1,
            text_ip,
            text_ip + text.len(),
        )
        .unwrap()
        .unwrap();
        let mut regs = [0usize; PT_REGS_SIZE / 8];
        regs[PT_REGS_BP / 8] = 0xaaaa_bbbb;
        regs[PT_REGS_IP / 8] = 0x1111_2222;
        regs[PT_REGS_CS / 8] = 0x8;
        regs[PT_REGS_SP / 8] = regs.as_ptr() as usize + PT_REGS_SIZE + 8;
        let regs_addr = regs.as_ptr() as usize;
        let mut state = ModuleOrcUnwindState {
            ip: text_ip + 1,
            sp: regs_addr,
            ..Default::default()
        };
        assert_eq!(orc_module_unwind_next(&mut state), Ok(true));
        assert_eq!(state.ip, 0x1111_2222);
        assert_eq!(state.bp, 0xaaaa_bbbb);
        assert_eq!(state.regs, regs_addr);
        assert!(state.full_regs);
        assert!(state.signal);
        drop(registration);
    }
}

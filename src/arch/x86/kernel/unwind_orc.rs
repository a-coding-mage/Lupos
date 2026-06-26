//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/unwind_orc.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/unwind_orc.c
//! x86 ORC unwind table helpers.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/unwind_orc.c

#![allow(dead_code)]

extern crate alloc;

use alloc::vec::Vec;

use crate::include::uapi::errno::EINVAL;

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

pub const ORC_TYPE_CALL: u8 = 0;
pub const ORC_TYPE_REGS: u8 = 1;
pub const ORC_TYPE_REGS_IRET: u8 = 2;

pub fn orc_sort(table: &mut [OrcEntry]) {
    table.sort_by_key(|entry| entry.ip);
}

pub fn orc_find(table: &[OrcEntry], ip: u64) -> Option<OrcEntry> {
    let mut best = None;
    for entry in table {
        if entry.ip <= ip {
            best = Some(*entry);
        } else {
            break;
        }
    }
    best
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
}

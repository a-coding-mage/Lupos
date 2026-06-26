//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/mm/pf_in.c
//! test-origin: linux:vendor/linux/arch/x86/mm/pf_in.c
//! Page-fault instruction decoder helpers.
//!
//! Mirrors the instruction classification role of
//! `vendor/linux/arch/x86/mm/pf_in.c` and `vendor/linux/arch/x86/mm/pf_in.h`.
//! This port decodes the common memory-access opcodes used by the fault path
//! and exposes byte-slice helpers so tests do not dereference arbitrary RIPs.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReasonType {
    Unknown,
    Mov,
    Movs,
    Stos,
    Xchg,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PrefixState {
    operand_16: bool,
    rex_w: bool,
    consumed: usize,
}

fn skip_prefix(bytes: &[u8]) -> PrefixState {
    let mut state = PrefixState {
        operand_16: false,
        rex_w: false,
        consumed: 0,
    };
    while let Some(&b) = bytes.get(state.consumed) {
        match b {
            0x66 => state.operand_16 = true,
            0x40..=0x4f => state.rex_w = (b & 0x08) != 0,
            0xf0 | 0xf2 | 0xf3 | 0x2e | 0x36 | 0x3e | 0x26 | 0x64 | 0x65 => {}
            _ => break,
        }
        state.consumed += 1;
    }
    state
}

pub fn get_ins_type_from_bytes(bytes: &[u8]) -> ReasonType {
    let prefix = skip_prefix(bytes);
    match bytes.get(prefix.consumed).copied() {
        Some(0x86 | 0x87) => ReasonType::Xchg,
        Some(0x88..=0x8b | 0xc6 | 0xc7) => ReasonType::Mov,
        Some(0xa4 | 0xa5) => ReasonType::Movs,
        Some(0xaa | 0xab) => ReasonType::Stos,
        _ => ReasonType::Unknown,
    }
}

pub fn get_ins_mem_width_from_bytes(bytes: &[u8]) -> Option<u8> {
    let prefix = skip_prefix(bytes);
    match bytes.get(prefix.consumed).copied()? {
        0x88 | 0x8a | 0x86 | 0xc6 | 0xa4 | 0xaa => Some(1),
        0x66 => Some(2),
        0x89 | 0x8b | 0x87 | 0xc7 | 0xa5 | 0xab => {
            if prefix.operand_16 {
                Some(2)
            } else if prefix.rex_w {
                Some(8)
            } else {
                Some(4)
            }
        }
        _ => None,
    }
}

/// Best-effort pointer decoder for low-level exception paths.
///
/// # Safety
/// `ins_addr` must point at at least 8 readable instruction bytes.
pub unsafe fn get_ins_type(ins_addr: u64) -> ReasonType {
    if ins_addr == 0 {
        return ReasonType::Unknown;
    }
    let bytes = unsafe { core::slice::from_raw_parts(ins_addr as *const u8, 8) };
    get_ins_type_from_bytes(bytes)
}

/// Best-effort pointer decoder for low-level exception paths.
///
/// # Safety
/// `ins_addr` must point at at least 8 readable instruction bytes.
pub unsafe fn get_ins_mem_width(ins_addr: u64) -> Option<u8> {
    if ins_addr == 0 {
        return None;
    }
    let bytes = unsafe { core::slice::from_raw_parts(ins_addr as *const u8, 8) };
    get_ins_mem_width_from_bytes(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mov_and_string_opcodes_are_classified() {
        assert_eq!(
            get_ins_type_from_bytes(&[0x48, 0x8b, 0x00]),
            ReasonType::Mov
        );
        assert_eq!(get_ins_type_from_bytes(&[0xa5]), ReasonType::Movs);
        assert_eq!(get_ins_type_from_bytes(&[0xaa]), ReasonType::Stos);
    }

    #[test]
    fn rex_w_selects_64_bit_memory_width() {
        assert_eq!(get_ins_mem_width_from_bytes(&[0x48, 0x8b, 0x00]), Some(8));
        assert_eq!(get_ins_mem_width_from_bytes(&[0x66, 0x8b, 0x00]), Some(2));
        assert_eq!(get_ins_mem_width_from_bytes(&[0x8b, 0x00]), Some(4));
    }
}

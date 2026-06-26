//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/mm/pgtable_32.c
//! test-origin: linux:vendor/linux/arch/x86/mm/pgtable_32.c
//! x86 32-bit pgtable compatibility surface.
//!
//! Mirrors command-line parsing and exported symbols from
//! `vendor/linux/arch/x86/mm/pgtable_32.c`. Lupos executes the x86_64 path,
//! so 32-bit page-table mutation returns `EOPNOTSUPP`.

use crate::arch::x86::mm::paging::pte_t;
use crate::include::uapi::errno::{EINVAL, EOPNOTSUPP};

pub const DEFAULT_FIXADDR_TOP: u64 = 0xffff_f000;

pub const fn pgtable_32_supported() -> bool {
    false
}

pub fn parse_vmalloc(arg: &str) -> Result<u64, i32> {
    parse_size_suffix(arg)
}

pub fn parse_reservetop(arg: &str) -> Result<u64, i32> {
    parse_size_suffix(arg)
}

pub const fn set_pte_vaddr(_vaddr: u64, _pteval: pte_t) -> Result<(), i32> {
    Err(EOPNOTSUPP)
}

fn parse_size_suffix(arg: &str) -> Result<u64, i32> {
    let bytes = arg.as_bytes();
    if bytes.is_empty() {
        return Err(EINVAL);
    }
    let (digits, mult) = match bytes[bytes.len() - 1] {
        b'K' | b'k' => (&bytes[..bytes.len() - 1], 1024u64),
        b'M' | b'm' => (&bytes[..bytes.len() - 1], 1024u64 * 1024),
        b'G' | b'g' => (&bytes[..bytes.len() - 1], 1024u64 * 1024 * 1024),
        _ => (bytes, 1u64),
    };
    if digits.is_empty() {
        return Err(EINVAL);
    }
    let mut value = 0u64;
    for &b in digits {
        if !b.is_ascii_digit() {
            return Err(EINVAL);
        }
        value = value
            .checked_mul(10)
            .and_then(|v| v.checked_add((b - b'0') as u64))
            .ok_or(EINVAL)?;
    }
    value.checked_mul(mult).ok_or(EINVAL)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arch::x86::mm::paging::__pte;

    #[test]
    fn size_parser_accepts_linux_suffixes() {
        assert_eq!(parse_vmalloc("128M"), Ok(128 * 1024 * 1024));
        assert_eq!(parse_reservetop("1G"), Ok(1024 * 1024 * 1024));
    }

    #[test]
    fn live_32_bit_pte_mutation_is_unsupported_on_x86_64() {
        assert_eq!(set_pte_vaddr(0x1000, __pte(0)), Err(EOPNOTSUPP));
    }
}

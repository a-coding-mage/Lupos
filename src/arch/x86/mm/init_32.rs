//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/mm/init_32.c
//! test-origin: linux:vendor/linux/arch/x86/mm/init_32.c
//! x86 32-bit memory initialization compatibility surface.
//!
//! Mirrors the 32-bit-only gates from `vendor/linux/arch/x86/mm/init_32.c`.
//! Lupos targets x86_64, so operational setup returns `EOPNOTSUPP` while
//! command-line parsing remains available for structural parity.

use crate::include::uapi::errno::{EINVAL, EOPNOTSUPP};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HighmemMode {
    Off,
    Size(u64),
}

pub const fn init_32_supported() -> bool {
    false
}

pub fn parse_highmem(arg: &str) -> Result<HighmemMode, i32> {
    if arg == "off" || arg == "0" {
        return Ok(HighmemMode::Off);
    }
    let size = parse_decimal_mib(arg)?;
    Ok(HighmemMode::Size(size * 1024 * 1024))
}

pub const fn paging_init_32() -> Result<(), i32> {
    Err(EOPNOTSUPP)
}

fn parse_decimal_mib(arg: &str) -> Result<u64, i32> {
    let suffixless = arg
        .strip_suffix('M')
        .or_else(|| arg.strip_suffix('m'))
        .unwrap_or(arg);
    if suffixless.is_empty() {
        return Err(EINVAL);
    }
    let mut value = 0u64;
    for b in suffixless.bytes() {
        if !b.is_ascii_digit() {
            return Err(EINVAL);
        }
        value = value
            .checked_mul(10)
            .and_then(|v| v.checked_add((b - b'0') as u64))
            .ok_or(EINVAL)?;
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn highmem_parser_accepts_off_and_size() {
        assert_eq!(parse_highmem("off"), Ok(HighmemMode::Off));
        assert_eq!(
            parse_highmem("64M"),
            Ok(HighmemMode::Size(64 * 1024 * 1024))
        );
    }

    #[test]
    fn paging_init_32_is_not_available_on_lupos_x86_64() {
        assert_eq!(paging_init_32(), Err(EOPNOTSUPP));
    }
}

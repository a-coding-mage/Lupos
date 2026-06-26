//! linux-parity: complete
//! linux-source: vendor/linux/kernel/elfcorehdr.c
//! test-origin: linux:vendor/linux/kernel/elfcorehdr.c
//! Crash-kernel ELF core header command-line parsing.

use crate::include::uapi::errno::EINVAL;

pub const ELFCORE_ADDR_MAX: u64 = u64::MAX;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ElfCoreHeader {
    pub addr: u64,
    pub size: u64,
}

impl ElfCoreHeader {
    pub const fn empty() -> Self {
        Self {
            addr: ELFCORE_ADDR_MAX,
            size: 0,
        }
    }
}

pub fn setup_elfcorehdr(arg: Option<&str>) -> Result<ElfCoreHeader, i32> {
    let arg = arg.ok_or(-EINVAL)?;
    let (first, consumed) = memparse_prefix(arg).ok_or(-EINVAL)?;
    if consumed == 0 {
        return Err(-EINVAL);
    }

    let rest = &arg[consumed..];
    if let Some(offset_arg) = rest.strip_prefix('@') {
        let (addr, addr_consumed) = memparse_prefix(offset_arg).ok_or(-EINVAL)?;
        if addr_consumed == 0 {
            return Err(-EINVAL);
        }
        return Ok(ElfCoreHeader { addr, size: first });
    }

    Ok(ElfCoreHeader {
        addr: first,
        size: 0,
    })
}

fn memparse_prefix(arg: &str) -> Option<(u64, usize)> {
    let bytes = arg.as_bytes();
    let mut index = 0usize;
    let mut radix = 10u32;

    if bytes.len() >= 2 && bytes[0] == b'0' && matches!(bytes[1], b'x' | b'X') {
        radix = 16;
        index = 2;
    }

    let digits_start = index;
    let mut value = 0u64;
    while let Some(&byte) = bytes.get(index) {
        let digit = match byte {
            b'0'..=b'9' => (byte - b'0') as u32,
            b'a'..=b'f' => 10 + (byte - b'a') as u32,
            b'A'..=b'F' => 10 + (byte - b'A') as u32,
            _ => break,
        };
        if digit >= radix {
            break;
        }
        value = value
            .saturating_mul(radix as u64)
            .saturating_add(digit as u64);
        index += 1;
    }

    if index == digits_start {
        return None;
    }

    if let Some(&suffix) = bytes.get(index) {
        let shift = match suffix {
            b'K' | b'k' => Some(10),
            b'M' | b'm' => Some(20),
            b'G' | b'g' => Some(30),
            _ => None,
        };
        if let Some(shift) = shift {
            value = value.checked_shl(shift).unwrap_or(u64::MAX);
            index += 1;
        }
    }

    Some((value, index))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn elfcorehdr_parser_matches_linux_early_param_shape() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/elfcorehdr.c"
        ));
        assert!(source.contains("unsigned long long elfcorehdr_addr = ELFCORE_ADDR_MAX;"));
        assert!(source.contains("unsigned long long elfcorehdr_size;"));
        assert!(source.contains("elfcorehdr_addr = memparse(arg, &end);"));
        assert!(source.contains("if (*end == '@')"));
        assert!(source.contains("early_param(\"elfcorehdr\", setup_elfcorehdr);"));

        assert_eq!(ElfCoreHeader::empty().addr, ELFCORE_ADDR_MAX);
        assert_eq!(
            setup_elfcorehdr(Some("16M@0x400000")),
            Ok(ElfCoreHeader {
                addr: 0x400000,
                size: 16 << 20,
            })
        );
        assert_eq!(
            setup_elfcorehdr(Some("0x1234")),
            Ok(ElfCoreHeader {
                addr: 0x1234,
                size: 0,
            })
        );
        assert_eq!(setup_elfcorehdr(None), Err(-EINVAL));
        assert_eq!(setup_elfcorehdr(Some("@0x1000")), Err(-EINVAL));
    }
}

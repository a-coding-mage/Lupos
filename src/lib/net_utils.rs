//! linux-parity: complete
//! linux-source: vendor/linux/lib/net_utils.c
//! test-origin: linux:vendor/linux/lib/net_utils.c
//! Network utility helpers.

use crate::kernel::module::{export_symbol, find_symbol};

pub const ETH_ALEN: usize = 6;
pub const MAC_ADDR_STR_LEN: usize = 3 * ETH_ALEN - 1;

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("mac_pton", mac_pton as usize, false);
    export_symbol_once("in4_pton", in4_pton as usize, false);
    export_symbol_once("in6_pton", in6_pton as usize, false);
}

fn hex_to_bin(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

pub fn mac_pton_bytes(input: &[u8], mac: &mut [u8; ETH_ALEN]) -> bool {
    if input.len() < MAC_ADDR_STR_LEN {
        return false;
    }

    let mut index = 0usize;
    while index < ETH_ALEN {
        if hex_to_bin(input[index * 3]).is_none() || hex_to_bin(input[index * 3 + 1]).is_none() {
            return false;
        }
        if index != ETH_ALEN - 1 && input[index * 3 + 2] != b':' {
            return false;
        }
        index += 1;
    }

    let mut parsed = [0u8; ETH_ALEN];
    index = 0;
    while index < ETH_ALEN {
        parsed[index] = (hex_to_bin(input[index * 3]).unwrap() << 4)
            | hex_to_bin(input[index * 3 + 1]).unwrap();
        index += 1;
    }
    *mac = parsed;
    true
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mac_pton(s: *const u8, mac: *mut u8) -> bool {
    if s.is_null() || mac.is_null() {
        return false;
    }
    let input = unsafe { core::slice::from_raw_parts(s, MAC_ADDR_STR_LEN) };
    let mut parsed = [0u8; ETH_ALEN];
    if !mac_pton_bytes(input, &mut parsed) {
        return false;
    }
    unsafe { core::ptr::copy_nonoverlapping(parsed.as_ptr(), mac, ETH_ALEN) };
    true
}

unsafe fn input_len(src: *const u8, srclen: i32) -> usize {
    if srclen >= 0 {
        return srclen as usize;
    }
    let mut len = 0usize;
    while unsafe { *src.add(len) } != 0 {
        len += 1;
    }
    len
}

unsafe fn write_end(end: *mut *const u8, src: *const u8, index: usize) {
    if !end.is_null() {
        unsafe { end.write(src.add(index)) };
    }
}

fn byte_matches_delim(byte: u8, delim: i32) -> bool {
    byte == 0 || ((0..=u8::MAX as i32).contains(&delim) && byte == delim as u8)
}

fn parse_ipv4_literal(
    input: &[u8],
    delim: i32,
    allow_colon_delim: bool,
) -> Result<([u8; 4], usize), usize> {
    let mut out = [0u8; 4];
    let mut index = 0usize;

    for octet in 0..4 {
        if index >= input.len() || !input[index].is_ascii_digit() {
            return Err(index);
        }
        let mut value = 0u16;
        while index < input.len() && input[index].is_ascii_digit() {
            value = value
                .saturating_mul(10)
                .saturating_add((input[index] - b'0') as u16);
            if value > u8::MAX as u16 {
                return Err(index);
            }
            index += 1;
        }
        out[octet] = value as u8;

        if octet != 3 {
            if index >= input.len() || input[index] != b'.' {
                return Err(index);
            }
            index += 1;
            continue;
        }

        if index == input.len()
            || byte_matches_delim(input[index], delim)
            || (allow_colon_delim && input[index] == b':')
            || (delim == -1 && !input[index].is_ascii_digit() && input[index] != b'.')
        {
            return Ok((out, index));
        }
        return Err(index);
    }

    Err(index)
}

/// `in4_pton()` — `vendor/linux/net/core/utils.c:119`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn in4_pton(
    src: *const u8,
    srclen: i32,
    dst: *mut u8,
    delim: i32,
    end: *mut *const u8,
) -> i32 {
    if src.is_null() || dst.is_null() {
        unsafe { write_end(end, src, 0) };
        return 0;
    }
    let len = unsafe { input_len(src, srclen) };
    let input = unsafe { core::slice::from_raw_parts(src, len) };
    match parse_ipv4_literal(input, delim, true) {
        Ok((parsed, index)) => {
            unsafe {
                core::ptr::copy_nonoverlapping(parsed.as_ptr(), dst, parsed.len());
                write_end(end, src, index);
            }
            1
        }
        Err(index) => {
            unsafe { write_end(end, src, index.min(len)) };
            0
        }
    }
}

fn ipv6_token_len(input: &[u8], delim: i32) -> usize {
    let mut index = 0usize;
    while index < input.len() {
        let byte = input[index];
        if byte_matches_delim(byte, delim) {
            break;
        }
        if delim == -1 && hex_to_bin(byte).is_none() && byte != b':' && byte != b'.' {
            break;
        }
        index += 1;
    }
    index
}

fn parse_hextet(input: &[u8]) -> Option<u16> {
    if input.is_empty() || input.len() > 4 {
        return None;
    }
    let mut value = 0u16;
    for byte in input {
        value = (value << 4) | hex_to_bin(*byte)? as u16;
    }
    Some(value)
}

fn parse_ipv6_literal(input: &[u8], out: &mut [u8; 16]) -> bool {
    if input.is_empty() {
        return false;
    }

    let mut groups = [0u16; 8];
    let mut group_count = 0usize;
    let mut compress_at = None;
    let mut index = 0usize;

    if input.starts_with(b"::") {
        compress_at = Some(0);
        index = 2;
        if index == input.len() {
            out.fill(0);
            return true;
        }
    } else if input[0] == b':' {
        return false;
    }

    while index < input.len() {
        if group_count >= groups.len() {
            return false;
        }

        let start = index;
        while index < input.len() && input[index] != b':' {
            index += 1;
        }
        let component = &input[start..index];
        if component.contains(&b'.') {
            if group_count > 6 || index != input.len() {
                return false;
            }
            let Ok((ipv4, end_index)) = parse_ipv4_literal(component, -2, false) else {
                return false;
            };
            if end_index != component.len() {
                return false;
            }
            groups[group_count] = u16::from_be_bytes([ipv4[0], ipv4[1]]);
            group_count += 1;
            groups[group_count] = u16::from_be_bytes([ipv4[2], ipv4[3]]);
            group_count += 1;
            break;
        }

        let Some(group) = parse_hextet(component) else {
            return false;
        };
        groups[group_count] = group;
        group_count += 1;

        if index < input.len() {
            if index + 1 < input.len() && input[index + 1] == b':' {
                if compress_at.is_some() {
                    return false;
                }
                compress_at = Some(group_count);
                index += 2;
                if index == input.len() {
                    break;
                }
            } else {
                index += 1;
                if index == input.len() {
                    return false;
                }
            }
        }
    }

    let mut expanded = [0u16; 8];
    if let Some(slot) = compress_at {
        if group_count >= 8 {
            return false;
        }
        let zeros = 8 - group_count;
        expanded[..slot].copy_from_slice(&groups[..slot]);
        expanded[slot + zeros..].copy_from_slice(&groups[slot..group_count]);
    } else {
        if group_count != 8 {
            return false;
        }
        expanded = groups;
    }

    for (index, group) in expanded.iter().enumerate() {
        let bytes = group.to_be_bytes();
        out[index * 2] = bytes[0];
        out[index * 2 + 1] = bytes[1];
    }
    true
}

/// `in6_pton()` — `vendor/linux/net/core/utils.c:185`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn in6_pton(
    src: *const u8,
    srclen: i32,
    dst: *mut u8,
    delim: i32,
    end: *mut *const u8,
) -> i32 {
    if src.is_null() || dst.is_null() {
        unsafe { write_end(end, src, 0) };
        return 0;
    }
    let len = unsafe { input_len(src, srclen) };
    let input = unsafe { core::slice::from_raw_parts(src, len) };
    let token_len = ipv6_token_len(input, delim);
    let mut parsed = [0u8; 16];
    if !parse_ipv6_literal(&input[..token_len], &mut parsed) {
        unsafe { write_end(end, src, token_len) };
        return 0;
    }
    unsafe {
        core::ptr::copy_nonoverlapping(parsed.as_ptr(), dst, parsed.len());
        write_end(end, src, token_len);
    }
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mac_pton_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/net_utils.c"
        ));
        assert!(source.contains("#include <linux/if_ether.h>"));
        assert!(source.contains("strnlen(s, MAC_ADDR_STR_LEN) < MAC_ADDR_STR_LEN"));
        assert!(source.contains("Don't dirty result unless string is valid MAC."));
        assert!(source.contains("EXPORT_SYMBOL(mac_pton);"));
        let mut mac = [0u8; ETH_ALEN];
        assert!(mac_pton_bytes(b"01:23:45:ab:CD:ef", &mut mac));
        assert_eq!(mac, [0x01, 0x23, 0x45, 0xab, 0xcd, 0xef]);
        let mut unchanged = [0xaau8; ETH_ALEN];
        assert!(!mac_pton_bytes(b"01:23:45:xx:00:00", &mut unchanged));
        assert_eq!(unchanged, [0xaau8; ETH_ALEN]);
        assert!(!mac_pton_bytes(b"01:23", &mut unchanged));
    }

    #[test]
    fn mac_pton_exports_register_for_modules() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("mac_pton"),
            Some(mac_pton as usize)
        );
    }

    #[test]
    fn inet_pton_exports_track_linux_utils() {
        let source = include_str!("../../vendor/linux/net/core/utils.c");
        assert!(source.contains("EXPORT_SYMBOL(in4_pton);"));
        assert!(source.contains("EXPORT_SYMBOL(in6_pton);"));

        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("in4_pton"),
            Some(in4_pton as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("in6_pton"),
            Some(in6_pton as usize)
        );
    }

    #[test]
    fn inet_pton_parses_common_literals() {
        let mut v4 = [0u8; 4];
        let mut end = core::ptr::null();
        let input = b"192.0.2.1,eth0";
        assert_eq!(
            unsafe {
                in4_pton(
                    input.as_ptr(),
                    input.len() as i32,
                    v4.as_mut_ptr(),
                    b',' as i32,
                    &mut end,
                )
            },
            1
        );
        assert_eq!(v4, [192, 0, 2, 1]);
        assert_eq!(unsafe { end.offset_from(input.as_ptr()) }, 9);

        let mut v6 = [0u8; 16];
        let input = b"2001:db8::1,eth0";
        assert_eq!(
            unsafe {
                in6_pton(
                    input.as_ptr(),
                    input.len() as i32,
                    v6.as_mut_ptr(),
                    b',' as i32,
                    &mut end,
                )
            },
            1
        );
        assert_eq!(
            v6,
            [0x20, 0x01, 0x0d, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]
        );
        assert_eq!(unsafe { end.offset_from(input.as_ptr()) }, 11);

        let mut unchanged = [0xaau8; 4];
        assert_eq!(
            unsafe {
                in4_pton(
                    b"999.0.2.1".as_ptr(),
                    9,
                    unchanged.as_mut_ptr(),
                    -1,
                    core::ptr::null_mut(),
                )
            },
            0
        );
        assert_eq!(unchanged, [0xaau8; 4]);
    }
}

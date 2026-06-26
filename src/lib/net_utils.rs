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
}

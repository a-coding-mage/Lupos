//! linux-parity: complete
//! linux-source: vendor/linux/lib/checksum.c
//! test-origin: linux:vendor/linux/lib/checksum.c
//! Generic IP/TCP/UDP checksum helpers.

use core::ffi::c_void;

use crate::kernel::module::{export_symbol, find_symbol};

pub type Sum16 = u16;
pub type Wsum = u32;

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("ip_fast_csum", ip_fast_csum as usize, false);
    export_symbol_once("csum_partial", csum_partial as usize, false);
    export_symbol_once("ip_compute_csum", ip_compute_csum as usize, false);
    export_symbol_once("csum_tcpudp_nofold", csum_tcpudp_nofold as usize, false);
}

pub fn csum_from32to16(mut sum: u32) -> u16 {
    sum = sum.wrapping_add((sum >> 16) | (sum << 16));
    (sum >> 16) as u16
}

pub fn csum_fold(csum: Wsum) -> Sum16 {
    ((!csum).wrapping_sub(csum.rotate_right(16)) >> 16) as u16
}

unsafe fn read_u16(ptr: *const u8) -> u16 {
    u16::from_ne_bytes(unsafe { core::ptr::read_unaligned(ptr.cast::<[u8; 2]>()) })
}

unsafe fn read_u32(ptr: *const u8) -> u32 {
    u32::from_ne_bytes(unsafe { core::ptr::read_unaligned(ptr.cast::<[u8; 4]>()) })
}

unsafe fn do_csum_ptr(mut buff: *const u8, mut len: i32) -> u32 {
    let mut result = 0u32;

    if len <= 0 {
        return result;
    }

    let odd = (buff as usize) & 1 != 0;
    if odd {
        let byte = unsafe { *buff } as u32;
        #[cfg(target_endian = "little")]
        {
            result = result.wrapping_add(byte << 8);
        }
        #[cfg(target_endian = "big")]
        {
            result = result.wrapping_add(byte);
        }
        len -= 1;
        buff = unsafe { buff.add(1) };
    }

    if len >= 2 {
        if (buff as usize) & 2 != 0 {
            result = result.wrapping_add(unsafe { read_u16(buff) } as u32);
            len -= 2;
            buff = unsafe { buff.add(2) };
        }
        if len >= 4 {
            let end = unsafe { buff.add((len as usize) & !3) };
            let mut carry = 0u32;
            while buff < end {
                let word = unsafe { read_u32(buff) };
                buff = unsafe { buff.add(4) };
                result = result.wrapping_add(carry);
                result = result.wrapping_add(word);
                carry = u32::from(word > result);
            }
            result = result.wrapping_add(carry);
            result = csum_from32to16(result) as u32;
        }
        if len & 2 != 0 {
            result = result.wrapping_add(unsafe { read_u16(buff) } as u32);
            buff = unsafe { buff.add(2) };
        }
    }

    if len & 1 != 0 {
        let byte = unsafe { *buff } as u32;
        #[cfg(target_endian = "little")]
        {
            result = result.wrapping_add(byte);
        }
        #[cfg(target_endian = "big")]
        {
            result = result.wrapping_add(byte << 8);
        }
    }

    result = csum_from32to16(result) as u32;
    if odd {
        result = ((result >> 8) & 0xff) | ((result & 0xff) << 8);
    }
    result
}

pub fn do_csum(buff: &[u8]) -> u32 {
    unsafe { do_csum_ptr(buff.as_ptr(), buff.len() as i32) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ip_fast_csum(iph: *const c_void, ihl: u32) -> Sum16 {
    if iph.is_null() {
        return 0;
    }
    let len = ihl.saturating_mul(4);
    !(unsafe { do_csum_ptr(iph.cast::<u8>(), len as i32) } as u16)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn csum_partial(buff: *const c_void, len: i32, wsum: Wsum) -> Wsum {
    if buff.is_null() && len > 0 {
        return wsum;
    }
    let sum = wsum;
    let mut result = unsafe { do_csum_ptr(buff.cast::<u8>(), len) };
    result = result.wrapping_add(sum);
    if sum > result {
        result = result.wrapping_add(1);
    }
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ip_compute_csum(buff: *const c_void, len: i32) -> Sum16 {
    if buff.is_null() && len > 0 {
        return 0;
    }
    !(unsafe { do_csum_ptr(buff.cast::<u8>(), len) } as u16)
}

fn from64to32(mut x: u64) -> u32 {
    x = (x & 0xffff_ffff) + (x >> 32);
    x = (x & 0xffff_ffff) + (x >> 32);
    x as u32
}

#[unsafe(no_mangle)]
pub extern "C" fn csum_tcpudp_nofold(
    saddr: u32,
    daddr: u32,
    len: u32,
    proto: u8,
    sum: Wsum,
) -> Wsum {
    let mut s = sum as u64;
    s += saddr as u64;
    s += daddr as u64;
    #[cfg(target_endian = "little")]
    {
        s += ((proto as u32).wrapping_add(len) << 8) as u64;
    }
    #[cfg(target_endian = "big")]
    {
        s += (proto as u32).wrapping_add(len) as u64;
    }
    from64to32(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;

    fn checksum_source() -> &'static str {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/checksum.c"
        ))
    }

    fn checksum_kunit_source() -> &'static str {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/tests/checksum_kunit.c"
        ))
    }

    fn extract_array(source: &str, name: &str) -> Vec<u32> {
        let marker = alloc::format!("{name}[] = {{");
        let start = source.find(&marker).expect("array marker") + marker.len();
        let end = source[start..].find("};").expect("array end") + start;
        source[start..end]
            .split(|c: char| !(c.is_ascii_hexdigit() || c == 'x' || c == 'X'))
            .filter(|token| !token.is_empty())
            .map(|token| {
                if let Some(hex) = token
                    .strip_prefix("0x")
                    .or_else(|| token.strip_prefix("0X"))
                {
                    u32::from_str_radix(hex, 16).expect("hex array value")
                } else {
                    token.parse::<u32>().expect("decimal array value")
                }
            })
            .collect()
    }

    fn to_sum16(x: u32) -> u16 {
        u16::from_le(x as u16)
    }

    fn to_wsum(x: u32) -> u32 {
        let hi = u16::from_le((x >> 16) as u16) as u32;
        let lo = u16::from_le(x as u16) as u32;
        (hi << 16) | lo
    }

    fn full_csum(buff: &[u8], sum: u32) -> u16 {
        let partial = unsafe { csum_partial(buff.as_ptr().cast(), buff.len() as i32, sum) };
        csum_fold(partial)
    }

    #[test]
    fn checksum_matches_linux_source_shape() {
        let source = checksum_source();
        assert!(source.contains("static unsigned int do_csum"));
        assert!(source.contains("result = csum_from32to16(result);"));
        assert!(source.contains("__sum16 ip_fast_csum(const void *iph, unsigned int ihl)"));
        assert!(source.contains("__wsum csum_partial(const void *buff, int len, __wsum wsum)"));
        assert!(source.contains("__sum16 ip_compute_csum(const void *buff, int len)"));
        assert!(source.contains("__wsum csum_tcpudp_nofold(__be32 saddr, __be32 daddr,"));
        assert!(source.contains("EXPORT_SYMBOL(ip_fast_csum);"));
        assert!(source.contains("EXPORT_SYMBOL(csum_partial);"));
        assert!(source.contains("EXPORT_SYMBOL(ip_compute_csum);"));
        assert!(source.contains("EXPORT_SYMBOL(csum_tcpudp_nofold);"));
    }

    #[test]
    fn csum_partial_uses_linux_kunit_fixed_random_vectors() {
        let kunit = checksum_kunit_source();
        assert!(kunit.contains("Test cases csum_partial, csum_fold, ip_fast_csum"));
        assert!(kunit.contains("test_csum_fixed_random_inputs"));

        let random_buf: Vec<u8> = extract_array(kunit, "random_buf")
            .into_iter()
            .map(|x| x as u8)
            .collect();
        let expected_results = extract_array(kunit, "expected_results");
        assert_eq!(random_buf.len(), 512);
        assert_eq!(expected_results.len(), 512);

        let mut tmp_buf = [0u8; 512 + 64];
        for align in 0..64 {
            tmp_buf[align..align + random_buf.len()].copy_from_slice(&random_buf);
            for len in 0..random_buf.len() {
                let sum = to_wsum(0x0284_7aab);
                let result = full_csum(&tmp_buf[align..align + len], sum);
                assert_eq!(result, to_sum16(expected_results[len]));
            }
        }
    }

    #[test]
    fn csum_partial_uses_linux_kunit_carry_vectors() {
        let kunit = checksum_kunit_source();
        assert!(kunit.contains("test_csum_all_carry_inputs"));
        assert!(kunit.contains("test_csum_no_carry_inputs"));

        let init_sums_no_overflow = extract_array(kunit, "init_sums_no_overflow");
        assert_eq!(init_sums_no_overflow.len(), 512);

        let mut tmp_buf = [0xffu8; 512 + 64];
        for align in 0..64 {
            for len in 0..512 {
                let result = full_csum(&tmp_buf[align..align + len], to_wsum(0xffff_ffff));
                let expected = if len & 1 != 0 { to_sum16(0xff00) } else { 0 };
                assert_eq!(result, expected);

                let result = full_csum(&tmp_buf[align..align + len], 0);
                let expected = if len & 1 != 0 {
                    to_sum16(0xff00)
                } else if len != 0 {
                    0
                } else {
                    to_sum16(0xffff)
                };
                assert_eq!(result, expected);
            }
        }

        tmp_buf.fill(0x04);
        for align in 0..64 {
            for len in 0..512 {
                let result = full_csum(
                    &tmp_buf[align..align + len],
                    to_wsum(init_sums_no_overflow[len]),
                );
                assert_eq!(result, 0);

                let result = full_csum(
                    &tmp_buf[align..align + len],
                    to_wsum(init_sums_no_overflow[len].wrapping_add(1)),
                );
                let expected = to_sum16(if len != 0 { 0xfffe } else { 0xffff });
                assert_eq!(result, expected);
            }
        }
    }

    #[test]
    fn ip_fast_csum_uses_linux_kunit_vectors() {
        let kunit = checksum_kunit_source();
        assert!(kunit.contains("test_ip_fast_csum"));

        let random_buf: Vec<u8> = extract_array(kunit, "random_buf")
            .into_iter()
            .map(|x| x as u8)
            .collect();
        let expected_fast_csum = extract_array(kunit, "expected_fast_csum");

        for len in 5usize..15 {
            for index in 0..181usize {
                let result =
                    unsafe { ip_fast_csum(random_buf[index..].as_ptr().cast(), len as u32) };
                let expected = expected_fast_csum[(len - 5) * 181 + index];
                assert_eq!(result, to_sum16(expected));
            }
        }
    }

    #[test]
    fn csum_tcpudp_nofold_matches_linux_addition_contract() {
        let source = checksum_source();
        assert!(source.contains("s += (__force u32)saddr;"));
        assert!(source.contains("s += (__force u32)daddr;"));
        assert!(source.contains("s += (proto + len) << 8;"));

        let sum = csum_tcpudp_nofold(0x0102_0304, 0x1121_3141, 1460, 17, 0x5566_7788);
        let expected = from64to32(0x5566_7788u64 + 0x0102_0304 + 0x1121_3141 + ((1460 + 17) << 8));
        assert_eq!(sum, expected);
    }

    #[test]
    fn checksum_exports_register_for_modules() {
        register_module_exports();
        for (name, addr) in [
            ("ip_fast_csum", ip_fast_csum as usize),
            ("csum_partial", csum_partial as usize),
            ("ip_compute_csum", ip_compute_csum as usize),
            ("csum_tcpudp_nofold", csum_tcpudp_nofold as usize),
        ] {
            assert_eq!(crate::kernel::module::find_symbol(name), Some(addr));
        }
    }
}

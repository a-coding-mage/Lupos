//! linux-parity: complete
//! linux-source: vendor/linux/lib/uuid.c
//! test-origin: linux:vendor/linux/lib/uuid.c
//! UUID/GUID parsing and random generation helpers.

use core::ffi::c_char;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::include::uapi::errno::EINVAL;
use crate::kernel::module::{export_symbol, find_symbol};

pub const UUID_STRING_LEN: usize = 36;
const SI: [usize; 16] = [0, 2, 4, 6, 9, 11, 14, 16, 19, 21, 24, 26, 28, 30, 32, 34];

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Guid {
    pub b: [u8; 16],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Uuid {
    pub b: [u8; 16],
}

pub type GuidT = Guid;
pub type UuidT = Uuid;

pub static guid_null: GuidT = GuidT { b: [0; 16] };
pub static uuid_null: UuidT = UuidT { b: [0; 16] };

pub static guid_index: [u8; 16] = [3, 2, 1, 0, 5, 4, 7, 6, 8, 9, 10, 11, 12, 13, 14, 15];
pub static uuid_index: [u8; 16] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];

static RANDOM_STATE: AtomicU64 = AtomicU64::new(0x243f_6a88_85a3_08d3);

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("guid_null", &raw const guid_null as usize, false);
    export_symbol_once("uuid_null", &raw const uuid_null as usize, false);
    export_symbol_once("generate_random_uuid", generate_random_uuid as usize, false);
    export_symbol_once("generate_random_guid", generate_random_guid as usize, false);
    export_symbol_once("guid_gen", guid_gen as usize, true);
    export_symbol_once("uuid_gen", uuid_gen as usize, true);
    export_symbol_once("uuid_is_valid", uuid_is_valid as usize, false);
    export_symbol_once("guid_parse", guid_parse as usize, false);
    export_symbol_once("uuid_parse", uuid_parse as usize, false);
}

fn next_random_u64() -> u64 {
    let mut cur = RANDOM_STATE.load(Ordering::Acquire);
    loop {
        let mut next = cur;
        next ^= next << 13;
        next ^= next >> 7;
        next ^= next << 17;
        match RANDOM_STATE.compare_exchange(cur, next, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_) => return next,
            Err(actual) => cur = actual,
        }
    }
}

fn get_random_bytes(out: &mut [u8]) {
    let mut offset = 0usize;
    while offset < out.len() {
        let bytes = next_random_u64().to_ne_bytes();
        let len = (out.len() - offset).min(bytes.len());
        out[offset..offset + len].copy_from_slice(&bytes[..len]);
        offset += len;
    }
}

pub fn generate_random_uuid_bytes(uuid: &mut [u8; 16]) {
    get_random_bytes(uuid);
    uuid[6] = (uuid[6] & 0x0f) | 0x40;
    uuid[8] = (uuid[8] & 0x3f) | 0x80;
}

pub fn generate_random_guid_bytes(guid: &mut [u8; 16]) {
    get_random_bytes(guid);
    guid[7] = (guid[7] & 0x0f) | 0x40;
    guid[8] = (guid[8] & 0x3f) | 0x80;
}

pub unsafe extern "C" fn generate_random_uuid(uuid: *mut u8) {
    if uuid.is_null() {
        return;
    }
    let uuid = unsafe { core::slice::from_raw_parts_mut(uuid, 16) };
    let mut bytes = [0u8; 16];
    generate_random_uuid_bytes(&mut bytes);
    uuid.copy_from_slice(&bytes);
}

pub unsafe extern "C" fn generate_random_guid(guid: *mut u8) {
    if guid.is_null() {
        return;
    }
    let guid = unsafe { core::slice::from_raw_parts_mut(guid, 16) };
    let mut bytes = [0u8; 16];
    generate_random_guid_bytes(&mut bytes);
    guid.copy_from_slice(&bytes);
}

fn __uuid_gen_common(b: &mut [u8; 16]) {
    get_random_bytes(b);
    b[8] = (b[8] & 0x3f) | 0x80;
}

pub unsafe extern "C" fn guid_gen(lu: *mut GuidT) {
    if lu.is_null() {
        return;
    }
    let lu = unsafe { &mut *lu };
    __uuid_gen_common(&mut lu.b);
    lu.b[7] = (lu.b[7] & 0x0f) | 0x40;
}

pub unsafe extern "C" fn uuid_gen(bu: *mut UuidT) {
    if bu.is_null() {
        return;
    }
    let bu = unsafe { &mut *bu };
    __uuid_gen_common(&mut bu.b);
    bu.b[6] = (bu.b[6] & 0x0f) | 0x40;
}

#[inline]
fn is_xdigit(c: u8) -> bool {
    c.is_ascii_hexdigit()
}

#[inline]
fn hex_to_bin(c: u8) -> i32 {
    match c {
        b'0'..=b'9' => (c - b'0') as i32,
        b'a'..=b'f' => (c - b'a' + 10) as i32,
        b'A'..=b'F' => (c - b'A' + 10) as i32,
        _ => -1,
    }
}

pub fn uuid_is_valid_bytes(uuid: &[u8]) -> bool {
    if uuid.len() < UUID_STRING_LEN {
        return false;
    }

    for (i, &c) in uuid.iter().take(UUID_STRING_LEN).enumerate() {
        if i == 8 || i == 13 || i == 18 || i == 23 {
            if c != b'-' {
                return false;
            }
        } else if !is_xdigit(c) {
            return false;
        }
    }

    true
}

pub unsafe extern "C" fn uuid_is_valid(uuid: *const c_char) -> bool {
    if uuid.is_null() {
        return false;
    }
    let uuid = unsafe { core::slice::from_raw_parts(uuid as *const u8, UUID_STRING_LEN) };
    uuid_is_valid_bytes(uuid)
}

fn __uuid_parse_bytes(uuid: &[u8], b: &mut [u8; 16], ei: &[u8; 16]) -> i32 {
    if !uuid_is_valid_bytes(uuid) {
        return -EINVAL;
    }

    for i in 0..16 {
        let hi = hex_to_bin(uuid[SI[i]]);
        let lo = hex_to_bin(uuid[SI[i] + 1]);
        b[ei[i] as usize] = ((hi << 4) | lo) as u8;
    }

    0
}

pub fn guid_parse_bytes(uuid: &[u8]) -> Result<GuidT, i32> {
    let mut out = GuidT { b: [0; 16] };
    match __uuid_parse_bytes(uuid, &mut out.b, &guid_index) {
        0 => Ok(out),
        err => Err(err),
    }
}

pub fn uuid_parse_bytes(uuid: &[u8]) -> Result<UuidT, i32> {
    let mut out = UuidT { b: [0; 16] };
    match __uuid_parse_bytes(uuid, &mut out.b, &uuid_index) {
        0 => Ok(out),
        err => Err(err),
    }
}

unsafe fn __uuid_parse(uuid: *const c_char, b: &mut [u8; 16], ei: &[u8; 16]) -> i32 {
    if uuid.is_null() {
        return -EINVAL;
    }
    let uuid = unsafe { core::slice::from_raw_parts(uuid as *const u8, UUID_STRING_LEN) };
    __uuid_parse_bytes(uuid, b, ei)
}

pub unsafe extern "C" fn guid_parse(uuid: *const c_char, u: *mut GuidT) -> i32 {
    if u.is_null() {
        return -EINVAL;
    }
    unsafe { __uuid_parse(uuid, &mut (*u).b, &guid_index) }
}

pub unsafe extern "C" fn uuid_parse(uuid: *const c_char, u: *mut UuidT) -> i32 {
    if u.is_null() {
        return -EINVAL;
    }
    unsafe { __uuid_parse(uuid, &mut (*u).b, &uuid_index) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uuid_kunit_vectors_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/uuid.c"
        ));
        assert!(source.contains("const u8 guid_index[16] = {3,2,1,0,5,4,7,6"));
        assert!(source.contains("uuid[6] = (uuid[6] & 0x0F) | 0x40;"));
        assert!(source.contains("guid[7] = (guid[7] & 0x0F) | 0x40;"));
        assert!(source.contains("static const u8 si[16] = {0,2,4,6,9,11,14,16"));
        assert!(source.contains("EXPORT_SYMBOL(uuid_parse);"));

        for data in crate::lib::tests::uuid_kunit::UUID_TEST_DATA {
            assert_eq!(
                guid_parse_bytes(data.uuid.as_bytes()).map(|uuid| uuid.b),
                Ok(data.guid_le)
            );
            assert_eq!(
                uuid_parse_bytes(data.uuid.as_bytes()).map(|uuid| uuid.b),
                Ok(data.uuid_be)
            );
        }
        for uuid in crate::lib::tests::uuid_kunit::UUID_WRONG_DATA {
            assert_eq!(guid_parse_bytes(uuid.as_bytes()), Err(-EINVAL));
            assert_eq!(uuid_parse_bytes(uuid.as_bytes()), Err(-EINVAL));
        }
    }

    #[test]
    fn uuid_raw_exports_and_c_string_prefix_semantics_match_linux() {
        let input = b"c33f4995-3701-450e-9fbf-206a2e98e576extra\0";
        assert!(unsafe { uuid_is_valid(input.as_ptr() as *const c_char) });

        let mut guid = GuidT { b: [0; 16] };
        let mut uuid = UuidT { b: [0; 16] };
        assert_eq!(
            unsafe { guid_parse(input.as_ptr() as *const c_char, &mut guid) },
            0
        );
        assert_eq!(
            unsafe { uuid_parse(input.as_ptr() as *const c_char, &mut uuid) },
            0
        );
        assert_eq!(
            guid.b,
            crate::lib::tests::uuid_kunit::UUID_TEST_DATA[0].guid_le
        );
        assert_eq!(
            uuid.b,
            crate::lib::tests::uuid_kunit::UUID_TEST_DATA[0].uuid_be
        );

        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("uuid_parse"),
            Some(uuid_parse as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("guid_gen"),
            Some(guid_gen as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("guid_null"),
            Some(&raw const guid_null as usize)
        );
    }

    #[test]
    fn random_generation_sets_linux_version_and_variant_bits() {
        let mut uuid = [0u8; 16];
        let mut guid = [0u8; 16];
        unsafe {
            generate_random_uuid(uuid.as_mut_ptr());
            generate_random_guid(guid.as_mut_ptr());
        }
        assert_eq!(uuid[6] & 0xf0, 0x40);
        assert_eq!(uuid[8] & 0xc0, 0x80);
        assert_eq!(guid[7] & 0xf0, 0x40);
        assert_eq!(guid[8] & 0xc0, 0x80);

        let mut u = UuidT { b: [0; 16] };
        let mut g = GuidT { b: [0; 16] };
        unsafe {
            uuid_gen(&mut u);
            guid_gen(&mut g);
        }
        assert_eq!(u.b[6] & 0xf0, 0x40);
        assert_eq!(u.b[8] & 0xc0, 0x80);
        assert_eq!(g.b[7] & 0xf0, 0x40);
        assert_eq!(g.b[8] & 0xc0, 0x80);
    }
}

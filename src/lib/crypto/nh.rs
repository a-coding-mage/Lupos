//! linux-parity: complete
//! linux-source: vendor/linux/lib/crypto/nh.c
//! test-origin: linux:vendor/linux/lib/crypto/nh.c
//! NH almost-universal hash helper used by Adiantum.

use crate::kernel::module::{export_symbol, find_symbol};

pub const NH_PAIR_STRIDE: usize = 2;
pub const NH_NUM_PASSES: usize = 4;
pub const NH_MESSAGE_UNIT: usize = NH_PAIR_STRIDE * 2 * core::mem::size_of::<u32>();
pub const NH_HASH_BYTES: usize = NH_NUM_PASSES * core::mem::size_of::<u64>();

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("nh", nh as usize, true);
}

fn le32_at(message: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        message[offset],
        message[offset + 1],
        message[offset + 2],
        message[offset + 3],
    ])
}

pub fn nh_hash(key: &[u32], message: &[u8]) -> [u64; NH_NUM_PASSES] {
    assert_eq!(message.len() % NH_MESSAGE_UNIT, 0);
    if message.is_empty() {
        return [0; NH_NUM_PASSES];
    }
    let blocks = message.len() / NH_MESSAGE_UNIT;
    assert!(key.len() >= blocks * 4 + 12);

    let mut sums = [0u64; NH_NUM_PASSES];
    let mut key_offset = 0usize;
    let mut msg_offset = 0usize;
    while msg_offset < message.len() {
        let m0 = le32_at(message, msg_offset);
        let m1 = le32_at(message, msg_offset + 4);
        let m2 = le32_at(message, msg_offset + 8);
        let m3 = le32_at(message, msg_offset + 12);

        sums[0] = sums[0].wrapping_add(
            u64::from(m0.wrapping_add(key[key_offset]))
                .wrapping_mul(u64::from(m2.wrapping_add(key[key_offset + 2]))),
        );
        sums[1] = sums[1].wrapping_add(
            u64::from(m0.wrapping_add(key[key_offset + 4]))
                .wrapping_mul(u64::from(m2.wrapping_add(key[key_offset + 6]))),
        );
        sums[2] = sums[2].wrapping_add(
            u64::from(m0.wrapping_add(key[key_offset + 8]))
                .wrapping_mul(u64::from(m2.wrapping_add(key[key_offset + 10]))),
        );
        sums[3] = sums[3].wrapping_add(
            u64::from(m0.wrapping_add(key[key_offset + 12]))
                .wrapping_mul(u64::from(m2.wrapping_add(key[key_offset + 14]))),
        );
        sums[0] = sums[0].wrapping_add(
            u64::from(m1.wrapping_add(key[key_offset + 1]))
                .wrapping_mul(u64::from(m3.wrapping_add(key[key_offset + 3]))),
        );
        sums[1] = sums[1].wrapping_add(
            u64::from(m1.wrapping_add(key[key_offset + 5]))
                .wrapping_mul(u64::from(m3.wrapping_add(key[key_offset + 7]))),
        );
        sums[2] = sums[2].wrapping_add(
            u64::from(m1.wrapping_add(key[key_offset + 9]))
                .wrapping_mul(u64::from(m3.wrapping_add(key[key_offset + 11]))),
        );
        sums[3] = sums[3].wrapping_add(
            u64::from(m1.wrapping_add(key[key_offset + 13]))
                .wrapping_mul(u64::from(m3.wrapping_add(key[key_offset + 15]))),
        );

        key_offset += NH_MESSAGE_UNIT / core::mem::size_of::<u32>();
        msg_offset += NH_MESSAGE_UNIT;
    }
    sums
}

pub unsafe extern "C" fn nh(
    key: *const u32,
    message: *const u8,
    message_len: usize,
    hash: *mut u64,
) {
    if key.is_null() || message.is_null() || hash.is_null() || message_len % NH_MESSAGE_UNIT != 0 {
        return;
    }
    let blocks = message_len / NH_MESSAGE_UNIT;
    let key_words = blocks * 4 + 12;
    let key = unsafe { core::slice::from_raw_parts(key, key_words) };
    let message = unsafe { core::slice::from_raw_parts(message, message_len) };
    let sums = nh_hash(key, message);
    unsafe { core::ptr::copy_nonoverlapping(sums.as_ptr(), hash, NH_NUM_PASSES) };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nh_matches_linux_source_and_hash_flow() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/nh.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/crypto/nh.h"
        ));
        assert!(source.contains("static bool nh_arch("));
        assert!(source.contains("u64 sums[4] = { 0, 0, 0, 0 };"));
        assert!(source.contains("static_assert(NH_PAIR_STRIDE == 2);"));
        assert!(source.contains("sums[0] += (u64)(u32)(m0 + key[0]) * (u32)(m2 + key[2]);"));
        assert!(source.contains("key += NH_MESSAGE_UNIT / sizeof(key[0]);"));
        assert!(source.contains("hash[3] = cpu_to_le64(sums[3]);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(nh);"));
        assert!(header.contains("#define NH_NUM_PASSES"));
        assert!(header.contains("Must be a multiple of 16"));

        let key = [0u32; 16];
        let message = [1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
        let m0 = u64::from(u32::from_le_bytes([1, 2, 3, 4]));
        let m1 = u64::from(u32::from_le_bytes([5, 6, 7, 8]));
        let m2 = u64::from(u32::from_le_bytes([9, 10, 11, 12]));
        let m3 = u64::from(u32::from_le_bytes([13, 14, 15, 16]));
        assert_eq!(nh_hash(&key, &message), [m0 * m2 + m1 * m3; 4]);

        register_module_exports();
        assert_eq!(crate::kernel::module::find_symbol("nh"), Some(nh as usize));
    }
}

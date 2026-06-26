//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/lib/csum-partial_64.c
//! test-origin: linux:vendor/linux/arch/x86/lib/csum-partial_64.c
//! x86-64 Internet checksum partial-sum routines.

pub fn csum_finalize_sum(temp64: u64) -> u32 {
    ((temp64.wrapping_add(temp64.rotate_right(32))) >> 32) as u32
}

pub fn update_csum_40b(mut sum: u64, words: [u64; 5]) -> u64 {
    for word in words {
        sum = add_with_carry(sum, word);
    }
    add_with_carry(sum, 0)
}

pub fn csum_partial(buff: &[u8], sum: u32) -> u32 {
    let mut temp64 = sum as u64;
    let mut offset = 0usize;
    let mut len = buff.len();

    if len >= 80 {
        let mut temp64_2 = 0u64;
        while len >= 80 {
            temp64 = update_csum_40b(temp64, read_words_40(&buff[offset..]));
            temp64_2 = update_csum_40b(temp64_2, read_words_40(&buff[offset + 40..]));
            offset += 80;
            len -= 80;
        }
        temp64 = add_with_carry(temp64, temp64_2);
    }

    if len >= 40 {
        temp64 = update_csum_40b(temp64, read_words_40(&buff[offset..]));
        offset += 40;
        len -= 40;
        if len == 0 {
            return csum_finalize_sum(temp64);
        }
    }

    if len & 32 != 0 {
        for _ in 0..4 {
            temp64 = add_with_carry(temp64, read_u64(buff, offset));
            offset += 8;
        }
    }
    if len & 16 != 0 {
        for _ in 0..2 {
            temp64 = add_with_carry(temp64, read_u64(buff, offset));
            offset += 8;
        }
    }
    if len & 8 != 0 {
        temp64 = add_with_carry(temp64, read_u64(buff, offset));
        offset += 8;
    }
    if len & 7 != 0 {
        let trail_len = len & 7;
        let shift = ((!trail_len + 1) << 3) & 63;
        let trail = (load_unaligned_zeropad(&buff[offset..]) << shift) >> shift;
        temp64 = add_with_carry(temp64, trail);
    }

    csum_finalize_sum(temp64)
}

pub fn ip_compute_csum(buff: &[u8]) -> u16 {
    csum_fold(csum_partial(buff, 0))
}

pub fn csum_fold(sum: u32) -> u16 {
    let mut folded = sum;
    folded = (folded & 0xffff).wrapping_add(folded >> 16);
    folded = (folded & 0xffff).wrapping_add(folded >> 16);
    !(folded as u16)
}

fn add_with_carry(a: u64, b: u64) -> u64 {
    let (sum, carry) = a.overflowing_add(b);
    sum.wrapping_add(carry as u64)
}

fn read_words_40(bytes: &[u8]) -> [u64; 5] {
    [
        read_u64(bytes, 0),
        read_u64(bytes, 8),
        read_u64(bytes, 16),
        read_u64(bytes, 24),
        read_u64(bytes, 32),
    ]
}

fn read_u64(bytes: &[u8], offset: usize) -> u64 {
    let mut word = [0u8; 8];
    word.copy_from_slice(&bytes[offset..offset + 8]);
    u64::from_le_bytes(word)
}

fn load_unaligned_zeropad(bytes: &[u8]) -> u64 {
    let mut word = [0u8; 8];
    let len = core::cmp::min(bytes.len(), 8);
    word[..len].copy_from_slice(&bytes[..len]);
    u64::from_le_bytes(word)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csum_partial_64_matches_linux_source_shape() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/lib/csum-partial_64.c"
        ));
        assert!(source.contains("static inline __wsum csum_finalize_sum(u64 temp64)"));
        assert!(source.contains("return (__force __wsum)((temp64 + ror64(temp64, 32)) >> 32);"));
        assert!(source.contains("static inline unsigned long update_csum_40b"));
        assert!(source.contains("adcq $0,%0"));
        assert!(source.contains("__wsum csum_partial(const void *buff, int len, __wsum sum)"));
        assert!(source.contains("if (likely(len >= 80))"));
        assert!(source.contains("temp64 = update_csum_40b(temp64, buff);"));
        assert!(source.contains("temp64_2 = update_csum_40b(temp64_2, buff + 40);"));
        assert!(source.contains("if (len >= 40)"));
        assert!(source.contains("if (len & 32)"));
        assert!(source.contains("if (len & 16)"));
        assert!(source.contains("if (len & 8)"));
        assert!(source.contains("if (len & 7)"));
        assert!(source.contains("load_unaligned_zeropad(buff)"));
        assert!(source.contains("EXPORT_SYMBOL(csum_partial);"));
        assert!(source.contains("__sum16 ip_compute_csum(const void *buff, int len)"));
        assert!(source.contains("return csum_fold(csum_partial(buff, len, 0));"));

        let words = [1, 2, 3, 4, 5];
        assert_eq!(update_csum_40b(0, words), 15);
        assert_eq!(csum_finalize_sum(0x0000_0001_0000_0002), 3);
    }

    #[test]
    fn csum_partial_handles_hot_40_byte_and_tail_cases() {
        let forty = [1u8; 40];
        let expected_word = u64::from_le_bytes([1; 8]);
        let expected_sum = update_csum_40b(0, [expected_word; 5]);
        assert_eq!(csum_partial(&forty, 0), csum_finalize_sum(expected_sum));

        let bytes = [0x45, 0x00, 0x00, 0x54];
        assert_eq!(csum_partial(&bytes, 0), 0x5400_0045);
        assert_eq!(ip_compute_csum(&bytes), !0x5445u16);
    }
}

//! linux-parity: complete
//! linux-source: vendor/linux/fs/ntfs3/upcase.c
//! test-origin: linux:vendor/linux/fs/ntfs3/upcase.c
//! NTFS3 name upcase, comparison, and hash helpers.

pub fn ntfs3_upcase_unicode_char(upcase: &[u16], chr: u16) -> u16 {
    if chr < b'a' as u16 {
        return chr;
    }
    if chr <= b'z' as u16 {
        return chr - (b'a' - b'A') as u16;
    }
    upcase.get(chr as usize).copied().unwrap_or(chr)
}

pub fn ntfs_cmp_names(s1: &[u16], s2: &[u16], upcase: Option<&[u16]>, bothcase: bool) -> i32 {
    let l1 = s1.len();
    let l2 = s2.len();
    let mut len = core::cmp::min(l1, l2);
    let mut idx = 0usize;
    let mut diff1 = 0i32;

    if !(!bothcase && upcase.is_some()) {
        while len != 0 {
            diff1 = s1[idx] as i32 - s2[idx] as i32;
            if diff1 != 0 {
                if bothcase && upcase.is_some() {
                    break;
                }
                return diff1;
            }
            idx += 1;
            len -= 1;
        }
        if len == 0 {
            return l1 as i32 - l2 as i32;
        }
    }

    let table = upcase.expect("ntfs3 case-insensitive comparison requires an upcase table");
    while len != 0 {
        let diff2 = ntfs3_upcase_unicode_char(table, s1[idx]) as i32
            - ntfs3_upcase_unicode_char(table, s2[idx]) as i32;
        if diff2 != 0 {
            return diff2;
        }
        idx += 1;
        len -= 1;
    }

    let diff2 = l1 as i32 - l2 as i32;
    if diff2 != 0 { diff2 } else { diff1 }
}

pub fn ntfs_cmp_names_cpu(s1: &[u16], s2: &[u16], upcase: Option<&[u16]>, bothcase: bool) -> i32 {
    ntfs_cmp_names(s1, s2, upcase, bothcase)
}

pub fn ntfs_names_hash(name: &[u16], upcase: &[u16], mut hash: u64) -> u64 {
    for &chr in name {
        let c = ntfs3_upcase_unicode_char(upcase, chr) as u64;
        hash = partial_name_hash(c, hash);
    }
    hash
}

pub const fn partial_name_hash(c: u64, prevhash: u64) -> u64 {
    prevhash
        .wrapping_add(c << 4)
        .wrapping_add(c >> 4)
        .wrapping_mul(11)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ntfs3_upcase_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/ntfs3/upcase.c"
        ));
        assert!(source.contains("#include <linux/kernel.h>"));
        assert!(source.contains("#include <linux/types.h>"));
        assert!(source.contains("#include \"ntfs_fs.h\""));
        assert!(
            source.contains("static inline u16 upcase_unicode_char(const u16 *upcase, u16 chr)")
        );
        assert!(source.contains("if (chr < 'a')"));
        assert!(source.contains("if (chr <= 'z')"));
        assert!(source.contains("return chr - ('a' - 'A');"));
        assert!(source.contains("return upcase[chr];"));
        assert!(source.contains(
            "int ntfs_cmp_names(const __le16 *s1, size_t l1, const __le16 *s2, size_t l2,"
        ));
        assert!(source.contains("if (!bothcase && upcase)"));
        assert!(source.contains("goto case_insentive;"));
        assert!(source.contains("diff1 = le16_to_cpu(*s1) - le16_to_cpu(*s2);"));
        assert!(source.contains("if (bothcase && upcase)"));
        assert!(source.contains("return l1 - l2;"));
        assert!(source.contains("diff2 = upcase_unicode_char(upcase, le16_to_cpu(*s1)) -"));
        assert!(source.contains("return diff2 ? diff2 : diff1;"));
        assert!(source.contains(
            "int ntfs_cmp_names_cpu(const struct cpu_str *uni1, const struct le_str *uni2,"
        ));
        assert!(source.contains(
            "unsigned long ntfs_names_hash(const u16 *name, size_t len, const u16 *upcase,"
        ));
        assert!(source.contains("hash = partial_name_hash(c, hash);"));

        let mut upcase = [0u16; 256];
        for (idx, slot) in upcase.iter_mut().enumerate() {
            *slot = idx as u16;
        }
        upcase[b'e' as usize] = b'E' as u16;
        upcase[0x00e9] = 0x00c9;

        assert_eq!(ntfs3_upcase_unicode_char(&upcase, b'a' as u16), b'A' as u16);
        assert_eq!(ntfs3_upcase_unicode_char(&upcase, 0x00e9), 0x00c9);
        assert_eq!(
            ntfs_cmp_names(&[b'a' as u16], &[b'A' as u16], Some(&upcase), false),
            0
        );
        assert!(ntfs_cmp_names(&[b'b' as u16], &[b'A' as u16], Some(&upcase), false) > 0);
        assert_eq!(
            ntfs_cmp_names(&[b'a' as u16], &[b'A' as u16], Some(&upcase), true),
            32
        );
        assert_eq!(
            ntfs_cmp_names(&[b'A' as u16, 1], &[b'A' as u16], Some(&upcase), false),
            1
        );
        assert_eq!(
            ntfs_cmp_names_cpu(&[0x00e9], &[0x00c9], Some(&upcase), false),
            0
        );
        assert_eq!(
            ntfs_names_hash(&[b'a' as u16, 0x00e9], &upcase, 0),
            partial_name_hash(0x00c9, partial_name_hash(b'A' as u64, 0))
        );
    }
}

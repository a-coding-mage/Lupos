//! linux-parity: complete
//! linux-source: vendor/linux/fs/hfs/string.c
//! test-origin: linux:vendor/linux/fs/hfs/string.c
//! HFS Macintosh filename ordering.

pub const HFS_NAMELEN: usize = 31;

pub const HFS_CASEORDER: [u8; 256] = [
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
    0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f,
    0x20, 0x22, 0x23, 0x28, 0x29, 0x2a, 0x2b, 0x2c, 0x2f, 0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36,
    0x37, 0x38, 0x39, 0x3a, 0x3b, 0x3c, 0x3d, 0x3e, 0x3f, 0x40, 0x41, 0x42, 0x43, 0x44, 0x45, 0x46,
    0x47, 0x48, 0x57, 0x59, 0x5d, 0x5f, 0x66, 0x68, 0x6a, 0x6c, 0x72, 0x74, 0x76, 0x78, 0x7a, 0x7e,
    0x8c, 0x8e, 0x90, 0x92, 0x95, 0x97, 0x9e, 0xa0, 0xa2, 0xa4, 0xa7, 0xa9, 0xaa, 0xab, 0xac, 0xad,
    0x4e, 0x48, 0x57, 0x59, 0x5d, 0x5f, 0x66, 0x68, 0x6a, 0x6c, 0x72, 0x74, 0x76, 0x78, 0x7a, 0x7e,
    0x8c, 0x8e, 0x90, 0x92, 0x95, 0x97, 0x9e, 0xa0, 0xa2, 0xa4, 0xa7, 0xaf, 0xb0, 0xb1, 0xb2, 0xb3,
    0x4a, 0x4c, 0x5a, 0x60, 0x7b, 0x7f, 0x98, 0x4f, 0x49, 0x51, 0x4a, 0x4b, 0x4c, 0x5a, 0x60, 0x63,
    0x64, 0x65, 0x6e, 0x6f, 0x70, 0x71, 0x7b, 0x84, 0x85, 0x86, 0x7f, 0x80, 0x9a, 0x9b, 0x9c, 0x98,
    0xb4, 0xb5, 0xb6, 0xb7, 0xb8, 0xb9, 0xba, 0x94, 0xbb, 0xbc, 0xbd, 0xbe, 0xbf, 0xc0, 0x4d, 0x81,
    0xc1, 0xc2, 0xc3, 0xc4, 0xc5, 0xc6, 0xc7, 0xc8, 0xc9, 0xca, 0xcb, 0x55, 0x8a, 0xcc, 0x4d, 0x81,
    0xcd, 0xce, 0xcf, 0xd0, 0xd1, 0xd2, 0xd3, 0x26, 0x27, 0xd4, 0x20, 0x49, 0x4b, 0x80, 0x82, 0x82,
    0xd5, 0xd6, 0x24, 0x25, 0x2d, 0x2e, 0xd7, 0xd8, 0xa6, 0xd9, 0xda, 0xdb, 0xdc, 0xdd, 0xde, 0xdf,
    0xe0, 0xe1, 0xe2, 0xe3, 0xe4, 0xe5, 0xe6, 0xe7, 0xe8, 0xe9, 0xea, 0xeb, 0xec, 0xed, 0xee, 0xef,
    0xf0, 0xf1, 0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7, 0xf8, 0xf9, 0xfa, 0xfb, 0xfc, 0xfd, 0xfe, 0xff,
];

pub fn hfs_strcmp(s1: &[u8], s2: &[u8]) -> i32 {
    let len = core::cmp::min(s1.len(), s2.len());
    for i in 0..len {
        let tmp = HFS_CASEORDER[s1[i] as usize] as i32 - HFS_CASEORDER[s2[i] as usize] as i32;
        if tmp != 0 {
            return tmp;
        }
    }
    s1.len() as i32 - s2.len() as i32
}

pub fn hfs_compare_dentry(len: usize, str_name: &[u8], name: &[u8]) -> i32 {
    let len = if len >= HFS_NAMELEN {
        if name.len() < HFS_NAMELEN {
            return 1;
        }
        HFS_NAMELEN
    } else {
        if len != name.len() {
            return 1;
        }
        len
    };

    for i in 0..len {
        if HFS_CASEORDER[str_name[i] as usize] != HFS_CASEORDER[name[i] as usize] {
            return 1;
        }
    }
    0
}

pub fn hfs_hash_folded(name: &[u8], mut hash: u64) -> u64 {
    let len = core::cmp::min(name.len(), HFS_NAMELEN);
    for &c in name.iter().take(len) {
        hash = partial_name_hash(HFS_CASEORDER[c as usize] as u64, hash);
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
    fn hfs_string_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/hfs/string.c"
        ));
        assert!(source.contains("#include \"hfs_fs.h\""));
        assert!(source.contains("#include <linux/dcache.h>"));
        assert!(source.contains("#include <kunit/visibility.h>"));
        assert!(source.contains("static unsigned char caseorder[256]"));
        assert!(
            source.contains("int hfs_hash_dentry(const struct dentry *dentry, struct qstr *this)")
        );
        assert!(source.contains("if (len > HFS_NAMELEN)"));
        assert!(source.contains("hash = init_name_hash(dentry);"));
        assert!(source.contains("hash = partial_name_hash(caseorder[*name++], hash);"));
        assert!(source.contains("this->hash = end_name_hash(hash);"));
        assert!(source.contains("EXPORT_SYMBOL_IF_KUNIT(hfs_hash_dentry);"));
        assert!(source.contains("int hfs_strcmp(const unsigned char *s1, unsigned int len1,"));
        assert!(source.contains("tmp = (int)caseorder[*(s1++)] - (int)caseorder[*(s2++)];"));
        assert!(source.contains("return len1 - len2;"));
        assert!(source.contains("EXPORT_SYMBOL_IF_KUNIT(hfs_strcmp);"));
        assert!(source.contains("int hfs_compare_dentry(const struct dentry *dentry,"));
        assert!(source.contains("if (len >= HFS_NAMELEN)"));
        assert!(source.contains("if (name->len < HFS_NAMELEN)"));
        assert!(source.contains("if (caseorder[*n1++] != caseorder[*n2++])"));
        assert!(source.contains("return 0;"));
        assert!(source.contains("EXPORT_SYMBOL_IF_KUNIT(hfs_compare_dentry);"));

        assert_eq!(HFS_CASEORDER[b'A' as usize], HFS_CASEORDER[b'a' as usize]);
        assert_eq!(hfs_strcmp(b"File", b"file"), 0);
        assert!(hfs_strcmp(b"abc", b"abd") < 0);
        assert_eq!(hfs_strcmp(b"abc", b"abcX"), -1);
        assert_eq!(hfs_compare_dentry(4, b"File", b"file"), 0);
        assert_eq!(hfs_compare_dentry(4, b"File", b"fild"), 1);
        assert_eq!(
            hfs_compare_dentry(HFS_NAMELEN, &[b'A'; HFS_NAMELEN], &[b'a'; HFS_NAMELEN]),
            0
        );
        assert_eq!(
            hfs_compare_dentry(HFS_NAMELEN, &[b'A'; HFS_NAMELEN], b"short"),
            1
        );
        assert_eq!(
            hfs_hash_folded(b"Aa", 0),
            partial_name_hash(
                HFS_CASEORDER[b'a' as usize] as u64,
                partial_name_hash(HFS_CASEORDER[b'A' as usize] as u64, 0)
            )
        );
    }
}
